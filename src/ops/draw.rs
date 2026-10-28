use std::ptr;

#[cfg(any(safegl_desktop, safegl_webgl))]
use smallvec::SmallVec;

use glow::HasContext;

use crate::BufferExt;
use crate::CapabilitiesSource;
use crate::BufferSliceExt;
use crate::DrawError;
use crate::GlObject;
use crate::ProgramExt;
use crate::uniforms::CollectedUniforms;

use crate::context::Context;
use crate::ContextExt;
use crate::TransformFeedbackExt;

use crate::fbo::{self, FramebufferTarget};

use crate::index::{self, IndicesSource};
use crate::program::DrawProgram;
use crate::uniforms::Uniforms;
use crate::vertex::{InstanceRate, MultiVerticesSource, TransformFeedback, VerticesSource};
use crate::vertex_array_object::VertexAttributesSystem;
use crate::ToGlEnum;

use crate::draw_parameters::DrawParameters;

use crate::version::Api;
use crate::version::Version;
use crate::{context, draw_parameters, gl};

/// Draws everything.
pub fn draw<'a, U, V>(
    context: &Context,
    framebuffer: FramebufferTarget<'_>,
    vertex_buffers: V,
    indices: IndicesSource<'_>,
    program: DrawProgram<'_>,
    uniforms: &U,
    draw_parameters: &DrawParameters<'_>,
    dimensions: (u32, u32),
) -> Result<(), DrawError>
where
    U: Uniforms,
    V: MultiVerticesSource<'a>,
{
    // this contains the list of fences that will need to be fulfilled after the draw command
    // has started
    let mut fences = crate::buffer::FenceInserters::new();

    let vertices_per_patch = check_stages(context, &program, indices.get_primitives_type())?;

    // the base instance given in the draw parameters
    let supports_base_instance = is_base_instance_supported(context);
    #[cfg(any(safegl_gl42, safegl_webgl))]
    let parameters_base_instance = draw_parameters.base_instance;
    #[cfg(not(any(safegl_gl42, safegl_webgl)))]
    let parameters_base_instance = 0u32;
    #[cfg(any(safegl_gl42, safegl_webgl))]
    if parameters_base_instance != 0 && !supports_base_instance {
        return Err(DrawError::BaseInstanceNotSupported);
    }
    // the commands of multidraws give their own base instance, and captured vertices are drawn
    // without one
    #[cfg(safegl_gl42)]
    if parameters_base_instance != 0 {
        match indices {
            IndicesSource::MultidrawArray { .. }
            | IndicesSource::MultidrawElement { .. }
            | IndicesSource::TransformFeedback { .. } => {
                return Err(DrawError::BaseInstanceNotApplicable)
            }
            _ => (),
        }
    }

    // WebGL always restarts primitives at the largest index value
    if crate::version::is_webgl()
        && draw_parameters.primitive_restart == draw_parameters::PrimitiveRestart::Disabled
    {
        if let IndicesSource::IndexBuffer { ref buffer, .. } = indices {
            if buffer.may_hold_restart_index() {
                return Err(DrawError::PrimitiveRestartAlwaysOn);
            }
        }
    }

    if !draw_parameters::check_queries(draw_parameters)? {
        return Ok(());
    }

    // the values of the uniforms are collected before the context is borrowed, so that the code
    // that gives them can use SafeGL
    let uniforms = CollectedUniforms::new(uniforms);

    // starting the state changes
    let mut ctxt = context.make_current();

    // OpenGL objects can only be used with the context that created them
    check_contexts(&ctxt, &program, &indices, draw_parameters)?;
    // the commands of a lost context do nothing
    if ctxt.state.lost_context {
        return Ok(());
    }
    #[cfg(safegl_gl46)]
    check_draw_count(&ctxt, &indices)?;
    #[cfg(any(safegl_desktop, safegl_gles31))]
    check_indirect(&ctxt, &indices, draw_parameters)?;

    // handling vertices source
    let (vertices_count, instances_count, base_vertex, base_instance) = {
        let index_buffer = match indices {
            IndicesSource::IndexBuffer { buffer, .. } => Some(buffer),
            #[cfg(any(safegl_desktop, safegl_gles31))]
            IndicesSource::MultidrawArray { .. } => None,
            #[cfg(any(safegl_desktop, safegl_gles31))]
            IndicesSource::MultidrawElement { indices, .. } => Some(indices),
            IndicesSource::NoIndices { .. } => None,
            #[cfg(safegl_desktop)]
            IndicesSource::TransformFeedback { .. } => None,
            #[cfg(any(safegl_desktop, safegl_webgl))]
            IndicesSource::NoIndicesRanges { .. } => None,
            #[cfg(any(safegl_desktop, safegl_webgl))]
            IndicesSource::IndexBufferRanges { buffer, .. } => Some(buffer),
        };

        // determining whether we can use the `base_vertex` variants for drawing
        let use_base_vertex = match indices {
            #[cfg(any(safegl_desktop, safegl_gles31))]
            IndicesSource::MultidrawArray { .. } => false,
            #[cfg(any(safegl_desktop, safegl_gles31))]
            IndicesSource::MultidrawElement { .. } => false,
            IndicesSource::NoIndices { .. } => true,
            // the base vertex is added to the first vertex of each range
            #[cfg(any(safegl_desktop, safegl_webgl))]
            IndicesSource::NoIndicesRanges { .. } => true,
            _ => {
                ctxt.version.0 == Api::Gl
                    || ctxt.version >= &Version(Api::GlEs, 3, 2)
                    || ctxt.extensions.gl_oes_draw_elements_base_vertex
                    || ctxt.extensions.webgl_draw_instanced_base_vertex_base_instance
            }
        };

        // the commands of multidraws have their own base instance; draws of ranges and of
        // captured vertices take the base instance of the parameters only
        let use_base_instance = supports_base_instance
            && match indices {
                #[cfg(any(safegl_desktop, safegl_gles31))]
                IndicesSource::MultidrawArray { .. } | IndicesSource::MultidrawElement { .. } => {
                    false
                }
                #[cfg(any(safegl_desktop, safegl_webgl))]
                IndicesSource::NoIndicesRanges { .. } | IndicesSource::IndexBufferRanges { .. } => {
                    false
                }
                #[cfg(safegl_desktop)]
                IndicesSource::TransformFeedback { .. } => false,
                _ => true,
            };

        // object that is used to build the bindings
        let mut binder = VertexAttributesSystem::start(
            &mut ctxt,
            program.vertex(),
            index_buffer,
            use_base_vertex,
            use_base_instance,
        );
        // number of vertices in the vertices sources, and whether they have different numbers
        let mut vertices_count: Option<usize> = None;
        let mut vertices_mismatch = false;
        // number of instances to draw
        let mut instances_count: Option<usize> = None;

        for src in vertex_buffers.iter() {
            // Allow single match for consistency with the match below.
            // Integrating the two matches wouldn't improve the code either.
            #[allow(clippy::single_match)]
            match src {
                VerticesSource::VertexBuffer(buffer, format, rate) => {
                    if !binder_context_owns(&buffer, context) {
                        return Err(DrawError::ContextMismatch {
                            object: "vertex buffer".to_owned(),
                        });
                    }

                    if let Some(fence) = buffer.add_fence() {
                        fences.push(fence);
                    }

                    let divisor = match rate {
                        InstanceRate::PerVertex => None,
                        InstanceRate::PerInstance(divisor) => Some(divisor.get()),
                    };
                    binder = binder.add(&buffer, format, divisor);
                }
                VerticesSource::Constant(ref constant) => {
                    binder = binder.add_constant(constant);
                }
                _ => {}
            }

            match src {
                VerticesSource::VertexBuffer(ref buffer, _, InstanceRate::PerVertex) => {
                    if let Some(curr) = vertices_count {
                        if curr != buffer.get_elements_count() {
                            vertices_mismatch = true;
                        }
                    } else {
                        vertices_count = Some(buffer.get_elements_count());
                    }
                }
                VerticesSource::VertexBuffer(ref buffer, _, InstanceRate::PerInstance(divisor)) => {
                    // each element provides the attributes of `divisor` instances
                    let instances = buffer
                        .get_elements_count()
                        .saturating_mul(divisor.get() as usize);
                    if let Some(curr) = instances_count {
                        if curr != instances {
                            return Err(DrawError::InstancesCountMismatch);
                        }
                    } else {
                        instances_count = Some(instances);
                    }
                }
                VerticesSource::Marker { len, per_instance } if !per_instance => {
                    if let Some(curr) = vertices_count {
                        if curr != len {
                            vertices_mismatch = true;
                        }
                    } else {
                        vertices_count = Some(len);
                    }
                }
                VerticesSource::Marker { len, per_instance } if per_instance => {
                    if let Some(curr) = instances_count {
                        if curr != len {
                            return Err(DrawError::InstancesCountMismatch);
                        }
                    } else {
                        instances_count = Some(len);
                    }
                }
                _ => (),
            }
        }

        let vertices_count = if vertices_mismatch {
            None
        } else {
            vertices_count
        };
        let (base_vertex, base_instance) = binder.bind()?;
        (
            vertices_count,
            instances_count,
            base_vertex.unwrap_or(0),
            base_instance + parameters_base_instance,
        )
    };

    // binding the FBO to draw upon
    {
        let fbo_id =
            fbo::FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, framebuffer);
        unsafe { fbo::bind_framebuffer(&mut ctxt, fbo_id, true, false) };
    };
    draw_parameters::validate_dual_source_blending(&mut ctxt, draw_parameters, |ctxt| {
        fbo::draw_buffers_count(ctxt, framebuffer)
    })?;

    // transform feedback can't record while the program changes: the capture is paused, unless
    // the draw continues it
    if !draw_parameters
        .transform_feedback
        .is_some_and(|transform_feedback| transform_feedback.is_recording(&ctxt))
    {
        TransformFeedback::pause(&mut ctxt);
    }

    // binding the program and uniforms
    bind_programs(&mut ctxt, &program, &uniforms, &mut fences)?;

    // OpenGL ES and WebGL only blend into 32-bit float targets with `GL_EXT_float_blend`
    if let FramebufferTarget::Attachments(attachments) = framebuffer {
        if ctxt.version.0 == Api::GlEs
            && !ctxt.extensions.gl_ext_float_blend
            && attachments.has_float32_colors()
            && draw_parameters::blends(draw_parameters)
        {
            return Err(DrawError::FloatBlendNotSupported);
        }
    }

    // sampling a level that the draw renders to is a feedback loop: undefined on OpenGL, and an
    // error on WebGL; texture barriers make it defined when each texel is read before it is
    // written (`texture_barrier`)
    let texture_barriers = ctxt.version >= &Version(Api::Gl, 4, 5)
        || ctxt.extensions.gl_arb_texture_barrier
        || ctxt.extensions.gl_nv_texture_barrier;
    if let (FramebufferTarget::Attachments(attachments), false) = (framebuffer, texture_barriers) {
        if attachments.attaches_any(&ctxt.sampled_textures) {
            return Err(DrawError::FeedbackLoop);
        }
    }

    // sync-ing draw_parameters
    unsafe {
        let index_type = match indices {
            IndicesSource::IndexBuffer { data_type, .. } => Some(data_type),
            #[cfg(any(safegl_desktop, safegl_gles31))]
            IndicesSource::MultidrawElement { data_type, .. } => Some(data_type),
            #[cfg(safegl_desktop)]
            IndicesSource::IndexBufferRanges { data_type, .. } => Some(data_type),
            _ => None,
        };
        draw_parameters::sync(
            &mut ctxt,
            draw_parameters,
            dimensions,
            indices.get_primitives_type(),
            index_type,
        )?;
        sync_vertices_per_patch(&mut ctxt, vertices_per_patch);

        if let Some(ref tf) = draw_parameters.transform_feedback {
            tf.bind(&mut ctxt, indices.get_primitives_type())?;
        }
    }

    // drawing
    // TODO: make this code more readable
    {
        match &indices {
            IndicesSource::IndexBuffer {
                ref buffer,
                data_type,
                primitives,
            } => {
                let ptr: *const u8 = ptr::null_mut();
                let ptr = unsafe { ptr.add(buffer.get_offset_bytes()) };
                let offset = buffer.get_offset_bytes() as i32;

                if let Some(fence) = buffer.add_fence() {
                    fences.push(fence);
                }

                // WebGL draws with a base vertex or a base instance with
                // `WEBGL_draw_instanced_base_vertex_base_instance`, as one instance or more
                let webgl = cfg!(safegl_browser) && (base_instance != 0 || base_vertex != 0);

                unsafe {
                    if webgl {
                        #[cfg(safegl_browser)]
                        ctxt.webgl.draw_elements_instanced_base_vertex_base_instance(
                            primitives.to_glenum(),
                            buffer.get_elements_count() as gl::types::GLsizei,
                            data_type.to_glenum(),
                            offset,
                            instances_count.unwrap_or(1) as gl::types::GLsizei,
                            base_vertex,
                            base_instance,
                        );
                    } else if base_instance != 0 {
                        // a draw without per-instance buffers draws one instance
                        let instances_count = instances_count.unwrap_or(1) as gl::types::GLsizei;
                        let count = buffer.get_elements_count() as gl::types::GLsizei;
                        if ctxt.version.0 == Api::Gl {
                            ctxt.glow.draw_elements_instanced_base_vertex_base_instance(
                                primitives.to_glenum(),
                                count,
                                data_type.to_glenum(),
                                offset,
                                instances_count,
                                base_vertex,
                                base_instance,
                            );
                        } else if base_vertex != 0 {
                            ctxt.extra.DrawElementsInstancedBaseVertexBaseInstanceEXT(
                                primitives.to_glenum(),
                                count,
                                data_type.to_glenum(),
                                ptr as *const _,
                                instances_count,
                                base_vertex,
                                base_instance,
                            );
                        } else {
                            ctxt.extra.DrawElementsInstancedBaseInstanceEXT(
                                primitives.to_glenum(),
                                count,
                                data_type.to_glenum(),
                                ptr as *const _,
                                instances_count,
                                base_instance,
                            );
                        }
                    } else if let Some(instances_count) = instances_count {
                        if base_vertex != 0 {
                            if ctxt.version.0 == Api::Gl
                                || ctxt.version >= &Version(Api::GlEs, 3, 2)
                            {
                                ctxt.glow.draw_elements_instanced_base_vertex(
                                    primitives.to_glenum(),
                                    buffer.get_elements_count() as gl::types::GLsizei,
                                    data_type.to_glenum(),
                                    offset,
                                    instances_count as gl::types::GLsizei,
                                    base_vertex,
                                );
                            } else if ctxt.extensions.gl_oes_draw_elements_base_vertex {
                                ctxt.extra.DrawElementsInstancedBaseVertexOES(
                                    primitives.to_glenum(),
                                    buffer.get_elements_count() as gl::types::GLsizei,
                                    data_type.to_glenum(),
                                    ptr as *const _,
                                    instances_count as gl::types::GLsizei,
                                    base_vertex,
                                );
                            } else {
                                unreachable!();
                            }
                        } else {
                            ctxt.glow.draw_elements_instanced(
                                primitives.to_glenum(),
                                buffer.get_elements_count() as gl::types::GLsizei,
                                data_type.to_glenum(),
                                offset,
                                instances_count as gl::types::GLsizei,
                            );
                        }
                    } else if base_vertex != 0 {
                        if ctxt.version.0 == Api::Gl || ctxt.version >= &Version(Api::GlEs, 3, 2) {
                            ctxt.glow.draw_elements_base_vertex(
                                primitives.to_glenum(),
                                buffer.get_elements_count() as gl::types::GLsizei,
                                data_type.to_glenum(),
                                offset,
                                base_vertex,
                            );
                        } else if ctxt.extensions.gl_oes_draw_elements_base_vertex {
                            ctxt.extra.DrawElementsBaseVertexOES(
                                primitives.to_glenum(),
                                buffer.get_elements_count() as gl::types::GLsizei,
                                data_type.to_glenum(),
                                ptr as *const _,
                                base_vertex,
                            );
                        } else {
                            unreachable!();
                        }
                    } else {
                        ctxt.glow.draw_elements(
                            primitives.to_glenum(),
                            buffer.get_elements_count() as gl::types::GLsizei,
                            data_type.to_glenum(),
                            offset,
                        );
                    }
                }
            }

            #[cfg(any(safegl_desktop, safegl_gles31))]
            IndicesSource::MultidrawArray {
                ref buffer,
                command_offset,
                primitives,
                #[cfg(safegl_gl46)]
                draw_count,
            } => {
                let offset = buffer.get_offset_bytes() + command_offset;
                let ptr: *const u8 = ptr::null_mut();
                let ptr = unsafe { ptr.add(offset) };
                let stride = buffer.get_elements_size() as gl::types::GLsizei;

                debug_assert_eq!(base_vertex, 0); // enforced earlier in this function

                if let Some(fence) = buffer.add_fence() {
                    fences.push(fence);
                }

                unsafe {
                    buffer.prepare_and_bind_for_draw_indirect(&mut ctxt);
                    let count = buffer.get_elements_count() as gl::types::GLsizei;
                    #[cfg(safegl_gl46)]
                    let counted = draw_count.map(|draw_count| {
                        let offset = bind_draw_count(&mut ctxt, &draw_count, &mut fences);
                        let max = draw_count.max_draw_count() as gl::types::GLsizei;
                        if ctxt.version >= &Version(Api::Gl, 4, 6) {
                            ctxt.extra.MultiDrawArraysIndirectCount(
                                primitives.to_glenum(),
                                ptr as *const _,
                                offset,
                                max,
                                stride,
                            );
                        } else {
                            ctxt.extra.MultiDrawArraysIndirectCountARB(
                                primitives.to_glenum(),
                                ptr as *const _,
                                offset,
                                max,
                                stride,
                            );
                        }
                    });
                    #[cfg(not(safegl_gl46))]
                    let counted: Option<()> = None;

                    // a single command doesn't need multidraw; OpenGL ES only has
                    // `GL_EXT_multi_draw_indirect`
                    if counted.is_some() {
                        // drawn with the number of commands of the buffer
                    } else if count == 0 {
                        // an empty buffer draws nothing, without multidraw
                    } else if count == 1 {
                        ctxt.glow
                            .draw_arrays_indirect_offset(primitives.to_glenum(), offset as i32);
                    } else if ctxt.version.0 == Api::GlEs {
                        ctxt.extra.MultiDrawArraysIndirectEXT(
                            primitives.to_glenum(),
                            ptr as *const _,
                            count,
                            stride,
                        );
                    } else {
                        ctxt.extra.MultiDrawArraysIndirect(
                            primitives.to_glenum(),
                            ptr as *const _,
                            count,
                            stride,
                        );
                    }
                }
            }

            #[cfg(any(safegl_desktop, safegl_gles31))]
            IndicesSource::MultidrawElement {
                ref commands,
                command_offset,
                ref indices,
                data_type,
                primitives,
                #[cfg(safegl_gl46)]
                draw_count,
            } => {
                let cmd_offset = commands.get_offset_bytes() + command_offset;
                let cmd_ptr: *const u8 = ptr::null_mut();
                let cmd_ptr = unsafe { cmd_ptr.add(cmd_offset) };
                let stride = commands.get_elements_size() as gl::types::GLsizei;

                if let Some(fence) = commands.add_fence() {
                    fences.push(fence);
                }

                if let Some(fence) = indices.add_fence() {
                    fences.push(fence);
                }

                unsafe {
                    commands.prepare_and_bind_for_draw_indirect(&mut ctxt);
                    debug_assert_eq!(base_vertex, 0); // enforced earlier in this function
                    let count = commands.get_elements_count() as gl::types::GLsizei;
                    #[cfg(safegl_gl46)]
                    let counted = draw_count.map(|draw_count| {
                        let offset = bind_draw_count(&mut ctxt, &draw_count, &mut fences);
                        let max = draw_count.max_draw_count() as gl::types::GLsizei;
                        if ctxt.version >= &Version(Api::Gl, 4, 6) {
                            ctxt.extra.MultiDrawElementsIndirectCount(
                                primitives.to_glenum(),
                                data_type.to_glenum(),
                                cmd_ptr as *const _,
                                offset,
                                max,
                                stride,
                            );
                        } else {
                            ctxt.extra.MultiDrawElementsIndirectCountARB(
                                primitives.to_glenum(),
                                data_type.to_glenum(),
                                cmd_ptr as *const _,
                                offset,
                                max,
                                stride,
                            );
                        }
                    });
                    #[cfg(not(safegl_gl46))]
                    let counted: Option<()> = None;

                    // a single command doesn't need multidraw; OpenGL ES only has
                    // `GL_EXT_multi_draw_indirect`
                    if counted.is_some() {
                        // drawn with the number of commands of the buffer
                    } else if count == 0 {
                        // an empty buffer draws nothing, without multidraw
                    } else if count == 1 {
                        ctxt.glow.draw_elements_indirect_offset(
                            primitives.to_glenum(),
                            data_type.to_glenum(),
                            cmd_offset as i32,
                        );
                    } else if ctxt.version.0 == Api::GlEs {
                        ctxt.extra.MultiDrawElementsIndirectEXT(
                            primitives.to_glenum(),
                            data_type.to_glenum(),
                            cmd_ptr as *const _,
                            count,
                            stride,
                        );
                    } else {
                        ctxt.extra.MultiDrawElementsIndirect(
                            primitives.to_glenum(),
                            data_type.to_glenum(),
                            cmd_ptr as *const _,
                            count,
                            stride,
                        );
                    }
                }
            }

            #[cfg(safegl_desktop)]
            IndicesSource::TransformFeedback {
                captured,
                stream,
                primitives,
            } => {
                let (stream, primitives) = (*stream, *primitives);
                // the GPU draws as many vertices as the capture wrote, at most the size of its
                // largest buffer, which the vertex sources must cover
                match vertices_count {
                    Some(count) if count >= captured.capacity() => (),
                    Some(_) => return Err(DrawError::CapturedVerticesOutOfBounds),
                    None => return Err(DrawError::VerticesSourcesLengthMismatch),
                }
                if ctxt.version.0 != Api::Gl {
                    return Err(DrawError::CapturedVerticesDrawNotSupported);
                }
                if (stream as gl::types::GLint)
                    >= ctxt.capabilities.limits.max_vertex_streams.unwrap_or(1)
                {
                    return Err(DrawError::CapturedVerticesDrawNotSupported);
                }
                let instanced = instances_count.is_some_and(|instances| instances != 1);
                if instanced
                    && !(ctxt.version >= &Version(Api::Gl, 4, 2)
                        || ctxt.extensions.gl_arb_transform_feedback_instanced)
                {
                    return Err(DrawError::CapturedVerticesDrawNotSupported);
                }

                if let Some(object) = captured.object(&ctxt)? {
                    let name = object.0.get();
                    unsafe {
                        match (stream, instances_count) {
                            (0, Some(instances)) if instanced => ctxt
                                .extra
                                .DrawTransformFeedbackInstanced(
                                    primitives.to_glenum(),
                                    name,
                                    instances as gl::types::GLsizei,
                                ),
                            (_, Some(instances)) if instanced => ctxt
                                .extra
                                .DrawTransformFeedbackStreamInstanced(
                                    primitives.to_glenum(),
                                    name,
                                    stream,
                                    instances as gl::types::GLsizei,
                                ),
                            (0, _) => ctxt
                                .extra
                                .DrawTransformFeedback(primitives.to_glenum(), name),
                            (_, _) => ctxt.extra.DrawTransformFeedbackStream(
                                primitives.to_glenum(),
                                name,
                                stream,
                            ),
                        }
                    }
                }
            }

            #[cfg(any(safegl_desktop, safegl_webgl))]
            IndicesSource::NoIndicesRanges { primitives, ranges } => {
                let vertices_count = match vertices_count {
                    Some(c) => c,
                    None => return Err(DrawError::VerticesSourcesLengthMismatch),
                };
                check_ranges(&ctxt, ranges, vertices_count, instances_count)?;

                let firsts: SmallVec<[gl::types::GLint; 8]> = ranges
                    .iter()
                    .map(|range| range.start as gl::types::GLint + base_vertex)
                    .collect();
                let counts: SmallVec<[gl::types::GLsizei; 8]> = ranges
                    .iter()
                    .map(|range| (range.end - range.start) as gl::types::GLsizei)
                    .collect();
                unsafe {
                    // `glMultiDrawArrays` has no base instance: each range is drawn with it
                    if cfg!(safegl_browser) {
                        #[cfg(safegl_browser)]
                        draw_array_ranges_in_browser(
                            &ctxt,
                            primitives.to_glenum(),
                            &firsts,
                            &counts,
                            base_instance,
                        );
                    } else if base_instance != 0 {
                        for (&first, &count) in firsts.iter().zip(counts.iter()) {
                            ctxt.glow.draw_arrays_instanced_base_instance(
                                primitives.to_glenum(),
                                first,
                                count,
                                1,
                                base_instance,
                            );
                        }
                    } else {
                        ctxt.extra.MultiDrawArrays(
                            primitives.to_glenum(),
                            firsts.as_ptr(),
                            counts.as_ptr(),
                            ranges.len() as gl::types::GLsizei,
                        );
                    }
                }
            }

            #[cfg(any(safegl_desktop, safegl_webgl))]
            IndicesSource::IndexBufferRanges {
                ref buffer,
                data_type,
                primitives,
                ranges,
                base_vertices,
            } => {
                check_ranges(&ctxt, ranges, buffer.get_elements_count(), instances_count)?;

                if let Some(fence) = buffer.add_fence() {
                    fences.push(fence);
                }

                let offsets: SmallVec<[*const std::os::raw::c_void; 8]> = ranges
                    .iter()
                    .map(|range| {
                        (buffer.get_offset_bytes() + range.start * data_type.get_size()) as *const _
                    })
                    .collect();
                let counts: SmallVec<[gl::types::GLsizei; 8]> = ranges
                    .iter()
                    .map(|range| (range.end - range.start) as gl::types::GLsizei)
                    .collect();
                let base_vertices: SmallVec<[gl::types::GLint; 8]> = (0..ranges.len())
                    .map(|i| base_vertices.map_or(0, |bases| bases[i]) + base_vertex)
                    .collect();
                unsafe {
                    // `glMultiDrawElementsBaseVertex` has no base instance: each range is drawn
                    // with it
                    if cfg!(safegl_browser) {
                        #[cfg(safegl_browser)]
                        draw_element_ranges_in_browser(
                            &ctxt,
                            (primitives.to_glenum(), data_type.to_glenum()),
                            &offsets,
                            &counts,
                            &base_vertices,
                            base_instance,
                        )?;
                    } else if base_instance != 0 {
                        for i in 0..ranges.len() {
                            ctxt.glow.draw_elements_instanced_base_vertex_base_instance(
                                primitives.to_glenum(),
                                counts[i],
                                data_type.to_glenum(),
                                offsets[i] as i32,
                                1,
                                base_vertices[i],
                                base_instance,
                            );
                        }
                    } else {
                        ctxt.extra.MultiDrawElementsBaseVertex(
                            primitives.to_glenum(),
                            counts.as_ptr(),
                            data_type.to_glenum(),
                            offsets.as_ptr(),
                            ranges.len() as gl::types::GLsizei,
                            base_vertices.as_ptr(),
                        );
                    }
                }
            }

            IndicesSource::NoIndices { primitives } => {
                let vertices_count = match vertices_count {
                    Some(c) => c,
                    None => return Err(DrawError::VerticesSourcesLengthMismatch),
                };

                // WebGL draws with a base instance with
                // `WEBGL_draw_instanced_base_vertex_base_instance`
                let webgl = cfg!(safegl_browser) && base_instance != 0;

                unsafe {
                    if webgl {
                        #[cfg(safegl_browser)]
                        ctxt.webgl.draw_arrays_instanced_base_instance(
                            primitives.to_glenum(),
                            base_vertex,
                            vertices_count as gl::types::GLsizei,
                            instances_count.unwrap_or(1) as gl::types::GLsizei,
                            base_instance,
                        );
                    } else if base_instance != 0 {
                        // a draw without per-instance buffers draws one instance
                        let instances_count = instances_count.unwrap_or(1) as gl::types::GLsizei;
                        if ctxt.version.0 == Api::Gl {
                            ctxt.glow.draw_arrays_instanced_base_instance(
                                primitives.to_glenum(),
                                base_vertex,
                                vertices_count as gl::types::GLsizei,
                                instances_count,
                                base_instance,
                            );
                        } else {
                            ctxt.extra.DrawArraysInstancedBaseInstanceEXT(
                                primitives.to_glenum(),
                                base_vertex,
                                vertices_count as gl::types::GLsizei,
                                instances_count,
                                base_instance,
                            );
                        }
                    } else if let Some(instances_count) = instances_count {
                        ctxt.glow.draw_arrays_instanced(
                            primitives.to_glenum(),
                            base_vertex,
                            vertices_count as gl::types::GLsizei,
                            instances_count as gl::types::GLsizei,
                        );
                    } else {
                        ctxt.glow.draw_arrays(
                            primitives.to_glenum(),
                            base_vertex,
                            vertices_count as gl::types::GLsizei,
                        );
                    }
                }
            }
        };
    };

    ctxt.state.next_draw_call_id += 1;

    // fulfilling the fences
    for fence in fences.into_iter() {
        fence.insert(&mut ctxt);
    }

    Ok(())
}

