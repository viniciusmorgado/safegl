use crate::DrawError;

use crate::uniforms::{SamplerBehavior, SamplerWrapFunction};
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
    /// Builds a new sampler object.
    pub fn new(ctxt: &mut CommandContext<'_>, behavior: &SamplerBehavior) -> SamplerObject {
        // sampler objects are core in OpenGL 3.3 and OpenGL ES 3.0
        let sampler = unsafe { ctxt.glow.create_sampler().expect("glGenSamplers failed") };

        unsafe {
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_WRAP_S,
                wrap_mode(ctxt, behavior.wrap_function.0) as gl::types::GLint,
            );
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_WRAP_T,
                wrap_mode(ctxt, behavior.wrap_function.1) as gl::types::GLint,
            );
            ctxt.glow.sampler_parameter_i32(
                sampler,
                glow::TEXTURE_WRAP_R,
                wrap_mode(ctxt, behavior.wrap_function.2) as gl::types::GLint,
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

            #[cfg(any(safegl_desktop, safegl_gles32))]
            if let Some(color) = behavior.border_color {
                if is_border_clamp_supported(ctxt) {
                    ctxt.glow.sampler_parameter_f32_slice(
                        sampler,
                        glow::TEXTURE_BORDER_COLOR,
                        &color.to_f32(),
                    );
                }
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

        SamplerObject {
            id: sampler,
            destroyed: false,
        }
    }

    ///
    #[inline]
    pub fn destroy(mut self, ctxt: &mut CommandContext<'_>) {
        self.destroyed = true;

        unsafe {
            ctxt.glow.delete_sampler(self.id);
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

/// Returns the OpenGL wrap mode of `function`.
///
/// Like the anisotropy, which is clamped to the maximum the backend supports, a wrap function
/// that OpenGL ES (or WebGL) doesn't support falls back to the closest supported one,
/// `GL_CLAMP_TO_EDGE`, instead of failing.
/// Returns true if `GL_CLAMP_TO_BORDER` and border colors are supported.
#[cfg(any(safegl_desktop, safegl_gles32))]
fn is_border_clamp_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version.0 == Api::Gl
        || ctxt.version >= &Version(Api::GlEs, 3, 2)
        || ctxt.extensions.gl_ext_texture_border_clamp
        || ctxt.extensions.gl_oes_texture_border_clamp
}

fn wrap_mode(ctxt: &CommandContext<'_>, function: SamplerWrapFunction) -> gl::types::GLenum {
    let supported = match function {
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
    };

    if supported {
        function.to_glenum()
    } else {
        gl::CLAMP_TO_EDGE
    }
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
    let sampler = SamplerObject::new(ctxt, behavior);
    let id = sampler.get_id();
    ctxt.samplers.insert(*behavior, sampler);
    Ok(id)
}
