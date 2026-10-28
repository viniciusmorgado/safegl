use crate::DrawError;

use crate::uniforms::{SamplerBehavior, SamplerNotSupportedError, SamplerWrapFunction};
#[cfg(any(safegl_desktop, safegl_gles32))]
use crate::version::{Api, Version};

use crate::context::CommandContext;
use crate::gl;
use crate::GlObject;
use crate::ToGlEnum;
use glow::HasContext;

/// An OpenGL sampler object.
pub struct SamplerObject {
    id: glow::Sampler,
    destroyed: bool,
}

impl SamplerObject {
    /// Builds a new sampler object, which fails when the context is lost.
    pub fn new(
        ctxt: &mut CommandContext<'_>,
        behavior: &SamplerBehavior,
    ) -> Result<SamplerObject, DrawError> {
        // sampler objects are core in OpenGL 3.3 and OpenGL ES 3.0
        let sampler = unsafe { ctxt.glow.create_sampler() }.map_err(|_| DrawError::ContextLost)?;

        unsafe {
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_WRAP_S,
                behavior.wrap_function.0.to_glenum() as gl::types::GLint,
            );
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_WRAP_T,
                behavior.wrap_function.1.to_glenum() as gl::types::GLint,
            );
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_WRAP_R,
                behavior.wrap_function.2.to_glenum() as gl::types::GLint,
            );
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_MIN_FILTER,
                behavior.minify_filter.to_glenum() as gl::types::GLint,
            );
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_MAG_FILTER,
                behavior.magnify_filter.to_glenum() as gl::types::GLint,
            );

            if let Some(dtc) = behavior.depth_texture_comparison {
                ctxt.glow.sampler_parameter_i32(
                    sampler,
                    glow::TEXTURE_COMPARE_MODE,
                    gl::COMPARE_REF_TO_TEXTURE as gl::types::GLint,
                );
                ctxt.glow.sampler_parameter_i32(
                    sampler,
                    glow::TEXTURE_COMPARE_FUNC,
                    dtc.to_glenum() as gl::types::GLint,
                );
            }

            // `check_support` made sure that border colors are supported
            #[cfg(any(safegl_desktop, safegl_gles32))]
            if let Some(color) = behavior.border_color {
                set_border_color(ctxt, sampler, color);
            }

            let lod = behavior.level_of_detail;
            if lod != Default::default() {
                ctxt.glow
                    .sampler_parameter_f32(sampler, glow::TEXTURE_MIN_LOD, lod.min);
                ctxt.glow
                    .sampler_parameter_f32(sampler, glow::TEXTURE_MAX_LOD, lod.max);
                // `GL_TEXTURE_LOD_BIAS` is a sampler parameter of desktop OpenGL only
                #[cfg(safegl_desktop)]
                if ctxt.version.0 == Api::Gl {
                    ctxt.glow
                        .sampler_parameter_f32(sampler, glow::TEXTURE_LOD_BIAS, lod.bias);
                }
            }

            if let Some(max_value) = ctxt.capabilities.max_texture_max_anisotropy {
                let value = if behavior.max_anisotropy as f32 > max_value {
                    max_value
                } else {
                    behavior.max_anisotropy as f32
                };

                ctxt.glow
                    .sampler_parameter_f32(sampler, glow::TEXTURE_MAX_ANISOTROPY_EXT, value);
            }
        }

        Ok(SamplerObject {
            id: sampler,
            destroyed: false,
        })
    }

    ///
    #[inline]
    pub fn destroy(mut self, ctxt: &mut CommandContext<'_>) {
        self.destroyed = true;

        unsafe {
            if !ctxt.objects_lost() {
                ctxt.glow.delete_sampler(self.id);
            }
        }
    }
}

impl GlObject for SamplerObject {
    type Id = glow::Sampler;

    #[inline]
    fn get_id(&self) -> glow::Sampler {
        self.id
    }
}

impl Drop for SamplerObject {
    #[inline]
    fn drop(&mut self) {
        assert!(self.destroyed);
    }
}

