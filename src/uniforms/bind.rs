/*!

Handles binding uniforms to the OpenGL state machine.

*/
use crate::gl;

#[cfg(safegl_desktop)]
use std::collections::HashMap;
#[cfg(safegl_desktop)]
use std::hash::BuildHasherDefault;

#[cfg(safegl_desktop)]
use fnv::FnvHasher;

use crate::BufferExt;
use crate::BufferSliceExt;
use crate::DrawError;
use crate::ProgramExt;
use crate::RawUniformValue;
use crate::TextureExt;

#[cfg(any(safegl_gl42, safegl_gles31))]
use crate::uniforms::ImageUnitBehavior;
use crate::uniforms::SamplerBehavior;
use crate::uniforms::UniformValue;
use crate::uniforms::Uniforms;
use crate::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};

use crate::buffer::{FenceInserters, Inserter};
use crate::context::CommandContext;
use glow::HasContext;

use crate::utils::bitsfield::Bitsfield;

use crate::context;
use crate::program;

/// The uniforms of a draw command or a compute dispatch.
///
/// They are collected before SafeGL starts to issue OpenGL commands, so that the implementations
/// of `Uniforms` and `AsUniformValue` can use SafeGL.
pub(crate) struct CollectedUniforms<'a> {
    values: smallvec::SmallVec<[(&'a str, UniformValue<'a>); 16]>,
}