/// Returns the draw count of a multidraw, if it has one.
#[cfg(safegl_gl46)]
fn draw_count<'a>(indices: &IndicesSource<'a>) -> Option<index::DrawCount<'a>> {
    match *indices {
        IndicesSource::MultidrawArray { draw_count, .. }
        | IndicesSource::MultidrawElement { draw_count, .. } => draw_count,
        _ => None,
    }
}

/// Checks that the backend can draw ranges of vertices or indices, which desktop OpenGL draws
/// with one call and browsers with `WEBGL_multi_draw` or one by one, that the ranges are in the
/// `len` vertices or indices, and that the draw has no instances.
#[cfg(any(safegl_desktop, safegl_webgl))]
fn check_ranges(
    ctxt: &context::CommandContext<'_>,
    ranges: &[std::ops::Range<usize>],
    len: usize,
    instances_count: Option<usize>,
) -> Result<(), DrawError> {
    if ctxt.version.0 != Api::Gl && !crate::version::is_webgl() {
        return Err(DrawError::DrawRangesNotSupported);
    }
    if instances_count.is_some_and(|instances| instances != 1) {
        return Err(DrawError::InstancedDrawRangesNotSupported);
    }
    if ranges
        .iter()
        .any(|range| range.start > range.end || range.end > len)
    {
        return Err(DrawError::DrawRangeOutOfBounds);
    }
    Ok(())
}

