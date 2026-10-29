//! Parameters of textures that change how they are sampled: swizzle, mipmap range and the
//! component of depth-stencil textures.

use std::error::Error;
use std::fmt;

use glow::HasContext;

use crate::context::CommandContext;
use crate::gl;
use crate::texture::TextureAny;
use crate::version::{Api, Version};
use crate::{ContextExt, GlObject, TextureExt};

impl TextureAny {
    /// Sets an integer parameter of the texture, with direct state access where it exists.
    fn set_parameter(&self, ctxt: &mut CommandContext<'_>, name: gl::types::GLenum, value: i32) {
        unsafe {
            if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access
            {
                ctxt.glow.texture_parameter_i32(self.get_id(), name, value);
            } else {
                let bind_point = self.bind_to_current(ctxt);
                ctxt.glow.tex_parameter_i32(bind_point, name, value);
            }
        }
    }

    /// Returns true if the texture is a depth-stencil texture whose stencil indices are sampled
    /// instead of its depth.
    pub(crate) fn samples_stencil(&self) -> bool {
        self.depth_stencil_mode.get() == Some(gl::STENCIL_INDEX)
    }

    /// Sets the levels that sampling and mipmap generation use, from `base` to `max` included
    /// (`GL_TEXTURE_BASE_LEVEL` and `GL_TEXTURE_MAX_LEVEL`). By default they are all the levels
    /// of the texture.
    ///
    /// The other levels keep their content: limiting the range lets a texture be sampled while
    /// its other levels are being filled or rendered to, such as the levels of a depth pyramid.
    /// Multisample textures only have the range `0..=0`.
    ///
    /// Returns an error if `base` is greater than `max`, or if `max` isn't a level of the
    /// texture.
    pub fn set_mipmap_range(&self, base: u32, max: u32) -> Result<(), MipmapRangeError> {
        if base > max {
            return Err(MipmapRangeError::BaseAfterMax);
        }
        if max >= self.get_mipmap_levels() {
            return Err(MipmapRangeError::LevelOutOfRange {
                levels: self.get_mipmap_levels(),
            });
        }
        if self.mipmap_range.get() == Some((base, max)) {
            return Ok(());
        }

        let mut ctxt = self.get_context().make_current();
        self.set_parameter(&mut ctxt, gl::TEXTURE_BASE_LEVEL, base as i32);
        self.set_parameter(&mut ctxt, gl::TEXTURE_MAX_LEVEL, max as i32);
        self.mipmap_range.set(Some((base, max)));
        Ok(())
    }

    /// Returns the levels that sampling and mipmap generation use, from the first to the second
    /// included. See `set_mipmap_range`.
    pub fn mipmap_range(&self) -> (u32, u32) {
        let last = self.get_mipmap_levels() - 1;
        match self.mipmap_range.get() {
            Some((base, max)) => (base, max.min(last)),
            None => (0, last),
        }
    }

    /// Sets the component, or the constant, that sampling the texture returns in each of the
    /// red, green, blue and alpha channels (`GL_TEXTURE_SWIZZLE_RGBA`).
    ///
    /// For instance, a texture with a single red channel can be sampled as gray with
    /// `Swizzle { r: Red, g: Red, b: Red, a: One }`. Only sampling is swizzled: framebuffers,
    /// image units, reads and writes see the texture as it is.
    #[cfg(any(safegl_desktop, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles30))))]
    pub fn set_swizzle(&self, swizzle: Swizzle) {
        let values = [swizzle.r, swizzle.g, swizzle.b, swizzle.a].map(SwizzleSource::to_glenum);
        if self.swizzle.get() == Some(values) {
            return;
        }

        let mut ctxt = self.get_context().make_current();
        // `GL_TEXTURE_SWIZZLE_RGBA` doesn't exist on OpenGL ES, which sets the channels one by one
        let names = [
            gl::TEXTURE_SWIZZLE_R,
            gl::TEXTURE_SWIZZLE_G,
            gl::TEXTURE_SWIZZLE_B,
            gl::TEXTURE_SWIZZLE_A,
        ];
        for (name, value) in names.into_iter().zip(values) {
            self.set_parameter(&mut ctxt, name, value as i32);
        }
        self.swizzle.set(Some(values));
    }

    /// Returns the swizzle of the texture. See `set_swizzle`.
    #[cfg(any(safegl_desktop, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles30))))]
    pub fn swizzle(&self) -> Swizzle {
        let [r, g, b, a] = self
            .swizzle
            .get()
            .unwrap_or([gl::RED, gl::GREEN, gl::BLUE, gl::ALPHA])
            .map(SwizzleSource::from_glenum);
        Swizzle { r, g, b, a }
    }