impl<'a> CollectedUniforms<'a> {
    /// Collects the values of `uniforms`.
    pub fn new<U>(uniforms: &'a U) -> CollectedUniforms<'a>
    where
        U: Uniforms,
    {
        let mut values = smallvec::SmallVec::new();
        uniforms.visit_values(|name, value| values.push((name, value)));
        CollectedUniforms { values }
    }

    /// Binds the uniforms to the program, which must be current, and checks that every uniform
    /// of the program has a value.
    ///
    /// Will replace texture and buffer bind points.
    pub fn bind<P>(
        &self,
        ctxt: &mut CommandContext<'_>,
        program: &P,
        fences: &mut FenceInserters<'a>,
    ) -> Result<(), DrawError>
    where
        P: ProgramExt,
    {
        let mut texture_bind_points = Bitsfield::new();
        let mut uniform_buffer_bind_points = Bitsfield::new();
        #[cfg(any(safegl_gl42, safegl_gles31))]
        let mut shared_storage_buffer_bind_points = Bitsfield::new();
        let mut image_unit_bind_points = Bitsfield::new();
        // image uniforms that aren't set use unit 0, so SafeGL doesn't give it to other images;
        // OpenGL ES uses the units of the shader instead
        if ctxt.version.0 != crate::version::Api::GlEs {
            image_unit_bind_points.set_used(0);
        }

        // Subroutine uniforms must be bound all at once, so we collect them first and process them at the end.
        // The vec contains the uniform we want to set and the value we want to set it to.
        #[cfg(safegl_desktop)]
        let mut subroutine_bindings: HashMap<
            program::ShaderStage,
            Vec<(&program::SubroutineUniform, &str)>,
            _,
        > = HashMap::with_hasher(Default::default());

        // the locations of the uniforms given in this draw command, and the blocks bound
        let mut given_locations: smallvec::SmallVec<[gl::types::GLint; 16]> =
            smallvec::SmallVec::new();
        let mut bound_blocks: smallvec::SmallVec<[&str; 8]> = smallvec::SmallVec::new();

        for &(name, value) in self.values.iter() {
            if let Some(uniform) = program.get_uniform(name) {
                // members of blocks are reflected with the location -1
                if uniform.location == -1 {
                    return Err(DrawError::UniformBlockMember {
                        name: name.to_owned(),
                    });
                }

                if !value.is_usable_with(&uniform.ty) {
                    return Err(DrawError::UniformTypeMismatch {
                        name: name.to_owned(),
                        expected: uniform.ty,
                    });
                }

                // shadow samplers return undefined values without a depth comparison, and
                // other samplers with one
                if let UniformValue::Texture(_, sampler) = value {
                    let compares = sampler.is_some_and(|s| s.depth_texture_comparison.is_some());
                    if compares != crate::uniforms::is_shadow_sampler(uniform.ty) {
                        return Err(DrawError::DepthComparisonMismatch {
                            name: name.to_owned(),
                        });
                    }
                }

                given_locations.push(uniform.location);
                if let UniformValue::Keep = value {
                    continue;
                }

                bind_uniform(
                    ctxt,
                    &value,
                    program,
                    uniform,
                    &mut texture_bind_points,
                    &mut image_unit_bind_points,
                    name,
                )?;
            } else if let Some(block) = program.get_uniform_blocks().get(name) {
                let fence = bind_uniform_block(
                    ctxt,
                    &value,
                    block,
                    program,
                    &mut uniform_buffer_bind_points,
                    name,
                )?;
                bound_blocks.push(name);

                if let Some(fence) = fence {
                    fences.push(fence);
                }
            } else {
                #[cfg(any(safegl_gl42, safegl_gles31))]
                if let Some(fence) = bind_storage_uniform(
                    ctxt,
                    &value,
                    program,
                    &mut shared_storage_buffer_bind_points,
                    name,
                )? {
                    bound_blocks.push(name);
                    if let Some(fence) = fence {
                        fences.push(fence);
                    }
                    continue;
                }

                #[cfg(safegl_desktop)]
                if let UniformValue::Subroutine(stage, sr_name) = value {
                    if let Some(subroutine_uniform) = program
                        .get_subroutine_data()
                        .subroutine_uniforms
                        .get(&(name.into(), stage))
                    {
                        subroutine_bindings
                            .entry(stage)
                            .or_default()
                            .push((subroutine_uniform, sr_name));
                    }
                }

                // values for uniforms that the program doesn't have, like the ones that the
                // compiler removed because they aren't used, are ignored
            }
        }

        check_missing_uniforms(program, &given_locations, &bound_blocks)?;

        // Process all subroutine uniforms in one batch.
        #[cfg(safegl_desktop)]
        bind_subroutine_uniforms(ctxt, program, &subroutine_bindings)?;

        Ok(())
    }
}

/// Checks that the samplers, images and blocks of the program were given a value, and that its
/// other uniforms have one.
fn check_missing_uniforms<P>(
    program: &P,
    given_locations: &[gl::types::GLint],
    bound_blocks: &[&str],
) -> Result<(), DrawError>
where
    P: ProgramExt,
{
    // an array has the location of its first element under its own name too: the shortest name
    // of a location is reported
    let mut missing: Option<(&str, bool)> = None;
    for (name, uniform) in program.get_uniforms() {
        if uniform.location == -1 || given_locations.contains(&uniform.location) {
            continue;
        }

        // the bindings of samplers and images are shared between programs, so they must be given
        // for every draw command, while the values of the other uniforms stay in the program
        let opaque = crate::uniforms::is_opaque(uniform.ty);
        if !opaque && program.is_uniform_set(uniform.location) {
            continue;
        }

        if missing.is_none_or(|(missing, _)| name.len() < missing.len()) {
            missing = Some((name, opaque));
        }
    }
    if let Some((name, opaque)) = missing {
        let name = name.to_owned();
        return Err(if opaque {
            DrawError::UniformBindingMissing { name }
        } else {
            DrawError::UniformMissing { name }
        });
    }

    let blocks = program.get_uniform_blocks().keys();
    #[cfg(any(safegl_gl42, safegl_gles31))]
    let blocks = blocks
        .chain(program.get_shader_storage_blocks().keys())
        .chain(program.get_atomic_counters().keys());
    for name in blocks {
        if !bound_blocks.contains(&name.as_str()) {
            return Err(DrawError::UniformBindingMissing { name: name.clone() });
        }
    }

    Ok(())
}

#[cfg(safegl_desktop)]
fn bind_subroutine_uniforms<P>(
    ctxt: &mut context::CommandContext<'_>,
    program: &P,
    subroutine_bindings: &HashMap<
        program::ShaderStage,
        Vec<(&program::SubroutineUniform, &str)>,
        BuildHasherDefault<FnvHasher>,
    >,
) -> Result<(), DrawError>
where
    P: ProgramExt,
{
    let subroutine_data = program.get_subroutine_data();

    // the subroutine uniforms of a stage are reset when the program is bound, so every stage
    // that has some needs all of them
    for &(_, stage) in subroutine_data.subroutine_uniforms.keys() {
        if !subroutine_bindings.contains_key(&stage) {
            return Err(DrawError::SubroutineUniformMissing {
                stage,
                real_count: 0,
                expected_count: subroutine_data
                    .subroutine_uniforms
                    .keys()
                    .filter(|&&(_, uni_stage)| uni_stage == stage)
                    .count(),
            });
        }
    }

    for (stage, bindings) in subroutine_bindings {
        // Validate that all subroutine uniforms of this stage are set, otherwise OpenGL will throw an error.
        let set_cnt = bindings.len();
        let expected_cnt = subroutine_data
            .subroutine_uniforms
            .iter()
            .filter(|&(&(_, uni_stage), _)| *stage == uni_stage)
            .count();
        if set_cnt != expected_cnt {
            return Err(DrawError::SubroutineUniformMissing {
                stage: *stage,
                real_count: set_cnt,
                expected_count: expected_cnt,
            });
        }

        // Build the indices array
        let mut indices =
            vec![0 as gl::types::GLuint; *subroutine_data.location_counts.get(stage).unwrap()];
        for binding in bindings {
            let uniform = binding.0;
            let subroutine_str = binding.1;
            let subroutine = match uniform
                .compatible_subroutines
                .iter()
                .find(|subroutine| subroutine.name == subroutine_str)
            {
                Some(subroutine) => subroutine,
                None => {
                    return Err(DrawError::SubroutineNotFound {
                        stage: *stage,
                        name: subroutine_str.into(),
                    })
                }
            };

            indices[uniform.location as usize] = subroutine.index;
        }
        program.set_subroutine_uniforms_for_stage(ctxt, *stage, &indices);
    }
    Ok(())
}

/// Checks that a buffer bound to a block belongs to the current context and starts at an offset
/// that is a multiple of `alignment`.
fn check_block_buffer(
    ctxt: &context::CommandContext<'_>,
    buffer: &crate::buffer::BufferAnySlice<'_>,
    alignment: usize,
    name: &str,
) -> Result<(), DrawError> {
    if !ctxt.owns(buffer.get_context()) {
        return Err(DrawError::ContextMismatch {
            object: format!("uniform `{}`", name),
        });
    }

    let offset = buffer.get_offset_bytes();
    if offset % alignment != 0 {
        return Err(DrawError::BufferOffsetNotAligned {
            name: name.to_owned(),
            offset,
            alignment,
        });
    }

    Ok(())
}

fn bind_uniform_block<'a, P>(
    ctxt: &mut context::CommandContext<'_>,
    value: &UniformValue<'a>,
    block: &program::UniformBlock,
    program: &P,
    buffer_bind_points: &mut Bitsfield,
    name: &str,
) -> Result<Option<Inserter<'a>>, DrawError>
where
    P: ProgramExt,
{
    match value {
        UniformValue::Block(buffer, ref layout) => {
            match layout(block) {
                Ok(_) => (),
                Err(e) => {
                    return Err(DrawError::UniformBlockLayoutMismatch {
                        name: name.to_owned(),
                        err: e,
                    });
                }
            }

            let bind_point = buffer_bind_points
                .get_unused()
                .filter(|&point| (point as i32) < ctxt.capabilities.max_indexed_uniform_buffer)
                .ok_or_else(|| DrawError::BufferBindingOutOfRange {
                    name: name.to_owned(),
                })?;
            buffer_bind_points.set_used(bind_point);

            check_block_buffer(
                ctxt,
                buffer,
                ctxt.capabilities.uniform_buffer_offset_alignment,
                name,
            )?;
            let fence = buffer.add_fence();
            let block_id = block.id as gl::types::GLuint;

            buffer.prepare_and_bind_for_uniform(ctxt, bind_point as gl::types::GLuint);
            program.set_uniform_block_binding(ctxt, block_id, bind_point as gl::types::GLuint);

            Ok(fence)
        }
        _ => Err(DrawError::UniformValueToBlock {
            name: name.to_owned(),
        }),
    }
}