/// Sets the border color of `sampler`, with the entry point of its kind: integer border colors
/// are given with `glSamplerParameterIiv` and `glSamplerParameterIuiv`, core in OpenGL 3.3 and
/// OpenGL ES 3.2, and suffixed by the border clamp extensions of OpenGL ES.
#[cfg(any(safegl_desktop, safegl_gles32))]
unsafe fn set_border_color(
    ctxt: &mut CommandContext<'_>,
    sampler: glow::Sampler,
    color: crate::uniforms::BorderColor,
) {
    use crate::handles::RawName;
    use crate::uniforms::BorderColorValue;

    let core = ctxt.version.0 == Api::Gl || ctxt.version >= &Version(Api::GlEs, 3, 2);
    let name = sampler.raw_name();
    match color.value() {
        BorderColorValue::Float(color) => {
            ctxt.glow
                .sampler_parameter_f32_slice(sampler, glow::TEXTURE_BORDER_COLOR, &color);
        }
        BorderColorValue::Int(color) => {
            if core {
                ctxt.extra
                    .SamplerParameterIiv(name, gl::TEXTURE_BORDER_COLOR, color.as_ptr());
            } else if ctxt.extensions.gl_ext_texture_border_clamp {
                ctxt.extra
                    .SamplerParameterIivEXT(name, gl::TEXTURE_BORDER_COLOR, color.as_ptr());
            } else {
                ctxt.extra
                    .SamplerParameterIivOES(name, gl::TEXTURE_BORDER_COLOR, color.as_ptr());
            }
        }
        BorderColorValue::UnsignedInt(color) => {
            if core {
                ctxt.extra
                    .SamplerParameterIuiv(name, gl::TEXTURE_BORDER_COLOR, color.as_ptr());
            } else if ctxt.extensions.gl_ext_texture_border_clamp {
                ctxt.extra
                    .SamplerParameterIuivEXT(name, gl::TEXTURE_BORDER_COLOR, color.as_ptr());
            } else {
                ctxt.extra
                    .SamplerParameterIuivOES(name, gl::TEXTURE_BORDER_COLOR, color.as_ptr());
            }
        }
    }
}

/// Returns true if `GL_CLAMP_TO_BORDER` and border colors are supported.
#[cfg(any(safegl_desktop, safegl_gles32))]
fn is_border_clamp_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version.0 == Api::Gl
        || ctxt.version >= &Version(Api::GlEs, 3, 2)
        || ctxt.extensions.gl_ext_texture_border_clamp
        || ctxt.extensions.gl_oes_texture_border_clamp
}

/// Returns true if the backend has the wrap function.
fn is_wrap_function_supported(ctxt: &CommandContext<'_>, function: SamplerWrapFunction) -> bool {
    match function {
        #[cfg(any(safegl_desktop, safegl_gles32))]
        SamplerWrapFunction::BorderClamp => is_border_clamp_supported(ctxt),
        #[cfg(safegl_gl44)]
        SamplerWrapFunction::MirrorClamp => {
            ctxt.version >= &Version(Api::Gl, 4, 4)
                || ctxt.extensions.gl_arb_texture_mirror_clamp_to_edge
                || ctxt.extensions.gl_ext_texture_mirror_clamp_to_edge
        }
        SamplerWrapFunction::Repeat | SamplerWrapFunction::Mirror | SamplerWrapFunction::Clamp => {
            true
        }
    }
}

/// Checks that the backend can sample with `behavior`: its wrap functions, its border color
/// and its anisotropy. The anisotropy above the maximum of the backend is clamped.
pub fn check_support(
    ctxt: &CommandContext<'_>,
    behavior: &SamplerBehavior,
) -> Result<(), SamplerNotSupportedError> {
    let (s, t, r) = behavior.wrap_function;
    for function in [s, t, r] {
        if !is_wrap_function_supported(ctxt, function) {
            return Err(SamplerNotSupportedError::WrapFunction(function));
        }
    }
    #[cfg(any(safegl_desktop, safegl_gles32))]
    if behavior.border_color.is_some() && !is_border_clamp_supported(ctxt) {
        return Err(SamplerNotSupportedError::WrapFunction(
            SamplerWrapFunction::BorderClamp,
        ));
    }
    if behavior.max_anisotropy > 1 && ctxt.capabilities.max_texture_max_anisotropy.is_none() {
        return Err(SamplerNotSupportedError::Anisotropy);
    }
    Ok(())
}

/// Returns the sampler corresponding to the given behavior, or a draw error if
/// samplers are not supported.
pub fn get_sampler(
    ctxt: &mut CommandContext<'_>,
    behavior: &SamplerBehavior,
) -> Result<glow::Sampler, DrawError> {
    // looking for an existing sampler
    if let Some(obj) = ctxt.samplers.get(behavior) {
        return Ok(obj.get_id());
    }

    // builds a new sampler
    check_support(ctxt, behavior).map_err(DrawError::SamplerNotSupported)?;
    let sampler = SamplerObject::new(ctxt, behavior)?;
    let id = sampler.get_id();
    ctxt.samplers.insert(*behavior, sampler);
    Ok(id)
}
