//! The textures of EGL images (`GL_OES_EGL_image_external`): camera frames, decoded video and
//! the buffers of other processes and APIs, which shaders sample in the format of the image, YUV
//! included, with `samplerExternalOES` uniforms.

use std::os::raw::c_void;

use crate::backend::Facade;
use crate::framebuffer::{ColorAttachment, ToColorAttachment};
use crate::texture::any::{TextureAny, TextureAnyImage};
use crate::texture::TextureCreationError;
use crate::uniforms::{
    AsUniformValue, MinifySamplerFilter, Sampler, SamplerBehavior, SamplerWrapFunction,
    UniformValue,
};
use crate::version::is_webgl;
use crate::{CapabilitiesSource, GlObject};

// WebGL has no external textures
#[cfg(not(safegl_browser))]
use crate::image_format::TextureFormatRequest;
#[cfg(not(safegl_browser))]
use crate::texture::any;
#[cfg(not(safegl_browser))]
use crate::texture::{Dimensions, MipmapsOption};
#[cfg(not(safegl_browser))]
use crate::{gl, ContextExt};

#[cfg(not(safegl_browser))]
use glow::HasContext;

/// Returns true if the backend has textures of EGL images (`GL_OES_EGL_image_external`), which
/// WebGL doesn't have. Their shaders also need `GL_OES_EGL_image_external_essl3` in GLSL ES 3.
pub fn is_external_texture_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    !is_webgl() && context.get_extensions().gl_oes_egl_image_external
}

/// The texture of an EGL image (`GL_OES_EGL_image_external`), sampled by `samplerExternalOES`
/// uniforms: the GPU converts the format of the image, YUV included, to RGB when it samples it.
///
/// The texture has a single level, which the GPU reads but SafeGL doesn't write: its content is
/// the one of the image, written by its producer, like a camera, a video decoder or another
/// process. Its samplers clamp the coordinates to the edges (`SamplerWrapFunction::Clamp`), unless
/// the backend has `GL_EXT_EGL_image_external_wrap_modes`, and don't use mipmaps; draws with other
/// samplers return `DrawError::ExternalTextureSampler`.
///
/// `backend::glutin::egl::EglImage` creates them from images; with `GL_EXT_YUV_target`, they are
/// also color attachments that shaders write in YUV with `layout(yuv) out`.
pub struct ExternalTexture(TextureAny);

impl ExternalTexture {
    /// Creates the texture of the EGL image `image` of `width` by `height` pixels
    /// (`glEGLImageTargetTexture2DOES`).
    ///
    /// Returns `TextureCreationError::TypeNotSupported` without `GL_OES_EGL_image_external`, and
    /// `TextureCreationError::FormatNotSupported` if the driver can't sample the image.
    ///
    /// # Safety
    ///
    /// `image` must be a valid `EGLImage` of the EGL display of the context, of that size, and
    /// must live as long as the texture, or be destroyed after the texture is created, which keeps
    /// its content.
    pub unsafe fn from_egl_image<F: ?Sized>(
        facade: &F,
        image: *const c_void,
        width: u32,
        height: u32,
    ) -> Result<ExternalTexture, TextureCreationError>
    where
        F: Facade,
    {
        let context = facade.get_context();
        if !is_external_texture_supported(&**context) {
            return Err(TextureCreationError::TypeNotSupported);
        }
        #[cfg(not(safegl_browser))]
        {
            let mut ctxt = context.make_current();
            if ctxt.objects_lost() {
                return Err(TextureCreationError::ContextLost);
            }
            let id = ctxt
                .glow
                .create_texture()
                .map_err(|_| TextureCreationError::ContextLost)?;
            let unit = ctxt.state.active_texture as usize;
            ctxt.glow.bind_texture(gl::TEXTURE_EXTERNAL_OES, Some(id));
            ctxt.state.texture_units[unit].texture = Some(id);
            // errors left by earlier commands would be taken for the ones of the image; each
            // `glGetError` clears one flag
            for _ in 0..8 {
                if ctxt.glow.get_error() == gl::NO_ERROR {
                    break;
                }
            }
            ctxt.extra
                .EGLImageTargetTexture2DOES(gl::TEXTURE_EXTERNAL_OES, image);
            if ctxt.glow.get_error() != gl::NO_ERROR {
                ctxt.glow.delete_texture(id);
                ctxt.state.texture_units[unit].texture = None;
                return Err(TextureCreationError::FormatNotSupported);
            }
            drop(ctxt);
            let texture = any::from_id(
                facade,
                TextureFormatRequest::AnyFloatingPoint,
                id,
                true,
                MipmapsOption::NoMipmap,
                Dimensions::External { width, height },
            );
            Ok(ExternalTexture(texture))
        }
        #[cfg(safegl_browser)]
        {
            let _ = (image, width, height);
            unreachable!("WebGL has no textures of EGL images")
        }
    }

    /// Returns the width of the texture.
    pub fn width(&self) -> u32 {
        self.0.get_width()
    }

    /// Returns the height of the texture.
    pub fn height(&self) -> u32 {
        self.0.get_height().unwrap_or(1)
    }

    /// Returns the texture, which SafeGL binds and checks like the others.
    pub(crate) fn any(&self) -> &TextureAny {
        &self.0
    }

    /// Builds a `Sampler` that tells how shaders sample the texture: by default, with a linear
    /// filter, and coordinates clamped to the edges, as external textures require.
    pub fn sampled(&self) -> Sampler<'_, ExternalTexture> {
        Sampler(self, default_sampler())
    }
}

/// The sampler of external textures by default: a linear filter without mipmaps, and coordinates
/// clamped to the edges.
fn default_sampler() -> SamplerBehavior {
    SamplerBehavior {
        wrap_function: (
            SamplerWrapFunction::Clamp,
            SamplerWrapFunction::Clamp,
            SamplerWrapFunction::Clamp,
        ),
        minify_filter: MinifySamplerFilter::Linear,
        ..Default::default()
    }
}

impl GlObject for ExternalTexture {
    type Id = glow::Texture;

    fn get_id(&self) -> glow::Texture {
        self.0.get_id()
    }
}

impl AsUniformValue for &ExternalTexture {
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Texture(&self.0, None)
    }
}

impl AsUniformValue for ExternalTexture {
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Texture(&self.0, None)
    }
}

impl AsUniformValue for Sampler<'_, ExternalTexture> {
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Texture(&self.0 .0, Some(self.1))
    }
}

/// Turns the sampler into a uniform value that borrows the texture rather than the sampler, so
/// that it can outlive the sampler.
impl<'a> From<Sampler<'a, ExternalTexture>> for UniformValue<'a> {
    fn from(sampler: Sampler<'a, ExternalTexture>) -> UniformValue<'a> {
        UniformValue::Texture(&sampler.0 .0, Some(sampler.1))
    }
}

/// External textures are color attachments with `GL_EXT_YUV_target`, written by fragment shaders
/// that declare a YUV output (`layout(yuv) out`); framebuffers return
/// `ValidationError::FormatNotRenderable` without the extension.
impl<'a> ToColorAttachment<'a> for &'a ExternalTexture {
    fn to_color_attachment(self) -> ColorAttachment<'a> {
        ColorAttachment::Texture(TextureAnyImage::first_level(&self.0))
    }
}

#[cfg(any(safegl_gl43, safegl_gles30))]
impl crate::debug::sealed::Labeled for ExternalTexture {
    fn labeled_object(&self) -> (&crate::context::Context, crate::debug::sealed::LabeledObject) {
        self.0.labeled_object()
    }
}