/// Binds `value` to the shader storage block or the atomic counter called `name`.
///
/// Returns `Ok(None)` if the program has neither, otherwise the fence to insert after the draw.
#[cfg(any(safegl_gl42, safegl_gles31))]
fn bind_storage_uniform<'a, P>(
    ctxt: &mut context::CommandContext<'_>,
    value: &UniformValue<'a>,
    program: &P,
    #[cfg_attr(not(any(safegl_gl43, safegl_gles31)), allow(unused_variables))]
    shared_storage_buffer_bind_points: &mut Bitsfield,
    name: &str,
) -> Result<Option<Option<Inserter<'a>>>, DrawError>
where
    P: ProgramExt,
{
    #[cfg(any(safegl_gl43, safegl_gles31))]
    if let Some(block) = program.get_shader_storage_blocks().get(name) {
        return bind_shared_storage_block(
            ctxt,
            value,
            block,
            program,
            shared_storage_buffer_bind_points,
            name,
        )
        .map(Some);
    }
    if let Some(block) = program.get_atomic_counters().get(name) {
        bind_atomic_counter(ctxt, value, block, program, name).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(any(safegl_gl43, safegl_gles31))]
fn bind_shared_storage_block<'a, P>(
    ctxt: &mut context::CommandContext<'_>,
    value: &UniformValue<'a>,
    block: &program::UniformBlock,
    program: &P,
    buffer_bind_points: &mut Bitsfield,
    name: &str,
) -> Result<Option<Inserter<'a>>, DrawError>
where
    P: ProgramExt,
{
    match value {
        UniformValue::Block(buffer, ref layout) => {
            match layout(block) {
                Ok(_) => (),
                Err(e) => {
                    return Err(DrawError::UniformBlockLayoutMismatch {
                        name: name.to_owned(),
                        err: e,
                    });
                }
            }

            // OpenGL ES has no `glShaderStorageBlockBinding`: the block uses the binding point
            // that the shader gives it
            let fixed_binding = ctxt.version.0 == crate::version::Api::GlEs;
            let bind_point = if fixed_binding {
                let point = block.initial_binding;
                if point < 0 || point >= ctxt.capabilities.max_indexed_shader_storage_buffer {
                    return Err(DrawError::BufferBindingOutOfRange {
                        name: name.to_owned(),
                    });
                }
                if buffer_bind_points.is_used(point as u16) {
                    return Err(DrawError::SharedBindingPoint {
                        name: name.to_owned(),
                    });
                }
                point as u16
            } else {
                buffer_bind_points
                    .get_unused()
                    .filter(|&point| {
                        (point as i32) < ctxt.capabilities.max_indexed_shader_storage_buffer
                    })
                    .ok_or_else(|| DrawError::BufferBindingOutOfRange {
                        name: name.to_owned(),
                    })?
            };
            buffer_bind_points.set_used(bind_point);

            check_block_buffer(
                ctxt,
                buffer,
                ctxt.capabilities.shader_storage_buffer_offset_alignment,
                name,
            )?;
            let fence = buffer.add_fence();
            let block_id = block.id as gl::types::GLuint;

            buffer.prepare_and_bind_for_shared_storage(ctxt, bind_point as gl::types::GLuint);
            if !fixed_binding {
                program.set_shader_storage_block_binding(
                    ctxt,
                    block_id,
                    bind_point as gl::types::GLuint,
                );
            }

            Ok(fence)
        }
        _ => Err(DrawError::UniformValueToBlock {
            name: name.to_owned(),
        }),
    }
}

