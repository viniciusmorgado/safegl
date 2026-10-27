//! What the implementation supports for each internal format (`glGetInternalformativ` with the
//! queries of OpenGL 4.3 and `GL_ARB_internalformat_query2`).

use std::error::Error;
use std::fmt;

use glow::HasContext;

use crate::context::Context;
use crate::gl;
use crate::image_format::{self, TextureFormat, TextureFormatRequest};
use crate::version::{Api, Version};
use crate::{CapabilitiesSource, ContextExt};

/// The kind of texture, or the render buffer, whose support of a format `Context::format_info`
/// describes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FormatTarget {
    /// One-dimensional textures.
    Texture1d,
    /// Arrays of one-dimensional textures.
    Texture1dArray,
    /// Two-dimensional textures.
    Texture2d,
    /// Arrays of two-dimensional textures.
    Texture2dArray,
    /// Multisample textures.
    Texture2dMultisample,
    /// Arrays of multisample textures.
    Texture2dMultisampleArray,
    /// Three-dimensional textures.
    Texture3d,
    /// Cube maps.
    Cubemap,
    /// Arrays of cube maps.
    CubemapArray,
    /// Rectangle textures.
    TextureRectangle,
    /// Buffer textures.
    TextureBuffer,
    /// Render buffers.
    RenderBuffer,
}

impl FormatTarget {
    fn to_glenum(self) -> gl::types::GLenum {
        match self {
            FormatTarget::Texture1d => gl::TEXTURE_1D,
            FormatTarget::Texture1dArray => gl::TEXTURE_1D_ARRAY,
            FormatTarget::Texture2d => gl::TEXTURE_2D,
            FormatTarget::Texture2dArray => gl::TEXTURE_2D_ARRAY,
            FormatTarget::Texture2dMultisample => gl::TEXTURE_2D_MULTISAMPLE,
            FormatTarget::Texture2dMultisampleArray => gl::TEXTURE_2D_MULTISAMPLE_ARRAY,
            FormatTarget::Texture3d => gl::TEXTURE_3D,
            FormatTarget::Cubemap => gl::TEXTURE_CUBE_MAP,
            FormatTarget::CubemapArray => gl::TEXTURE_CUBE_MAP_ARRAY,
            FormatTarget::TextureRectangle => gl::TEXTURE_RECTANGLE,
            FormatTarget::TextureBuffer => gl::TEXTURE_BUFFER,
            FormatTarget::RenderBuffer => gl::RENDERBUFFER,
        }
    }
}

/// How well an operation is supported for a format.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FormatSupport {
    /// The operation isn't supported.
    None,
    /// The operation works, but with caveats, such as being slow.
    Caveat,
    /// The operation is fully supported.
    Full,
}

impl FormatSupport {
    fn from_glenum(value: gl::types::GLenum) -> FormatSupport {
        match value {
            gl::FULL_SUPPORT => FormatSupport::Full,
            gl::CAVEAT_SUPPORT => FormatSupport::Caveat,
            _ => FormatSupport::None,
        }
    }
}

/// What the implementation supports for an internal format and a kind of texture. See
/// `Context::format_info`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FormatInfo {
    /// Whether the format can be used with the kind of texture at all
    /// (`GL_INTERNALFORMAT_SUPPORTED`). The other fields only matter if it is.
    pub supported: bool,
    /// The largest width, height, depth and number of layers (`GL_MAX_WIDTH`, ...); 0 for the
    /// dimensions that the kind of texture doesn't have.
    pub max_size: (u32, u32, u32, u32),
    /// Whether the format can be a color, a depth and a stencil attachment.
    pub renderable: (bool, bool, bool),
    /// Whether attaching it to a framebuffer works (`GL_FRAMEBUFFER_RENDERABLE`).
    pub framebuffer_renderable: FormatSupport,
    /// Whether blending works when drawing into it (`GL_FRAMEBUFFER_BLEND`).
    pub framebuffer_blend: FormatSupport,
    /// Whether linear filters work when sampling it (`GL_FILTER`).
    pub filter: FormatSupport,
    /// Whether it can have mipmaps (`GL_MIPMAP`).
    pub mipmaps: bool,
    /// Whether `generate_mipmaps` works for it (`GL_AUTO_GENERATE_MIPMAP`).
    pub mipmap_generation: FormatSupport,
    /// Whether shaders can load, store and use atomic functions on it through image units.
    pub image: (FormatSupport, FormatSupport, FormatSupport),
    /// Whether textures of this format can have texture views (`GL_VIEW_COMPATIBILITY_CLASS`
    /// isn't `GL_NONE`).
    pub texture_views: bool,
    /// The numbers of samples that multisample textures and render buffers of this format can
    /// have, from the largest; empty for other kinds of textures.
    pub samples: Vec<u32>,
}