/// Draws ranges of vertices in a browser, starting at `firsts`: with `WEBGL_multi_draw`, or one
/// by one, and with a base instance through `WEBGL_draw_instanced_base_vertex_base_instance`.
#[cfg(safegl_browser)]
unsafe fn draw_array_ranges_in_browser(
    ctxt: &context::CommandContext<'_>,
    mode: gl::types::GLenum,
    firsts: &[gl::types::GLint],
    counts: &[gl::types::GLsizei],
    base_instance: u32,
) {
    let ranges = firsts.iter().zip(counts);
    if base_instance != 0 {
        for (&first, &count) in ranges {
            ctxt.webgl
                .draw_arrays_instanced_base_instance(mode, first, count, 1, base_instance);
        }
    } else if ctxt.extensions.webgl_multi_draw {
        ctxt.webgl.multi_draw_arrays(mode, firsts, counts);
    } else {
        for (&first, &count) in ranges {
            ctxt.glow.draw_arrays(mode, first, count);
        }
    }
}

/// Draws ranges of indices in a browser, at the `offsets` in bytes in the bound index buffer:
/// with `WEBGL_multi_draw`, or one by one, and with base vertices or a base instance through
/// `WEBGL_draw_instanced_base_vertex_base_instance`, without which they return
/// `DrawError::DrawRangesNotSupported`.
#[cfg(safegl_browser)]
unsafe fn draw_element_ranges_in_browser(
    ctxt: &context::CommandContext<'_>,
    (mode, ty): (gl::types::GLenum, gl::types::GLenum),
    offsets: &[*const std::os::raw::c_void],
    counts: &[gl::types::GLsizei],
    base_vertices: &[gl::types::GLint],
    base_instance: u32,
) -> Result<(), DrawError> {
    let offsets: SmallVec<[i32; 8]> = offsets.iter().map(|&offset| offset as i32).collect();
    if base_instance != 0 || base_vertices.iter().any(|&base| base != 0) {
        if !ctxt.extensions.webgl_draw_instanced_base_vertex_base_instance {
            return Err(DrawError::DrawRangesNotSupported);
        }
        for i in 0..counts.len() {
            ctxt.webgl.draw_elements_instanced_base_vertex_base_instance(
                mode,
                counts[i],
                ty,
                offsets[i],
                1,
                base_vertices[i],
                base_instance,
            );
        }
    } else if ctxt.extensions.webgl_multi_draw {
        ctxt.webgl.multi_draw_elements(mode, counts, ty, &offsets);
    } else {
        for (&count, &offset) in counts.iter().zip(&offsets) {
            ctxt.glow.draw_elements(mode, count, ty, offset);
        }
    }
    Ok(())
}

