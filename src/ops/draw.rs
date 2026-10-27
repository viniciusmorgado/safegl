use std::ptr;

use glow::HasContext;

use crate::BufferExt;
use crate::BufferSliceExt;
use crate::DrawError;
use crate::GlObject;
use crate::ProgramExt;
use crate::uniforms::CollectedUniforms;

use crate::context::Context;
use crate::ContextExt;
use crate::TransformFeedbackSessionExt;

use crate::fbo::{self, FramebufferTarget};

use crate::index::{self, IndicesSource};
use crate::uniforms::Uniforms;
use crate::vertex::{MultiVerticesSource, TransformFeedbackSession, VerticesSource};
use crate::vertex_array_object::VertexAttributesSystem;
use crate::{Program, ToGlEnum};

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
    program: &Program,
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

    let vertices_per_patch = check_stages(context, program, indices.get_primitives_type())?;

    if !draw_parameters::check_queries(draw_parameters)? {
        return Ok(());
    }

    // the values of the uniforms are collected before the context is borrowed, so that the code
    // that gives them can use SafeGL
    let uniforms = CollectedUniforms::new(uniforms);

    // starting the state changes
    let mut ctxt = context.make_current();

    // OpenGL objects can only be used with the context that created them
    check_contexts(&ctxt, program, &indices, draw_parameters)?;

    // handling vertices source
    let (vertices_count, instances_count, base_vertex) = {
        let index_buffer = match indices {
            IndicesSource::IndexBuffer { buffer, .. } => Some(buffer),
            #[cfg(safegl_gl43)]
            IndicesSource::MultidrawArray { .. } => None,
            #[cfg(safegl_gl43)]
            IndicesSource::MultidrawElement { indices, .. } => Some(indices),
            IndicesSource::NoIndices { .. } => None,
        };

        // determining whether we can use the `base_vertex` variants for drawing
        let use_base_vertex = match indices {
            #[cfg(safegl_gl43)]
            IndicesSource::MultidrawArray { .. } => false,
            #[cfg(safegl_gl43)]
            IndicesSource::MultidrawElement { .. } => false,
            IndicesSource::NoIndices { .. } => true,
            _ => {
                ctxt.version.0 == Api::Gl
                    || ctxt.version >= &Version(Api::GlEs, 3, 2)
                    || ctxt.extensions.gl_oes_draw_elements_base_vertex
            }
        };

        // object that is used to build the bindings
        let mut binder =
            VertexAttributesSystem::start(&mut ctxt, program, index_buffer, use_base_vertex);
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
                VerticesSource::VertexBuffer(buffer, format, per_instance) => {
                    if !binder_context_owns(&buffer, context) {
                        return Err(DrawError::ContextMismatch {
                            object: "vertex buffer".to_owned(),
                        });
                    }

                    if let Some(fence) = buffer.add_fence() {
                        fences.push(fence);
                    }

                    binder = binder.add(&buffer, format, if per_instance { Some(1) } else { None });
                }
                _ => {}
            }

            match src {
                VerticesSource::VertexBuffer(ref buffer, _, false) => {
                    if let Some(curr) = vertices_count {
                        if curr != buffer.get_elements_count() {
                            vertices_mismatch = true;
                        }
                    } else {
                        vertices_count = Some(buffer.get_elements_count());
                    }
                }
                VerticesSource::VertexBuffer(ref buffer, _, true) => {
                    if let Some(curr) = instances_count {
                        if curr != buffer.get_elements_count() {
                            return Err(DrawError::InstancesCountMismatch);
                        }
                    } else {
                        instances_count = Some(buffer.get_elements_count());
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
        (vertices_count, instances_count, binder.bind()?.unwrap_or(0))
    };

    // binding the FBO to draw upon
    {
        let fbo_id =
            fbo::FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, framebuffer);
        unsafe { fbo::bind_framebuffer(&mut ctxt, fbo_id, true, false) };
    };

    // transform feedback can't stay active while the program changes, unless the draw command
    // continues the session
    if !draw_parameters
        .transform_feedback
        .is_some_and(|session| session.is_active(&ctxt))
    {
        TransformFeedbackSession::unbind(&mut ctxt);
    }

    // binding the program and uniforms
    program.use_program(&mut ctxt);
    uniforms.bind(&mut ctxt, program, &mut fences)?;

    // sync-ing draw_parameters
    unsafe {
        draw_parameters::sync(
            &mut ctxt,
            draw_parameters,
            dimensions,
            indices.get_primitives_type(),
        )?;
        sync_vertices_per_patch(&mut ctxt, vertices_per_patch);

        if let Some(ref tf) = draw_parameters.transform_feedback {
            tf.bind(&mut ctxt, indices.get_primitives_type());
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

                unsafe {
                    if let Some(instances_count) = instances_count {
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

            #[cfg(safegl_gl43)]
            IndicesSource::MultidrawArray {
                ref buffer,
                primitives,
            } => {
                let ptr: *const u8 = ptr::null_mut();
                let ptr = unsafe { ptr.add(buffer.get_offset_bytes()) };

                debug_assert_eq!(base_vertex, 0); // enforced earlier in this function

                if let Some(fence) = buffer.add_fence() {
                    fences.push(fence);
                }

                unsafe {
                    buffer.prepare_and_bind_for_draw_indirect(&mut ctxt);
                    let count = buffer.get_elements_count() as gl::types::GLsizei;
                    // OpenGL ES only has `GL_EXT_multi_draw_indirect`
                    if ctxt.version.0 == Api::GlEs {
                        ctxt.extra.MultiDrawArraysIndirectEXT(
                            primitives.to_glenum(),
                            ptr as *const _,
                            count,
                            0,
                        );
                    } else {
                        ctxt.extra.MultiDrawArraysIndirect(
                            primitives.to_glenum(),
                            ptr as *const _,
                            count,
                            0,
                        );
                    }
                }
            }

            #[cfg(safegl_gl43)]
            IndicesSource::MultidrawElement {
                ref commands,
                ref indices,
                data_type,
                primitives,
            } => {
                let cmd_ptr: *const u8 = ptr::null_mut();
                let cmd_ptr = unsafe { cmd_ptr.add(commands.get_offset_bytes()) };

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
                    // OpenGL ES only has `GL_EXT_multi_draw_indirect`
                    if ctxt.version.0 == Api::GlEs {
                        ctxt.extra.MultiDrawElementsIndirectEXT(
                            primitives.to_glenum(),
                            data_type.to_glenum(),
                            cmd_ptr as *const _,
                            count,
                            0,
                        );
                    } else {
                        ctxt.extra.MultiDrawElementsIndirect(
                            primitives.to_glenum(),
                            data_type.to_glenum(),
                            cmd_ptr as *const _,
                            count,
                            0,
                        );
                    }
                }
            }

            IndicesSource::NoIndices { primitives } => {
                let vertices_count = match vertices_count {
                    Some(c) => c,
                    None => return Err(DrawError::VerticesSourcesLengthMismatch),
                };

                unsafe {
                    if let Some(instances_count) = instances_count {
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

/// Checks that the primitives fit the tessellation and geometry stages of the program, and
/// returns the vertices per patch of patches.
#[cfg(any(safegl_desktop, safegl_gles32))]
fn check_stages(
    context: &Context,
    program: &Program,
    primitives: index::PrimitiveType,
) -> Result<Option<u16>, DrawError> {
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
    _: &Program,
    _: index::PrimitiveType,
) -> Result<Option<u16>, DrawError> {
    Ok(None)
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
    program: &Program,
    indices: &IndicesSource<'_>,
    draw_parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let mismatch = |object: &str| {
        Err(DrawError::ContextMismatch {
            object: object.to_owned(),
        })
    };

    if !ctxt.owns(program.get_context()) {
        return mismatch("program");
    }

    match *indices {
        IndicesSource::IndexBuffer { ref buffer, .. } => {
            if !ctxt.owns(buffer.get_context()) {
                return mismatch("index buffer");
            }
        }
        #[cfg(safegl_gl43)]
        IndicesSource::MultidrawArray { ref buffer, .. } => {
            if !ctxt.owns(buffer.get_context()) {
                return mismatch("draw commands buffer");
            }
        }
        #[cfg(safegl_gl43)]
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
    }

    if let Some(session) = draw_parameters.transform_feedback {
        if !ctxt.owns(session.get_context()) {
            return mismatch("transform feedback session");
        }
        if session.get_program_id() != program.get_id() {
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