#[cfg(any(safegl_gl42, safegl_gles31))]
fn bind_atomic_counter<'a, P>(
    ctxt: &mut context::CommandContext<'_>,
    value: &UniformValue<'a>,
    block: &program::UniformBlock,
    program: &P,
    name: &str,
) -> Result<Option<Inserter<'a>>, DrawError>
where
    P: ProgramExt,
{
    match value {
        UniformValue::Block(buffer, ref layout) => {
            match layout(block) {
                Ok(_) => (),
                Err(e) => {
                    return Err(DrawError::UniformBlockLayoutMismatch {
                        name: name.to_owned(),
                        err: e,
                    });
                }
            }

            // the binding point comes from the shader's `layout(binding = N)`
            if block.initial_binding < 0
                || block.initial_binding >= ctxt.capabilities.max_indexed_atomic_counter_buffer
            {
                return Err(DrawError::BufferBindingOutOfRange {
                    name: name.to_owned(),
                });
            }

            check_block_buffer(ctxt, buffer, 4, name)?;
            let fence = buffer.add_fence();

            buffer.prepare_and_bind_for_atomic_counter(
                ctxt,
                block.initial_binding as gl::types::GLuint,
            );

            Ok(fence)
        }
        _ => Err(DrawError::UniformValueToBlock {
            name: name.to_owned(),
        }),
    }
}