/// Checks that the backend can draw the commands of an indirect draw: one command needs OpenGL
/// 4.0 or OpenGL ES 3.1, and several ones multidraw (OpenGL 4.3, `GL_ARB_multi_draw_indirect` or
/// `GL_EXT_multi_draw_indirect`). OpenGL ES doesn't draw indirectly while transform feedback
/// records.
#[cfg(any(safegl_desktop, safegl_gles31))]
fn check_indirect(
    ctxt: &context::CommandContext<'_>,
    indices: &IndicesSource<'_>,
    draw_parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let (commands, counted) = match *indices {
        IndicesSource::MultidrawArray { ref buffer, .. } => (buffer.get_elements_count(), false),
        IndicesSource::MultidrawElement { ref commands, .. } => {
            (commands.get_elements_count(), false)
        }
        _ => return Ok(()),
    };
    #[cfg(safegl_gl46)]
    let counted = counted || draw_count(indices).is_some();

    if !(ctxt.version.0 == Api::Gl || ctxt.version >= &Version(Api::GlEs, 3, 1)) {
        return Err(DrawError::IndirectDrawsNotSupported);
    }
    // an empty buffer draws nothing and a single command is drawn without multidraw
    if (commands > 1 || counted)
        && !(ctxt.version >= &Version(Api::Gl, 4, 3)
            || (ctxt.version.0 == Api::Gl && ctxt.extensions.gl_arb_multi_draw_indirect)
            || (ctxt.version.0 == Api::GlEs && ctxt.extensions.gl_ext_multi_draw_indirect))
    {
        return Err(DrawError::MultidrawNotSupported);
    }
    if ctxt.version.0 == Api::GlEs && draw_parameters.transform_feedback.is_some() {
        return Err(DrawError::IndirectDrawWithTransformFeedback);
    }
    Ok(())
}