    /// Chooses whether sampling a depth-stencil texture returns its depth or its stencil index
    /// (`GL_DEPTH_STENCIL_TEXTURE_MODE`). By default it returns the depth.
    ///
    /// With `DepthStencilTextureMode::Stencil`, the texture is sampled by `usampler*` uniforms,
    /// without filtering and without depth comparison, and returns the stencil index in the red
    /// channel. With `Depth`, it is sampled by `sampler*` or `sampler*Shadow` uniforms.
    ///
    /// Needs OpenGL 4.3, OpenGL ES 3.1, `GL_ARB_stencil_texturing` or, on WebGL,
    /// `WEBGL_stencil_texturing`. Returns an error without them, or if the texture isn't a
    /// depth-stencil texture.
    #[cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))))]
    pub fn set_depth_stencil_mode(
        &self,
        mode: DepthStencilTextureMode,
    ) -> Result<(), DepthStencilTextureModeError> {
        if self.kind() != crate::texture::TextureKind::DepthStencil {
            return Err(DepthStencilTextureModeError::NotDepthStencil);
        }
        let mut ctxt = self.get_context().make_current();
        if !(ctxt.version >= &Version(Api::Gl, 4, 3)
            || ctxt.version >= &Version(Api::GlEs, 3, 1)
            || ctxt.extensions.gl_arb_stencil_texturing
            || ctxt.extensions.webgl_stencil_texturing)
        {
            return Err(DepthStencilTextureModeError::NotSupported);
        }

        let value = match mode {
            DepthStencilTextureMode::Depth => gl::DEPTH_COMPONENT,
            DepthStencilTextureMode::Stencil => gl::STENCIL_INDEX,
        };
        if self.depth_stencil_mode.get() != Some(value) {
            self.set_parameter(&mut ctxt, gl::DEPTH_STENCIL_TEXTURE_MODE, value as i32);
            self.depth_stencil_mode.set(Some(value));
        }
        Ok(())
    }

    /// Samples the texture as if it had the sRGB variant of its format, which decodes its texels
    /// to linear colors, if `srgb`, or as its own format if not
    /// (`GL_TEXTURE_FORMAT_SRGB_OVERRIDE_EXT`). Rendering, image units, reads and writes see the
    /// texture as it is.
    ///
    /// Needs `GL_EXT_texture_format_sRGB_override`, or returns `SrgbOverrideError::NotSupported`,
    /// and a texture created with SafeGL in a format that has an sRGB variant (`U8`, `U8U8`,
    /// `U8U8U8`, `U8U8U8U8`, and the ETC2, S3TC, BPTC and ASTC formats of colors), or returns
    /// `SrgbOverrideError::NoSrgbVariant`.
    pub fn set_srgb_override(&self, srgb: bool) -> Result<(), SrgbOverrideError> {
        let format = self.created_format();
        let astc = (0x93B0..=0x93BD).contains(&format);
        let has_variant = astc
            || matches!(
                format,
                gl::R8
                    | gl::RG8
                    | gl::RGB8
                    | gl::RGBA8
                    | gl::COMPRESSED_RGB8_ETC2
                    | gl::COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2
                    | gl::COMPRESSED_RGBA8_ETC2_EAC
                    | gl::COMPRESSED_RGB_S3TC_DXT1_EXT
                    | gl::COMPRESSED_RGBA_S3TC_DXT1_EXT
                    | gl::COMPRESSED_RGBA_S3TC_DXT3_EXT
                    | gl::COMPRESSED_RGBA_S3TC_DXT5_EXT
                    | gl::COMPRESSED_RGBA_BPTC_UNORM
            );
        let mut ctxt = self.get_context().make_current();
        if crate::version::is_webgl() || !ctxt.extensions.gl_ext_texture_format_srgb_override {
            return Err(SrgbOverrideError::NotSupported);
        }
        if !has_variant {
            return Err(SrgbOverrideError::NoSrgbVariant);
        }
        let value = if srgb { gl::SRGB } else { gl::NONE };
        self.set_parameter(&mut ctxt, gl::TEXTURE_FORMAT_SRGB_OVERRIDE_EXT, value as i32);
        Ok(())
    }

    /// Returns whether sampling a depth-stencil texture returns its depth or its stencil index.
    /// See `set_depth_stencil_mode`.
    #[cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))))]
    pub fn depth_stencil_mode(&self) -> DepthStencilTextureMode {
        if self.samples_stencil() {
            DepthStencilTextureMode::Stencil
        } else {
            DepthStencilTextureMode::Depth
        }
    }
}