fn bind_uniform<P>(
    ctxt: &mut context::CommandContext,
    value: &UniformValue,
    program: &P,
    uniform: &program::Uniform,
    texture_bind_points: &mut Bitsfield,
    image_unit_bind_points: &mut Bitsfield,
    name: &str,
) -> Result<(), DrawError>
where
    P: ProgramExt,
{
    let location = uniform.location;

    assert!(location >= 0);

    // textures can only be used with the context that created them
    let texture_context = match *value {
        UniformValue::Texture(texture, _) => Some(TextureExt::get_context(texture)),
        #[cfg(any(safegl_desktop, safegl_gles32))]
        UniformValue::BufferTexture(ref texture) => Some(TextureExt::get_context(texture)),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        UniformValue::Image(texture, _) => Some(TextureExt::get_context(texture)),
        _ => None,
    };
    if texture_context.is_some_and(|context| !ctxt.owns(context)) {
        return Err(DrawError::ContextMismatch {
            object: format!("uniform `{}`", name),
        });
    }

    match *value {
        // the uniform keeps its value
        UniformValue::Keep => Ok(()),
        UniformValue::Block(_, _) => Err(DrawError::UniformBufferToValue {
            name: name.to_owned(),
        }),
        #[cfg(safegl_desktop)]
        UniformValue::Subroutine(_, _) => Err(DrawError::SubroutineUniformToValue {
            name: name.to_owned(),
        }),
        UniformValue::Bool(val) => {
            // Booleans get passed as integers.
            program.set_uniform(ctxt, location, &RawUniformValue::SignedInt(val as i32));
            Ok(())
        }
        UniformValue::SignedInt(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::SignedInt(val));
            Ok(())
        }
        UniformValue::UnsignedInt(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedInt(val));
            Ok(())
        }
        UniformValue::Float(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Float(val));
            Ok(())
        }
        UniformValue::Mat2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat2(val));
            Ok(())
        }
        UniformValue::Mat3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat3(val));
            Ok(())
        }
        UniformValue::Mat4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat4(val));
            Ok(())
        }
        UniformValue::Mat2x3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat2x3(val));
            Ok(())
        }
        UniformValue::Mat2x4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat2x4(val));
            Ok(())
        }
        UniformValue::Mat3x2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat3x2(val));
            Ok(())
        }
        UniformValue::Mat3x4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat3x4(val));
            Ok(())
        }
        UniformValue::Mat4x2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat4x2(val));
            Ok(())
        }
        UniformValue::Mat4x3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Mat4x3(val));
            Ok(())
        }
        UniformValue::Vec2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Vec2(val));
            Ok(())
        }
        UniformValue::Vec3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Vec3(val));
            Ok(())
        }
        UniformValue::Vec4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Vec4(val));
            Ok(())
        }
        UniformValue::IntVec2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::IntVec2(val));
            Ok(())
        }
        UniformValue::IntVec3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::IntVec3(val));
            Ok(())
        }
        UniformValue::IntVec4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::IntVec4(val));
            Ok(())
        }
        UniformValue::UnsignedIntVec2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedIntVec2(val));
            Ok(())
        }
        UniformValue::UnsignedIntVec3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedIntVec3(val));
            Ok(())
        }
        UniformValue::UnsignedIntVec4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedIntVec4(val));
            Ok(())
        }
        UniformValue::BoolVec2(val) => {
            let val_casted = [val[0] as i32, val[1] as i32];
            program.set_uniform(ctxt, location, &RawUniformValue::IntVec2(val_casted));
            Ok(())
        }
        UniformValue::BoolVec3(val) => {
            let val_casted = [val[0] as i32, val[1] as i32, val[2] as i32];
            program.set_uniform(ctxt, location, &RawUniformValue::IntVec3(val_casted));
            Ok(())
        }
        UniformValue::BoolVec4(val) => {
            let val_casted = [val[0] as i32, val[1] as i32, val[2] as i32, val[3] as i32];
            program.set_uniform(ctxt, location, &RawUniformValue::IntVec4(val_casted));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::Double(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Double(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat2(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat3(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat4(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleVec2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleVec2(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleVec3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleVec3(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleVec4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleVec4(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::Int64(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Int64(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::Int64Vec2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Int64Vec2(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::Int64Vec3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Int64Vec3(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::Int64Vec4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::Int64Vec4(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::UnsignedInt64(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedInt64(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::UnsignedInt64Vec2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedInt64Vec2(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::UnsignedInt64Vec3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedInt64Vec3(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::UnsignedInt64Vec4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::UnsignedInt64Vec4(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat2x3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat2x3(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat2x4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat2x4(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat3x2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat3x2(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat3x4(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat3x4(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat4x2(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat4x2(val));
            Ok(())
        }
        #[cfg(safegl_desktop)]
        UniformValue::DoubleMat4x3(val) => {
            program.set_uniform(ctxt, location, &RawUniformValue::DoubleMat4x3(val));
            Ok(())
        }
        UniformValue::Texture(texture, sampler) => bind_texture_uniform(
            ctxt,
            texture,
            sampler,
            location,
            program,
            texture_bind_points,
        ),
        #[cfg(any(safegl_desktop, safegl_gles32))]
        UniformValue::BufferTexture(texture) => {
            bind_texture_uniform(ctxt, &texture, None, location, program, texture_bind_points)
        }
        #[cfg(any(safegl_gl42, safegl_gles31))]
        UniformValue::Image(texture, unit) => bind_image_uniform(
            ctxt,
            texture,
            unit,
            uniform,
            program,
            image_unit_bind_points,
            name,
        ),
    }
}

fn bind_texture_uniform<P, T>(
    ctxt: &mut context::CommandContext<'_>,
    texture: &T,
    sampler: Option<SamplerBehavior>,
    location: gl::types::GLint,
    program: &P,
    texture_bind_points: &mut Bitsfield,
) -> Result<(), DrawError>
where
    P: ProgramExt,
    T: TextureExt,
{
    texture.prepare_for_access(ctxt, crate::TextureAccess::TextureFetch);

    // integer textures can only be sampled without filtering: with linear filters they are
    // incomplete and read as zero
    let sampler = sampler.map(|mut sampler| {
        if texture.is_integer() {
            sampler.magnify_filter = MagnifySamplerFilter::Nearest;
            sampler.minify_filter = match sampler.minify_filter {
                MinifySamplerFilter::Nearest | MinifySamplerFilter::Linear => {
                    MinifySamplerFilter::Nearest
                }
                _ => MinifySamplerFilter::NearestMipmapNearest,
            };
            sampler.max_anisotropy = 1;
        }
        sampler
    });

    let sampler = if let Some(sampler) = sampler {
        Some(crate::sampler_object::get_sampler(ctxt, &sampler)?)
    } else {
        None
    };

    // finding an appropriate texture unit
    let texture_unit = ctxt
        .state
        .texture_units
        .iter()
        .enumerate()
        .find(|&(unit, content)| {
            content.texture == Some(texture.get_texture_id())
                && (content.sampler == sampler || !texture_bind_points.is_used(unit as u16))
        })
        .map(|(unit, _)| unit as u16)
        .or_else(|| {
            if ctxt.state.texture_units.len()
                < ctxt.capabilities.max_combined_texture_image_units as usize
            {
                Some(ctxt.state.texture_units.len() as u16)
            } else {
                None
            }
        })
        .or_else(|| texture_bind_points.get_unused())
        .filter(|&unit| {
            (unit as gl::types::GLint) < ctxt.capabilities.max_combined_texture_image_units
        })
        .ok_or(DrawError::InsufficientTextureUnits)?;
    texture_bind_points.set_used(texture_unit);

    // updating the program to use the right unit
    program.set_uniform(
        ctxt,
        location,
        &RawUniformValue::SignedInt(texture_unit as gl::types::GLint),
    );

    // updating the state of the texture unit
    if ctxt.state.texture_units.len() <= texture_unit as usize {
        for _ in ctxt.state.texture_units.len()..texture_unit as usize + 1 {
            ctxt.state.texture_units.push(Default::default());
        }
    }

    // TODO: do better
    if ctxt.state.texture_units[texture_unit as usize].texture != Some(texture.get_texture_id())
        || ctxt.state.texture_units[texture_unit as usize].sampler != sampler
    {
        // TODO: what if it's not supported?
        if ctxt.state.active_texture != texture_unit as gl::types::GLenum {
            unsafe {
                ctxt.glow
                    .active_texture(texture_unit as gl::types::GLenum + glow::TEXTURE0)
            };
            ctxt.state.active_texture = texture_unit as gl::types::GLenum;
        }

        texture.bind_to_current(ctxt);

        if ctxt.state.texture_units[texture_unit as usize].sampler != sampler {
            unsafe {
                ctxt.glow
                    .bind_sampler(texture_unit as gl::types::GLenum, sampler);
            }
            ctxt.state.texture_units[texture_unit as usize].sampler = sampler;
        }
    }

    Ok(())
}

#[cfg(any(safegl_gl42, safegl_gles31))]
fn bind_image_uniform<P, T>(
    ctxt: &mut context::CommandContext,
    texture: &T,
    unit_behavior: ImageUnitBehavior,
    uniform: &program::Uniform,
    program: &P,
    image_unit_bind_points: &mut Bitsfield,
    name: &str,
) -> Result<(), DrawError>
where
    P: ProgramExt,
    T: TextureExt,
{
    use crate::ToGlEnum;

    // OpenGL ES can't change the unit of an image uniform: the image goes to the unit that the
    // shader gives it
    let image_unit = if ctxt.version.0 == crate::version::Api::GlEs {
        let unit = uniform.binding.unwrap_or(0);
        if !(0..=u16::MAX as i32).contains(&unit) {
            return Err(DrawError::InsufficientImageUnits);
        }
        if image_unit_bind_points.is_used(unit as u16) {
            return Err(DrawError::SharedBindingPoint {
                name: name.to_owned(),
            });
        }
        image_unit_bind_points.set_used(unit as u16);
        unit as u16
    } else {
        let image_unit = match image_unit_bind_points.get_unused() {
            Some(unit) => unit,
            None => return Err(DrawError::InsufficientImageUnits),
        };
        image_unit_bind_points.set_used(image_unit);

        // Update the program to use the right unit
        program.set_uniform(
            ctxt,
            uniform.location,
            &RawUniformValue::SignedInt(image_unit as gl::types::GLint),
        );
        image_unit
    };

    // "If layered is false, then only a single level identified by <layer> will be bound"
    let (layered, layer) = match unit_behavior.layer {
        None => (true, 0),
        Some(l) => (false, l),
    };
    use crate::uniforms::ImageUnitAccess;
    let will_write = unit_behavior.access.eq(&ImageUnitAccess::Write)
        | unit_behavior.access.eq(&ImageUnitAccess::ReadWrite);

    texture.prepare_for_access(ctxt, crate::TextureAccess::ImageUnit { will_write });

    unsafe {
        ctxt.glow.bind_image_texture(
            image_unit as gl::types::GLuint,
            Some(texture.get_texture_id()),
            unit_behavior.level as i32,
            layered,
            layer as i32,
            unit_behavior.access.to_glenum(),
            unit_behavior.format.to_glenum(),
        )
    }

    Ok(())
}