/// Checks that the backend can read the number of commands of a multidraw from a buffer
/// (OpenGL 4.6 or `GL_ARB_indirect_parameters`), and that the buffer is of the context.
#[cfg(safegl_gl46)]
fn check_draw_count(
    ctxt: &context::CommandContext<'_>,
    indices: &IndicesSource<'_>,
) -> Result<(), DrawError> {
    let draw_count = match draw_count(indices) {
        Some(draw_count) => draw_count,
        None => return Ok(()),
    };
    if !ctxt.owns(draw_count.buffer().get_context()) {
        return Err(DrawError::ContextMismatch {
            object: "draw count buffer".to_owned(),
        });
    }
    let supported = ctxt.version >= &Version(Api::Gl, 4, 6)
        || (ctxt.version.0 == Api::Gl && ctxt.extensions.gl_arb_indirect_parameters);
    if supported {
        Ok(())
    } else {
        Err(DrawError::DrawCountNotSupported)
    }
}

/// Binds the buffer of a draw count to `GL_PARAMETER_BUFFER`, and returns the offset of the
/// number in it.
#[cfg(safegl_gl46)]
fn bind_draw_count<'a>(
    ctxt: &mut context::CommandContext<'_>,
    draw_count: &index::DrawCount<'a>,
    fences: &mut crate::buffer::FenceInserters<'a>,
) -> gl::types::GLintptr {
    let buffer = draw_count.buffer();
    if let Some(fence) = buffer.add_fence() {
        fences.push(fence);
    }
    buffer.prepare_and_bind_for_parameter(ctxt);
    buffer.get_offset_bytes() as gl::types::GLintptr
}