/// An error of `TextureAny::set_mipmap_range`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MipmapRangeError {
    /// The base level is greater than the maximum level.
    BaseAfterMax,
    /// The maximum level isn't a level of the texture.
    LevelOutOfRange {
        /// The number of levels of the texture.
        levels: u32,
    },
}

impl fmt::Display for MipmapRangeError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MipmapRangeError::BaseAfterMax => {
                fmt.write_str("The base level is greater than the maximum level")
            }
            MipmapRangeError::LevelOutOfRange { levels } => {
                write!(
                    fmt,
                    "The maximum level isn't one of the {levels} levels of the texture"
                )
            }
        }
    }
}

impl Error for MipmapRangeError {}

/// What sampling returns in each channel. See `TextureAny::set_swizzle`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg(any(safegl_desktop, safegl_gles30))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles30))))]
pub struct Swizzle {
    /// What the red channel returns.
    pub r: SwizzleSource,
    /// What the green channel returns.
    pub g: SwizzleSource,
    /// What the blue channel returns.
    pub b: SwizzleSource,
    /// What the alpha channel returns.
    pub a: SwizzleSource,
}

#[cfg(any(safegl_desktop, safegl_gles30))]
impl Default for Swizzle {
    /// Each channel returns its own component.
    fn default() -> Swizzle {
        Swizzle {
            r: SwizzleSource::Red,
            g: SwizzleSource::Green,
            b: SwizzleSource::Blue,
            a: SwizzleSource::Alpha,
        }
    }
}

/// A component of the texture, or a constant, returned in a channel by sampling.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg(any(safegl_desktop, safegl_gles30))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles30))))]
pub enum SwizzleSource {
    /// The red component.
    Red,
    /// The green component.
    Green,
    /// The blue component.
    Blue,
    /// The alpha component.
    Alpha,
    /// 0.
    Zero,
    /// 1.
    One,
}

#[cfg(any(safegl_desktop, safegl_gles30))]
impl SwizzleSource {
    fn to_glenum(self) -> gl::types::GLenum {
        match self {
            SwizzleSource::Red => gl::RED,
            SwizzleSource::Green => gl::GREEN,
            SwizzleSource::Blue => gl::BLUE,
            SwizzleSource::Alpha => gl::ALPHA,
            SwizzleSource::Zero => gl::ZERO,
            SwizzleSource::One => gl::ONE,
        }
    }

    fn from_glenum(value: gl::types::GLenum) -> SwizzleSource {
        match value {
            gl::RED => SwizzleSource::Red,
            gl::GREEN => SwizzleSource::Green,
            gl::BLUE => SwizzleSource::Blue,
            gl::ALPHA => SwizzleSource::Alpha,
            gl::ZERO => SwizzleSource::Zero,
            _ => SwizzleSource::One,
        }
    }
}

/// What sampling a depth-stencil texture returns. See `TextureAny::set_depth_stencil_mode`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))))]
pub enum DepthStencilTextureMode {
    /// The depth, the default.
    Depth,
    /// The stencil index.
    Stencil,
}

/// An error of `TextureAny::set_depth_stencil_mode`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))))]
#[non_exhaustive]
pub enum DepthStencilTextureModeError {
    /// Stencil texturing isn't supported by the backend.
    NotSupported,
    /// The texture isn't a depth-stencil texture.
    NotDepthStencil,
}

#[cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))]
impl fmt::Display for DepthStencilTextureModeError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(match self {
            DepthStencilTextureModeError::NotSupported => {
                "Sampling the stencil of depth-stencil textures is not supported by the backend"
            }
            DepthStencilTextureModeError::NotDepthStencil => {
                "The texture isn't a depth-stencil texture"
            }
        })
    }
}

#[cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))]
impl Error for DepthStencilTextureModeError {}

/// Error of `TextureAny::set_srgb_override`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SrgbOverrideError {
    /// Sampling textures as sRGB isn't supported by the backend: it needs
    /// `GL_EXT_texture_format_sRGB_override`.
    NotSupported,
    /// The format of the texture has no sRGB variant, or SafeGL didn't create the texture.
    NoSrgbVariant,
}

impl fmt::Display for SrgbOverrideError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(match self {
            SrgbOverrideError::NotSupported => {
                "Sampling textures as sRGB is not supported by the backend"
            }
            SrgbOverrideError::NoSrgbVariant => "The format of the texture has no sRGB variant",
        })
    }
}

impl Error for SrgbOverrideError {}