/// `Context::format_info` needs OpenGL 4.3 or `GL_ARB_internalformat_query2`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FormatInfoNotSupportedError;

impl fmt::Display for FormatInfoNotSupportedError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Querying the support of internal formats is not supported by the backend")
    }
}

impl Error for FormatInfoNotSupportedError {}

impl Context {
    /// Returns what the implementation supports for `format` with `target`, from
    /// `glGetInternalformativ`: whether it is supported at all, its largest dimensions, whether
    /// it is renderable, filterable and usable by image units and views, its mipmaps and its
    /// numbers of samples. The answers are cached.
    ///
    /// Needs OpenGL 4.3 or `GL_ARB_internalformat_query2`.
    #[cfg_attr(docsrs, doc(cfg(safegl_gl43)))]
    pub fn format_info(
        &self,
        target: FormatTarget,
        format: TextureFormat,
    ) -> Result<FormatInfo, FormatInfoNotSupportedError> {
        if !(self.get_opengl_version() >= &Version(Api::Gl, 4, 3)
            || self.get_extensions().gl_arb_internalformat_query2)
        {
            return Err(FormatInfoNotSupportedError);
        }
        let key = (target, format);
        if let Some(info) = self.format_infos.borrow().get(&key) {
            return Ok(info.clone());
        }

        let internal = image_format::format_request_to_glenum(
            self,
            TextureFormatRequest::Specific(format),
            image_format::RequestType::TexStorage(None),
        )
        .ok();
        let info = match internal {
            Some(internal) => self.query_format_info(target.to_glenum(), internal),
            // a format that the context doesn't know at all
            None => FormatInfo {
                supported: false,
                max_size: (0, 0, 0, 0),
                renderable: (false, false, false),
                framebuffer_renderable: FormatSupport::None,
                framebuffer_blend: FormatSupport::None,
                filter: FormatSupport::None,
                mipmaps: false,
                mipmap_generation: FormatSupport::None,
                image: (
                    FormatSupport::None,
                    FormatSupport::None,
                    FormatSupport::None,
                ),
                texture_views: false,
                samples: Vec::new(),
            },
        };
        self.format_infos.borrow_mut().insert(key, info.clone());
        Ok(info)
    }

    fn query_format_info(
        &self,
        target: gl::types::GLenum,
        format: gl::types::GLenum,
    ) -> FormatInfo {
        let ctxt = self.make_current();
        let query = |name: gl::types::GLenum| {
            let mut value = [0];
            unsafe {
                ctxt.glow
                    .get_internal_format_i32_slice(target, format, name, &mut value)
            };
            value[0]
        };
        let support = |name| FormatSupport::from_glenum(query(name) as gl::types::GLenum);
        let count = |name| query(name).max(0) as u32;

        let samples = if matches!(
            target,
            gl::TEXTURE_2D_MULTISAMPLE | gl::TEXTURE_2D_MULTISAMPLE_ARRAY | gl::RENDERBUFFER
        ) {
            let number = query(gl::NUM_SAMPLE_COUNTS).max(0) as usize;
            let mut samples = vec![0; number];
            if number > 0 {
                unsafe {
                    ctxt.glow.get_internal_format_i32_slice(
                        target,
                        format,
                        gl::SAMPLES,
                        &mut samples,
                    )
                };
            }
            samples.into_iter().map(|s| s.max(0) as u32).collect()
        } else {
            Vec::new()
        };

        FormatInfo {
            supported: query(gl::INTERNALFORMAT_SUPPORTED) != 0,
            max_size: (
                count(gl::MAX_WIDTH),
                count(gl::MAX_HEIGHT),
                count(gl::MAX_DEPTH),
                count(gl::MAX_LAYERS),
            ),
            renderable: (
                query(gl::COLOR_RENDERABLE) != 0,
                query(gl::DEPTH_RENDERABLE) != 0,
                query(gl::STENCIL_RENDERABLE) != 0,
            ),
            framebuffer_renderable: support(gl::FRAMEBUFFER_RENDERABLE),
            framebuffer_blend: support(gl::FRAMEBUFFER_BLEND),
            filter: support(gl::FILTER),
            mipmaps: query(gl::MIPMAP) != 0,
            mipmap_generation: support(gl::AUTO_GENERATE_MIPMAP),
            image: (
                support(gl::SHADER_IMAGE_LOAD),
                support(gl::SHADER_IMAGE_STORE),
                support(gl::SHADER_IMAGE_ATOMIC),
            ),
            texture_views: query(gl::VIEW_COMPATIBILITY_CLASS) as gl::types::GLenum != gl::NONE,
            samples,
        }
    }
}