/// Returns true if draws can start from a base instance: OpenGL 4.2, `GL_ARB_base_instance`, or
/// `GL_EXT_base_instance` on OpenGL ES.
fn is_base_instance_supported(context: &Context) -> bool {
    let version = context.get_opengl_version();
    let extensions = context.get_extensions();
    match version.0 {
        Api::Gl => version >= &Version(Api::Gl, 4, 2) || extensions.gl_arb_base_instance,
        Api::GlEs => {
            extensions.gl_ext_base_instance
                || extensions.webgl_draw_instanced_base_vertex_base_instance
        }
    }
}

/// Binds the program of a draw command and the values of its uniforms.
///
/// The programs of a pipeline get their uniforms one after the other, with bind points that
/// they share, then the pipeline is bound and their subroutine uniforms set, which binding a
/// program or a pipeline resets.
fn bind_programs<'a>(
    ctxt: &mut context::CommandContext<'_>,
    program: &DrawProgram<'_>,
    uniforms: &CollectedUniforms<'a>,
    fences: &mut crate::buffer::FenceInserters<'a>,
) -> Result<(), DrawError> {
    match *program {
        DrawProgram::Program(program) => {
            program.use_program(ctxt);
            uniforms.bind(ctxt, program, fences)
        }
        #[cfg(any(safegl_gl41, safegl_gles31))]
        DrawProgram::Pipeline(_) => {
            let mut points = crate::uniforms::BindPoints::new(ctxt);
            let mut subroutines = Vec::new();
            for &stage in program.programs() {
                stage.raw().use_program(ctxt);
                subroutines.push(uniforms.bind_values(ctxt, stage, &mut points, fences)?);
            }
            program.use_programs(ctxt);
            #[cfg(safegl_desktop)]
            for (&stage, subroutines) in program.programs().iter().zip(&subroutines) {
                stage.raw().flush_subroutine_uniforms();
                crate::uniforms::bind_subroutine_uniforms(ctxt, stage, subroutines)?;
            }
            #[cfg(not(safegl_desktop))]
            let _ = subroutines;
            Ok(())
        }
    }
}

/// Checks that the primitives fit the tessellation and geometry stages of the program, and
/// returns the vertices per patch of patches.
#[cfg(any(safegl_desktop, safegl_gles32))]
fn check_stages(
    context: &Context,
    program: &DrawProgram<'_>,
    primitives: index::PrimitiveType,
) -> Result<Option<u16>, DrawError> {
    check_complete(program)?;

    let vertices_per_patch = match primitives {
        index::PrimitiveType::Patches { vertices_per_patch } => {
            if let Some(max) = context.capabilities().max_patch_vertices {
                if vertices_per_patch == 0 || vertices_per_patch as gl::types::GLint > max {
                    return Err(DrawError::UnsupportedVerticesPerPatch);
                }
            } else {
                return Err(DrawError::TessellationNotSupported);
            }

            // patches are only drawn through a tessellation evaluation shader
            if !program.has_tessellation_evaluation_shader() {
                return Err(DrawError::PatchesWithoutTessellation);
            }

            Some(vertices_per_patch)
        }
        _ => {
            // a tessellation evaluation shader only takes patches
            if program.has_tessellation_evaluation_shader() {
                return Err(DrawError::TessellationWithoutPatches);
            }

            None
        }
    };

    // a geometry shader takes a single kind of primitives, unless it gets them from the
    // tessellation stages
    if !program.has_tessellation_evaluation_shader() {
        if let Some(input) = program.get_geometry_input_type() {
            use crate::index::PrimitiveType::*;
            let compatible = match primitives {
                Points => input == gl::POINTS,
                LinesList | LineStrip | LineLoop => input == gl::LINES,
                LinesListAdjacency | LineStripAdjacency => input == gl::LINES_ADJACENCY,
                TrianglesList | TriangleStrip | TriangleFan => input == gl::TRIANGLES,
                TrianglesListAdjacency | TriangleStripAdjacency => input == gl::TRIANGLES_ADJACENCY,
                Patches { .. } => true,
            };
            if !compatible {
                return Err(DrawError::GeometryShaderInputMismatch);
            }
        }
    }

    Ok(vertices_per_patch)
}

/// The levels without tessellation and geometry shaders have no patches.
#[cfg(not(any(safegl_desktop, safegl_gles32)))]
fn check_stages(
    _: &Context,
    program: &DrawProgram<'_>,
    _: index::PrimitiveType,
) -> Result<Option<u16>, DrawError> {
    check_complete(program)?;
    Ok(None)
}

/// Checks that a separable program drawn on its own has a vertex and a fragment shader.
#[cfg(any(safegl_gl41, safegl_gles31))]
fn check_complete(program: &DrawProgram<'_>) -> Result<(), DrawError> {
    if let DrawProgram::Program(program) = program {
        let stages = program.separable_stages();
        let both = gl::VERTEX_SHADER_BIT | gl::FRAGMENT_SHADER_BIT;
        if stages != 0 && stages & both != both {
            return Err(DrawError::IncompleteProgram);
        }
    }
    Ok(())
}

/// Without program pipelines, programs aren't separable.
#[cfg(not(any(safegl_gl41, safegl_gles31)))]
fn check_complete(_: &DrawProgram<'_>) -> Result<(), DrawError> {
    Ok(())
}

unsafe fn sync_vertices_per_patch(
    ctxt: &mut context::CommandContext<'_>,
    vertices_per_patch: Option<u16>,
) {
    if let Some(vertices_per_patch) = vertices_per_patch {
        let vertices_per_patch = vertices_per_patch as gl::types::GLint;
        if ctxt.state.patch_patch_vertices != vertices_per_patch {
            context::set_patch_vertices(
                (ctxt.glow, ctxt.extra),
                ctxt.version,
                ctxt.extensions,
                vertices_per_patch,
            );
            ctxt.state.patch_patch_vertices = vertices_per_patch;
        }
    }
}

/// Returns true if the buffer belongs to the context.
fn binder_context_owns(buffer: &crate::buffer::BufferAnySlice<'_>, context: &Context) -> bool {
    std::ptr::eq(&**buffer.get_context(), context)
}

/// Checks that the program, the indices and the objects of the draw parameters belong to the
/// current context.
fn check_contexts(
    ctxt: &context::CommandContext<'_>,
    program: &DrawProgram<'_>,
    indices: &IndicesSource<'_>,
    draw_parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let mismatch = |object: &str| {
        Err(DrawError::ContextMismatch {
            object: object.to_owned(),
        })
    };

    if program
        .programs()
        .iter()
        .any(|program| !ctxt.owns(program.get_context()))
    {
        return mismatch("program");
    }

    match *indices {
        IndicesSource::IndexBuffer { ref buffer, .. } => {
            if !ctxt.owns(buffer.get_context()) {
                return mismatch("index buffer");
            }
        }
        #[cfg(any(safegl_desktop, safegl_webgl))]
        IndicesSource::IndexBufferRanges { ref buffer, .. } => {
            if !ctxt.owns(buffer.get_context()) {
                return mismatch("index buffer");
            }
        }
        #[cfg(any(safegl_desktop, safegl_gles31))]
        IndicesSource::MultidrawArray { ref buffer, .. } => {
            if !ctxt.owns(buffer.get_context()) {
                return mismatch("draw commands buffer");
            }
        }
        #[cfg(any(safegl_desktop, safegl_gles31))]
        IndicesSource::MultidrawElement {
            ref commands,
            ref indices,
            ..
        } => {
            if !ctxt.owns(commands.get_context()) {
                return mismatch("draw commands buffer");
            }
            if !ctxt.owns(indices.get_context()) {
                return mismatch("index buffer");
            }
        }
        IndicesSource::NoIndices { .. } => (),
        #[cfg(any(safegl_desktop, safegl_webgl))]
        IndicesSource::NoIndicesRanges { .. } => (),
        #[cfg(safegl_desktop)]
        IndicesSource::TransformFeedback { captured, .. } => {
            if !ctxt.owns(captured.get_context()) {
                return mismatch("captured vertices");
            }
        }
    }

    if let Some(transform_feedback) = draw_parameters.transform_feedback {
        if !ctxt.owns(transform_feedback.get_context()) {
            return mismatch("transform feedback object");
        }
        if transform_feedback.get_program_id() != program.last_vertex_stage().get_id() {
            return Err(DrawError::TransformFeedbackProgramMismatch);
        }

        // without geometry shaders, OpenGL ES only captures the primitives of `glDrawArrays`,
        // exactly as they are drawn, which must be lists
        let arrays_of_lists_only = ctxt.version.0 == Api::GlEs
            && ctxt.version < &Version(Api::GlEs, 3, 2)
            && !ctxt.extensions.gl_ext_geometry_shader
            && !ctxt.extensions.gl_oes_geometry_shader;
        let is_array_of_list = matches!(
            indices,
            IndicesSource::NoIndices {
                primitives: index::PrimitiveType::Points
                    | index::PrimitiveType::LinesList
                    | index::PrimitiveType::TrianglesList
            }
        );
        if arrays_of_lists_only && !is_array_of_list {
            return Err(DrawError::TransformFeedbackDrawNotSupported);
        }
    }

    draw_parameters::check_query_contexts(ctxt, draw_parameters)
}
