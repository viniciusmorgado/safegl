/*!
This private module handles the various image formats in OpenGL.

*/
use std::error::Error;
use std::fmt;

use crate::context::Context;
use crate::gl;

use crate::version::{Api, Version};
use crate::CapabilitiesSource;
use crate::ToGlEnum;

/// Error that is returned if the format is not supported by OpenGL.
#[derive(Copy, Clone, Debug)]
pub struct FormatNotSupportedError;

impl fmt::Display for FormatNotSupportedError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Format is not supported by OpenGL")
    }
}

impl Error for FormatNotSupportedError {}

/// Texture format request.
#[derive(Copy, Clone, Debug)]
pub enum TextureFormatRequest {
    /// Request a specific format.
    Specific(TextureFormat),

    /// Request any floating-point format, normalized or not.
    AnyFloatingPoint,

    // TODO:
    // /// Request any floating-point format represented with floats.
    //AnyFloatingPointFloat,
    /// Request any compressed format.
    AnyCompressed,

    /// Request any sRGB format.
    AnySrgb,

    /// Request any compressed sRGB format.
    AnyCompressedSrgb,

    /// Request any integral format.
    AnyIntegral,

    /// Request any unsigned format.
    AnyUnsigned,

    /// Request any depth format.
    AnyDepth,

    /// Request any stencil format.
    AnyStencil,

    /// Request any depth-stencil format.
    AnyDepthStencil,
}

/// List of client-side pixel formats.
///
/// These are all the possible formats of input data when uploading to a texture.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientFormat {
    U8,
    U8U8,
    U8U8U8,
    U8U8U8U8,
    I8,
    I8I8,
    I8I8I8,
    I8I8I8I8,
    U16,
    U16U16,
    U16U16U16,
    U16U16U16U16,
    I16,
    I16I16,
    I16I16I16,
    I16I16I16I16,
    U32,
    U32U32,
    U32U32U32,
    U32U32U32U32,
    I32,
    I32I32,
    I32I32I32,
    I32I32I32I32,
    U3U3U2,
    U5U6U5,
    U4U4U4U4,
    U5U5U5U1,
    U1U5U5U5Reversed,
    U10U10U10U2,
    F16,
    F16F16,
    F16F16F16,
    F16F16F16F16,
    F32,
    F32F32,
    F32F32F32,
    F32F32F32F32,
}

impl ClientFormat {
    /// Returns the size in bytes of a pixel of this type.
    pub fn get_size(&self) -> usize {
        use std::mem;

        match *self {
            ClientFormat::U8 => 1 * mem::size_of::<u8>(),
            ClientFormat::U8U8 => 2 * mem::size_of::<u8>(),
            ClientFormat::U8U8U8 => 3 * mem::size_of::<u8>(),
            ClientFormat::U8U8U8U8 => 4 * mem::size_of::<u8>(),
            ClientFormat::I8 => 1 * mem::size_of::<i8>(),
            ClientFormat::I8I8 => 2 * mem::size_of::<i8>(),
            ClientFormat::I8I8I8 => 3 * mem::size_of::<i8>(),
            ClientFormat::I8I8I8I8 => 4 * mem::size_of::<i8>(),
            ClientFormat::U16 => 1 * mem::size_of::<u16>(),
            ClientFormat::U16U16 => 2 * mem::size_of::<u16>(),
            ClientFormat::U16U16U16 => 3 * mem::size_of::<u16>(),
            ClientFormat::U16U16U16U16 => 4 * mem::size_of::<u16>(),
            ClientFormat::I16 => 1 * mem::size_of::<i16>(),
            ClientFormat::I16I16 => 2 * mem::size_of::<i16>(),
            ClientFormat::I16I16I16 => 3 * mem::size_of::<i16>(),
            ClientFormat::I16I16I16I16 => 4 * mem::size_of::<i16>(),
            ClientFormat::U32 => 1 * mem::size_of::<u32>(),
            ClientFormat::U32U32 => 2 * mem::size_of::<u32>(),
            ClientFormat::U32U32U32 => 3 * mem::size_of::<u32>(),
            ClientFormat::U32U32U32U32 => 4 * mem::size_of::<u32>(),
            ClientFormat::I32 => 1 * mem::size_of::<i32>(),
            ClientFormat::I32I32 => 2 * mem::size_of::<i32>(),
            ClientFormat::I32I32I32 => 3 * mem::size_of::<i32>(),
            ClientFormat::I32I32I32I32 => 4 * mem::size_of::<i32>(),
            // 3 + 3 + 2 bits
            ClientFormat::U3U3U2 => 1,
            ClientFormat::U5U6U5 => (5 + 6 + 5) / 8,
            ClientFormat::U4U4U4U4 => (4 + 4 + 4 + 4) / 8,
            ClientFormat::U5U5U5U1 => (5 + 5 + 5 + 1) / 8,
            ClientFormat::U1U5U5U5Reversed => (1 + 5 + 5 + 5) / 8,
            ClientFormat::U10U10U10U2 => (10 + 10 + 10 + 2) / 8,
            ClientFormat::F16 => 16 / 8,
            ClientFormat::F16F16 => (16 + 16) / 8,
            ClientFormat::F16F16F16 => (16 + 16 + 16) / 8,
            ClientFormat::F16F16F16F16 => (16 + 16 + 16 + 16) / 8,
            ClientFormat::F32 => 1 * mem::size_of::<f32>(),
            ClientFormat::F32F32 => 2 * mem::size_of::<f32>(),
            ClientFormat::F32F32F32 => 3 * mem::size_of::<f32>(),
            ClientFormat::F32F32F32F32 => 4 * mem::size_of::<f32>(),
        }
    }

    /// Returns the number of components of this client format.
    pub fn get_num_components(&self) -> u8 {
        match *self {
            ClientFormat::U8 => 1,
            ClientFormat::U8U8 => 2,
            ClientFormat::U8U8U8 => 3,
            ClientFormat::U8U8U8U8 => 4,
            ClientFormat::I8 => 1,
            ClientFormat::I8I8 => 2,
            ClientFormat::I8I8I8 => 3,
            ClientFormat::I8I8I8I8 => 4,
            ClientFormat::U16 => 1,
            ClientFormat::U16U16 => 2,
            ClientFormat::U16U16U16 => 3,
            ClientFormat::U16U16U16U16 => 4,
            ClientFormat::I16 => 1,
            ClientFormat::I16I16 => 2,
            ClientFormat::I16I16I16 => 3,
            ClientFormat::I16I16I16I16 => 4,
            ClientFormat::U32 => 1,
            ClientFormat::U32U32 => 2,
            ClientFormat::U32U32U32 => 3,
            ClientFormat::U32U32U32U32 => 4,
            ClientFormat::I32 => 1,
            ClientFormat::I32I32 => 2,
            ClientFormat::I32I32I32 => 3,
            ClientFormat::I32I32I32I32 => 4,
            ClientFormat::U3U3U2 => 3,
            ClientFormat::U5U6U5 => 3,
            ClientFormat::U4U4U4U4 => 4,
            ClientFormat::U5U5U5U1 => 4,
            ClientFormat::U1U5U5U5Reversed => 4,
            ClientFormat::U10U10U10U2 => 4,
            ClientFormat::F16 => 1,
            ClientFormat::F16F16 => 2,
            ClientFormat::F16F16F16 => 3,
            ClientFormat::F16F16F16F16 => 4,
            ClientFormat::F32 => 1,
            ClientFormat::F32F32 => 2,
            ClientFormat::F32F32F32 => 3,
            ClientFormat::F32F32F32F32 => 4,
        }
    }
}

/// List of uncompressed pixel formats that contain floating-point-like data.
///
/// Some formats are marked as "guaranteed to be supported". What this means is that you are
/// certain that the backend will use exactly these formats. If you try to use a format that
/// is not supported by the backend, it will automatically fall back to a larger format.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UncompressedFloatFormat {
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U8,
    ///
    ///
    /// Guaranteed to be supported for textures.
    I8,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U16,
    ///
    ///
    /// Guaranteed to be supported for textures.
    I16,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U8U8,
    ///
    ///
    /// Guaranteed to be supported for textures.
    I8I8,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U16U16,
    ///
    ///
    /// Guaranteed to be supported for textures.
    I16I16,
    ///
    U3U3U2,
    ///
    U4U4U4,
    ///
    U5U5U5,
    /// Five bits of red, six of green and five of blue, common for embedded displays.
    U5U6U5,
    ///
    ///
    /// Guaranteed to be supported for textures.
    U8U8U8,
    ///
    ///
    /// Guaranteed to be supported for textures.
    I8I8I8,
    ///
    U10U10U10,
    ///
    U12U12U12,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U16U16U16,
    ///
    ///
    /// Guaranteed to be supported for textures.
    I16I16I16,
    ///
    U2U2U2U2,
    ///
    U4U4U4U4,
    ///
    U5U5U5U1,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U8U8U8U8,
    ///
    ///
    /// Guaranteed to be supported for textures.
    I8I8I8I8,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U10U10U10U2,
    ///
    U12U12U12U12,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    U16U16U16U16,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    I16I16I16I16,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    F16,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    F16F16,
    ///
    ///
    /// Guaranteed to be supported for textures.
    F16F16F16,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    F16F16F16F16,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    F32,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    F32F32,
    ///
    ///
    /// Guaranteed to be supported for textures.
    F32F32F32,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    F32F32F32F32,
    ///
    ///
    /// Guaranteed to be supported for both textures and renderbuffers.
    F11F11F10,
    /// Uses three components of 9 bits of precision that all share the same exponent.
    ///
    /// Use this format only if all the components are approximately equal.
    ///
    /// Guaranteed to be supported for textures.
    F9F9F9,
}

impl UncompressedFloatFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<UncompressedFloatFormat> {
        vec![
            UncompressedFloatFormat::U8,
            UncompressedFloatFormat::I8,
            UncompressedFloatFormat::U16,
            UncompressedFloatFormat::I16,
            UncompressedFloatFormat::U8U8,
            UncompressedFloatFormat::I8I8,
            UncompressedFloatFormat::U16U16,
            UncompressedFloatFormat::I16I16,
            UncompressedFloatFormat::U3U3U2,
            UncompressedFloatFormat::U4U4U4,
            UncompressedFloatFormat::U5U5U5,
            UncompressedFloatFormat::U5U6U5,
            UncompressedFloatFormat::U8U8U8,
            UncompressedFloatFormat::I8I8I8,
            UncompressedFloatFormat::U10U10U10,
            UncompressedFloatFormat::U12U12U12,
            UncompressedFloatFormat::U16U16U16,
            UncompressedFloatFormat::I16I16I16,
            UncompressedFloatFormat::U2U2U2U2,
            UncompressedFloatFormat::U4U4U4U4,
            UncompressedFloatFormat::U5U5U5U1,
            UncompressedFloatFormat::U8U8U8U8,
            UncompressedFloatFormat::I8I8I8I8,
            UncompressedFloatFormat::U10U10U10U2,
            UncompressedFloatFormat::U12U12U12U12,
            UncompressedFloatFormat::U16U16U16U16,
            UncompressedFloatFormat::I16I16I16I16,
            UncompressedFloatFormat::F16,
            UncompressedFloatFormat::F16F16,
            UncompressedFloatFormat::F16F16F16,
            UncompressedFloatFormat::F16F16F16F16,
            UncompressedFloatFormat::F32,
            UncompressedFloatFormat::F32F32,
            UncompressedFloatFormat::F32F32F32,
            UncompressedFloatFormat::F32F32F32F32,
            UncompressedFloatFormat::F11F11F10,
            UncompressedFloatFormat::F9F9F9,
        ]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::UncompressedFloat(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        let version = context.get_version();
        let extensions = context.get_extensions();

        // OpenGL ES 3.0 only has a subset of the desktop formats: the 16-bit normalized formats
        // need `GL_EXT_texture_norm16`, and the legacy formats below 8 bits per channel (apart
        // from RGBA4 and RGB5_A1) and the 12-bit ones don't exist
        let desktop = version.0 == Api::Gl;
        let norm16 = desktop || extensions.gl_ext_texture_norm16;

        match self {
            Self::U16
            | Self::U16U16
            | Self::U16U16U16
            | Self::U16U16U16U16
            | Self::I16
            | Self::I16I16
            | Self::I16I16I16
            | Self::I16I16I16I16 => norm16,
            Self::U3U3U2
            | Self::U4U4U4
            | Self::U5U5U5
            | Self::U10U10U10
            | Self::U12U12U12
            | Self::U2U2U2U2
            | Self::U12U12U12U12 => desktop,
            // `GL_RGB565` is core in OpenGL 4.1
            Self::U5U6U5 => {
                !desktop
                    || version >= &Version(Api::Gl, 4, 1)
                    || extensions.gl_arb_es2_compatibility
            }
            Self::U8
            | Self::U8U8
            | Self::I8
            | Self::I8I8
            | Self::I8I8I8
            | Self::I8I8I8I8
            | Self::U8U8U8
            | Self::U4U4U4U4
            | Self::U5U5U5U1
            | Self::U8U8U8U8
            | Self::U10U10U10U2
            | Self::F16
            | Self::F16F16
            | Self::F16F16F16
            | Self::F16F16F16F16
            | Self::F32
            | Self::F32F32
            | Self::F32F32F32
            | Self::F32F32F32F32
            | Self::F11F11F10
            | Self::F9F9F9 => true,
        }
    }

    /// Returns true if a texture or renderbuffer with this format can be used as a framebuffer
    /// attachment.
    pub fn is_color_renderable<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // this is the only format that is never renderable
        if matches!(self, UncompressedFloatFormat::F9F9F9) {
            return false;
        }

        // checking whether it's supported, so that we don't return `true` by accident
        if !self.is_supported(context) {
            return false;
        }

        let version = context.get_version();
        let extensions = context.get_extensions();

        // if we have OpenGL, everything here is color-renderable
        if version.0 == Api::Gl {
            return true;
        }

        // if we have OpenGL ES, it depends
        match self {
            UncompressedFloatFormat::U8
            | UncompressedFloatFormat::U8U8
            | UncompressedFloatFormat::U5U6U5
            | UncompressedFloatFormat::U8U8U8
            | UncompressedFloatFormat::U4U4U4U4
            | UncompressedFloatFormat::U5U5U5U1
            | UncompressedFloatFormat::U8U8U8U8
            | UncompressedFloatFormat::U10U10U10U2 => true,
            UncompressedFloatFormat::U16
            | UncompressedFloatFormat::U16U16
            | UncompressedFloatFormat::U16U16U16U16 => extensions.gl_ext_texture_norm16,
            UncompressedFloatFormat::F16
            | UncompressedFloatFormat::F16F16
            | UncompressedFloatFormat::F16F16F16F16 => {
                version >= &Version(Api::GlEs, 3, 2)
                    || extensions.gl_ext_color_buffer_float
                    || extensions.gl_ext_color_buffer_half_float
            }
            UncompressedFloatFormat::F32
            | UncompressedFloatFormat::F32F32
            | UncompressedFloatFormat::F32F32F32F32
            | UncompressedFloatFormat::F11F11F10 => {
                version >= &Version(Api::GlEs, 3, 2) || extensions.gl_ext_color_buffer_float
            }
            _ => false,
        }
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            UncompressedFloatFormat::U8 => gl::R8,
            UncompressedFloatFormat::I8 => gl::R8_SNORM,
            UncompressedFloatFormat::U16 => gl::R16,
            UncompressedFloatFormat::I16 => gl::R16_SNORM,
            UncompressedFloatFormat::U8U8 => gl::RG8,
            UncompressedFloatFormat::I8I8 => gl::RG8_SNORM,
            UncompressedFloatFormat::U16U16 => gl::RG16,
            UncompressedFloatFormat::I16I16 => gl::RG16_SNORM,
            UncompressedFloatFormat::U3U3U2 => gl::R3_G3_B2,
            UncompressedFloatFormat::U4U4U4 => gl::RGB4,
            UncompressedFloatFormat::U5U5U5 => gl::RGB5,
            UncompressedFloatFormat::U5U6U5 => gl::RGB565,
            UncompressedFloatFormat::U8U8U8 => gl::RGB8,
            UncompressedFloatFormat::I8I8I8 => gl::RGB8_SNORM,
            UncompressedFloatFormat::U10U10U10 => gl::RGB10,
            UncompressedFloatFormat::U12U12U12 => gl::RGB12,
            UncompressedFloatFormat::U16U16U16 => gl::RGB16,
            UncompressedFloatFormat::I16I16I16 => gl::RGB16_SNORM,
            UncompressedFloatFormat::U2U2U2U2 => gl::RGBA2,
            UncompressedFloatFormat::U4U4U4U4 => gl::RGBA4,
            UncompressedFloatFormat::U5U5U5U1 => gl::RGB5_A1,
            UncompressedFloatFormat::U8U8U8U8 => gl::RGBA8,
            UncompressedFloatFormat::I8I8I8I8 => gl::RGBA8_SNORM,
            UncompressedFloatFormat::U10U10U10U2 => gl::RGB10_A2,
            UncompressedFloatFormat::U12U12U12U12 => gl::RGBA12,
            UncompressedFloatFormat::U16U16U16U16 => gl::RGBA16,
            UncompressedFloatFormat::I16I16I16I16 => gl::RGBA16_SNORM,
            UncompressedFloatFormat::F16 => gl::R16F,
            UncompressedFloatFormat::F16F16 => gl::RG16F,
            UncompressedFloatFormat::F16F16F16 => gl::RGB16F,
            UncompressedFloatFormat::F16F16F16F16 => gl::RGBA16F,
            UncompressedFloatFormat::F32 => gl::R32F,
            UncompressedFloatFormat::F32F32 => gl::RG32F,
            UncompressedFloatFormat::F32F32F32 => gl::RGB32F,
            UncompressedFloatFormat::F32F32F32F32 => gl::RGBA32F,
            UncompressedFloatFormat::F11F11F10 => gl::R11F_G11F_B10F,
            UncompressedFloatFormat::F9F9F9 => gl::RGB9_E5,
        }
    }
}

/// List of uncompressed pixel formats that contain floating-point data in the sRGB color space.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SrgbFormat {
    U8U8U8,
    U8U8U8U8,
}

impl SrgbFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<SrgbFormat> {
        vec![SrgbFormat::U8U8U8, SrgbFormat::U8U8U8U8]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::Srgb(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, _context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // both are core in every version that SafeGL supports
        true
    }

    /// Returns true if a texture or renderbuffer with this format can be used as a framebuffer
    /// attachment.
    pub fn is_color_renderable<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // checking whether it's supported, so that we don't return `true` by accident
        if !self.is_supported(context) {
            return false;
        }

        // OpenGL ES can't render to `GL_SRGB8`
        match self {
            SrgbFormat::U8U8U8 => context.get_version().0 == Api::Gl,
            SrgbFormat::U8U8U8U8 => true,
        }
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            SrgbFormat::U8U8U8 => gl::SRGB8,
            SrgbFormat::U8U8U8U8 => gl::SRGB8_ALPHA8,
        }
    }
}

/// List of uncompressed pixel formats that contain signed integral data.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UncompressedIntFormat {
    I8,
    I16,
    I32,
    I8I8,
    I16I16,
    I32I32,
    I8I8I8,
    /// May not be supported by renderbuffers.
    I16I16I16,
    /// May not be supported by renderbuffers.
    I32I32I32,
    /// May not be supported by renderbuffers.
    I8I8I8I8,
    I16I16I16I16,
    I32I32I32I32,
}

impl UncompressedIntFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<UncompressedIntFormat> {
        vec![
            UncompressedIntFormat::I8,
            UncompressedIntFormat::I16,
            UncompressedIntFormat::I32,
            UncompressedIntFormat::I8I8,
            UncompressedIntFormat::I16I16,
            UncompressedIntFormat::I32I32,
            UncompressedIntFormat::I8I8I8,
            UncompressedIntFormat::I16I16I16,
            UncompressedIntFormat::I32I32I32,
            UncompressedIntFormat::I8I8I8I8,
            UncompressedIntFormat::I16I16I16I16,
            UncompressedIntFormat::I32I32I32I32,
        ]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::UncompressedIntegral(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, _context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // every integral format is core in OpenGL 3.0 and OpenGL ES 3.0
        true
    }

    /// Returns true if a texture or renderbuffer with this format can be used as a framebuffer
    /// attachment.
    pub fn is_color_renderable<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // OpenGL ES can't render to the formats with three components
        context.get_version().0 == Api::Gl
            || !matches!(
                self,
                UncompressedIntFormat::I8I8I8
                    | UncompressedIntFormat::I16I16I16
                    | UncompressedIntFormat::I32I32I32
            )
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            UncompressedIntFormat::I8 => gl::R8I,
            UncompressedIntFormat::I16 => gl::R16I,
            UncompressedIntFormat::I32 => gl::R32I,
            UncompressedIntFormat::I8I8 => gl::RG8I,
            UncompressedIntFormat::I16I16 => gl::RG16I,
            UncompressedIntFormat::I32I32 => gl::RG32I,
            UncompressedIntFormat::I8I8I8 => gl::RGB8I,
            UncompressedIntFormat::I16I16I16 => gl::RGB16I,
            UncompressedIntFormat::I32I32I32 => gl::RGB32I,
            UncompressedIntFormat::I8I8I8I8 => gl::RGBA8I,
            UncompressedIntFormat::I16I16I16I16 => gl::RGBA16I,
            UncompressedIntFormat::I32I32I32I32 => gl::RGBA32I,
        }
    }
}

/// List of uncompressed pixel formats that contain unsigned integral data.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UncompressedUintFormat {
    U8,
    U16,
    U32,
    U8U8,
    U16U16,
    U32U32,
    U8U8U8,
    /// May not be supported by renderbuffers.
    U16U16U16,
    /// May not be supported by renderbuffers.
    U32U32U32,
    /// May not be supported by renderbuffers.
    U8U8U8U8,
    U16U16U16U16,
    U32U32U32U32,
    U10U10U10U2,
}

impl UncompressedUintFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<UncompressedUintFormat> {
        vec![
            UncompressedUintFormat::U8,
            UncompressedUintFormat::U16,
            UncompressedUintFormat::U32,
            UncompressedUintFormat::U8U8,
            UncompressedUintFormat::U16U16,
            UncompressedUintFormat::U32U32,
            UncompressedUintFormat::U8U8U8,
            UncompressedUintFormat::U16U16U16,
            UncompressedUintFormat::U32U32U32,
            UncompressedUintFormat::U8U8U8U8,
            UncompressedUintFormat::U16U16U16U16,
            UncompressedUintFormat::U32U32U32U32,
            UncompressedUintFormat::U10U10U10U2,
        ]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::UncompressedUnsigned(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, _context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // every integral format is core in OpenGL 3.0 and OpenGL ES 3.0
        true
    }

    /// Returns true if a texture or renderbuffer with this format can be used as a framebuffer
    /// attachment.
    pub fn is_color_renderable<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // OpenGL ES can't render to the formats with three components
        context.get_version().0 == Api::Gl
            || !matches!(
                self,
                UncompressedUintFormat::U8U8U8
                    | UncompressedUintFormat::U16U16U16
                    | UncompressedUintFormat::U32U32U32
            )
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            UncompressedUintFormat::U8 => gl::R8UI,
            UncompressedUintFormat::U16 => gl::R16UI,
            UncompressedUintFormat::U32 => gl::R32UI,
            UncompressedUintFormat::U8U8 => gl::RG8UI,
            UncompressedUintFormat::U16U16 => gl::RG16UI,
            UncompressedUintFormat::U32U32 => gl::RG32UI,
            UncompressedUintFormat::U8U8U8 => gl::RGB8UI,
            UncompressedUintFormat::U16U16U16 => gl::RGB16UI,
            UncompressedUintFormat::U32U32U32 => gl::RGB32UI,
            UncompressedUintFormat::U8U8U8U8 => gl::RGBA8UI,
            UncompressedUintFormat::U16U16U16U16 => gl::RGBA16UI,
            UncompressedUintFormat::U32U32U32U32 => gl::RGBA32UI,
            UncompressedUintFormat::U10U10U10U2 => gl::RGB10_A2UI,
        }
    }
}

/// List of compressed texture formats.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum CompressedFormat {
    /// Red/green compressed texture with one unsigned component.
    RgtcFormatU,
    /// Red/green compressed texture with one signed component.
    RgtcFormatI,
    /// Red/green compressed texture with two unsigned components.
    RgtcFormatUU,
    /// Red/green compressed texture with two signed components.
    RgtcFormatII,

    /// BPTC format with four components represented as integers.
    BptcUnorm4,
    /// BPTC format with three components (no alpha) represented as signed floats.
    BptcSignedFloat3,
    /// BPTC format with three components (no alpha) represented as unsigned floats.
    BptcUnsignedFloat3,

    /// S3TC DXT1 without alpha, see <https://www.opengl.org/wiki/S3_Texture_Compression>.
    S3tcDxt1NoAlpha,
    /// S3TC DXT1 with 1-bit alpha, see <https://www.opengl.org/wiki/S3_Texture_Compression>.
    S3tcDxt1Alpha,
    /// S3TC DXT3, see <https://www.opengl.org/wiki/S3_Texture_Compression>.
    S3tcDxt3Alpha,
    /// S3TC DXT5, see <https://www.opengl.org/wiki/S3_Texture_Compression>.
    S3tcDxt5Alpha,
}

impl CompressedFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<CompressedFormat> {
        vec![
            CompressedFormat::RgtcFormatU,
            CompressedFormat::RgtcFormatI,
            CompressedFormat::RgtcFormatUU,
            CompressedFormat::RgtcFormatII,
            CompressedFormat::BptcUnorm4,
            CompressedFormat::BptcSignedFloat3,
            CompressedFormat::BptcUnsignedFloat3,
            CompressedFormat::S3tcDxt1NoAlpha,
            CompressedFormat::S3tcDxt1Alpha,
            CompressedFormat::S3tcDxt3Alpha,
            CompressedFormat::S3tcDxt5Alpha,
        ]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::CompressedFormat(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        let version = context.get_version();
        let extensions = context.get_extensions();

        match self {
            CompressedFormat::RgtcFormatU => version.0 == Api::Gl,
            CompressedFormat::RgtcFormatI => version.0 == Api::Gl,
            CompressedFormat::RgtcFormatUU => version.0 == Api::Gl,
            CompressedFormat::RgtcFormatII => version.0 == Api::Gl,
            CompressedFormat::BptcUnorm4 => {
                version >= &Version(Api::Gl, 4, 2) || extensions.gl_arb_texture_compression_bptc
            }
            CompressedFormat::BptcSignedFloat3 => {
                version >= &Version(Api::Gl, 4, 2) || extensions.gl_arb_texture_compression_bptc
            }
            CompressedFormat::BptcUnsignedFloat3 => {
                version >= &Version(Api::Gl, 4, 2) || extensions.gl_arb_texture_compression_bptc
            }
            CompressedFormat::S3tcDxt1NoAlpha => extensions.gl_ext_texture_compression_s3tc,
            CompressedFormat::S3tcDxt1Alpha => extensions.gl_ext_texture_compression_s3tc,
            CompressedFormat::S3tcDxt3Alpha => extensions.gl_ext_texture_compression_s3tc,
            CompressedFormat::S3tcDxt5Alpha => extensions.gl_ext_texture_compression_s3tc,
        }
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            CompressedFormat::RgtcFormatU => gl::COMPRESSED_RED_RGTC1,
            CompressedFormat::RgtcFormatI => gl::COMPRESSED_SIGNED_RED_RGTC1,
            CompressedFormat::RgtcFormatUU => gl::COMPRESSED_RG_RGTC2,
            CompressedFormat::RgtcFormatII => gl::COMPRESSED_SIGNED_RG_RGTC2,
            CompressedFormat::BptcUnorm4 => gl::COMPRESSED_RGBA_BPTC_UNORM,
            CompressedFormat::BptcSignedFloat3 => gl::COMPRESSED_RGB_BPTC_SIGNED_FLOAT,
            CompressedFormat::BptcUnsignedFloat3 => gl::COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT,
            CompressedFormat::S3tcDxt1NoAlpha => gl::COMPRESSED_RGB_S3TC_DXT1_EXT,
            CompressedFormat::S3tcDxt1Alpha => gl::COMPRESSED_RGBA_S3TC_DXT1_EXT,
            CompressedFormat::S3tcDxt3Alpha => gl::COMPRESSED_RGBA_S3TC_DXT3_EXT,
            CompressedFormat::S3tcDxt5Alpha => gl::COMPRESSED_RGBA_S3TC_DXT5_EXT,
        }
    }
}

/// List of compressed pixel formats in the sRGB color space.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum CompressedSrgbFormat {
    /// BPTC format. sRGB with alpha. Also called `BC7` by DirectX.
    Bptc,
    S3tcDxt1NoAlpha,
    S3tcDxt1Alpha,
    S3tcDxt3Alpha,
    S3tcDxt5Alpha,
}

impl CompressedSrgbFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<CompressedSrgbFormat> {
        vec![
            CompressedSrgbFormat::Bptc,
            CompressedSrgbFormat::S3tcDxt1NoAlpha,
            CompressedSrgbFormat::S3tcDxt1Alpha,
            CompressedSrgbFormat::S3tcDxt3Alpha,
            CompressedSrgbFormat::S3tcDxt5Alpha,
        ]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::CompressedSrgbFormat(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        let version = context.get_version();
        let extensions = context.get_extensions();

        match self {
            CompressedSrgbFormat::Bptc => {
                version >= &Version(Api::Gl, 4, 2) || extensions.gl_arb_texture_compression_bptc
            }
            CompressedSrgbFormat::S3tcDxt1NoAlpha => {
                extensions.gl_ext_texture_compression_s3tc && extensions.gl_ext_texture_srgb
            }
            CompressedSrgbFormat::S3tcDxt1Alpha => {
                extensions.gl_ext_texture_compression_s3tc && extensions.gl_ext_texture_srgb
            }
            CompressedSrgbFormat::S3tcDxt3Alpha => {
                extensions.gl_ext_texture_compression_s3tc && extensions.gl_ext_texture_srgb
            }
            CompressedSrgbFormat::S3tcDxt5Alpha => {
                extensions.gl_ext_texture_compression_s3tc && extensions.gl_ext_texture_srgb
            }
        }
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            CompressedSrgbFormat::Bptc => gl::COMPRESSED_SRGB_ALPHA_BPTC_UNORM,
            CompressedSrgbFormat::S3tcDxt1NoAlpha => gl::COMPRESSED_SRGB_S3TC_DXT1_EXT,
            CompressedSrgbFormat::S3tcDxt1Alpha => gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT,
            CompressedSrgbFormat::S3tcDxt3Alpha => gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT,
            CompressedSrgbFormat::S3tcDxt5Alpha => gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT,
        }
    }
}

/// List of formats available for depth textures.
///
/// `I16`, `I24` and `I32` are still treated as if they were floating points.
/// Only the internal representation is integral.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum DepthFormat {
    I16,
    I24,
    /// May not be supported by all hardware.
    I32,
    F32,
}

impl DepthFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<DepthFormat> {
        vec![
            DepthFormat::I16,
            DepthFormat::I24,
            DepthFormat::I32,
            DepthFormat::F32,
        ]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::DepthFormat(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        match self {
            DepthFormat::I16 | DepthFormat::I24 | DepthFormat::F32 => true,
            // OpenGL ES has no 32-bit integer depth format
            DepthFormat::I32 => context.get_version().0 == Api::Gl,
        }
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            DepthFormat::I16 => gl::DEPTH_COMPONENT16,
            DepthFormat::I24 => gl::DEPTH_COMPONENT24,
            DepthFormat::I32 => gl::DEPTH_COMPONENT32,
            DepthFormat::F32 => gl::DEPTH_COMPONENT32F,
        }
    }
}

/// List of formats available for depth-stencil textures.
// TODO: If OpenGL 4.3 or ARB_stencil_texturing is not available, then depth/stencil
//       textures are treated by samplers exactly like depth-only textures
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum DepthStencilFormat {
    I24I8,
    F32I8,
}

impl DepthStencilFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<DepthStencilFormat> {
        vec![DepthStencilFormat::I24I8, DepthStencilFormat::F32I8]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::DepthStencilFormat(self)
    }

    /// Returns true if this format is supported by the backend.
    pub fn is_supported<C: ?Sized>(&self, _context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // both are core in OpenGL 3.0 and OpenGL ES 3.0
        match self {
            DepthStencilFormat::I24I8 | DepthStencilFormat::F32I8 => true,
        }
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            DepthStencilFormat::I24I8 => gl::DEPTH24_STENCIL8,
            DepthStencilFormat::F32I8 => gl::DEPTH32F_STENCIL8,
        }
    }
}

/// List of formats available for stencil textures.
///
/// You are strongly advised to only use `I8`.
///
/// Stencil textures are a very recent OpenGL feature that may not be supported everywhere.
/// Only `I8` is supported for textures. All the other formats can only be used with renderbuffers.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum StencilFormat {
    I1,
    I4,
    I8,
    I16,
}

impl StencilFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<StencilFormat> {
        vec![
            StencilFormat::I1,
            StencilFormat::I4,
            StencilFormat::I8,
            StencilFormat::I16,
        ]
    }

    /// Turns this format into a more generic `TextureFormat`.
    #[inline]
    pub fn to_texture_format(self) -> TextureFormat {
        TextureFormat::StencilFormat(self)
    }

    /// Returns true if this format is supported by the backend for textures.
    pub fn is_supported_for_textures<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        let version = context.get_version();
        let extensions = context.get_extensions();

        match self {
            StencilFormat::I8 => {
                version >= &Version(Api::Gl, 4, 4)
                    || version >= &Version(Api::GlEs, 3, 2)
                    || extensions.gl_arb_texture_stencil8
                    || extensions.gl_oes_texture_stencil8
            }

            _ => false,
        }
    }

    /// Returns true if this format is supported by the backend for renderbuffers.
    pub fn is_supported_for_renderbuffers<C: ?Sized>(&self, context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        // OpenGL ES only has 8-bit stencil buffers
        match self {
            StencilFormat::I8 => true,
            StencilFormat::I1 | StencilFormat::I4 | StencilFormat::I16 => {
                context.get_version().0 == Api::Gl
            }
        }
    }

    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            StencilFormat::I1 => gl::STENCIL_INDEX1,
            StencilFormat::I4 => gl::STENCIL_INDEX4,
            StencilFormat::I8 => gl::STENCIL_INDEX8,
            StencilFormat::I16 => gl::STENCIL_INDEX16,
        }
    }
}

/// Format of the internal representation of a texture.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum TextureFormat {
    UncompressedFloat(UncompressedFloatFormat),
    UncompressedIntegral(UncompressedIntFormat),
    UncompressedUnsigned(UncompressedUintFormat),
    Srgb(SrgbFormat),
    CompressedFormat(CompressedFormat),
    CompressedSrgbFormat(CompressedSrgbFormat),
    DepthFormat(DepthFormat),
    StencilFormat(StencilFormat),
    DepthStencilFormat(DepthStencilFormat),
}

impl TextureFormat {
    /// Returns a list of all the possible values of this enumeration.
    #[inline]
    pub fn get_formats_list() -> Vec<TextureFormat> {
        // Chaining so many iterators can blow up the stack on some platforms
        let mut result: Vec<_> = UncompressedFloatFormat::get_formats_list()
            .into_iter()
            .map(|f| f.to_texture_format())
            .collect();
        result.extend(
            UncompressedIntFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result.extend(
            UncompressedUintFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result.extend(
            SrgbFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result.extend(
            CompressedFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result.extend(
            CompressedSrgbFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result.extend(
            DepthFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result.extend(
            StencilFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result.extend(
            DepthStencilFormat::get_formats_list()
                .into_iter()
                .map(|f| f.to_texture_format()),
        );
        result
    }

    /// Returns true if this format is supported by the backend for textures.
    #[inline]
    pub fn is_supported_for_textures<C: ?Sized>(&self, c: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        match self {
            TextureFormat::UncompressedFloat(format) => format.is_supported(c),
            TextureFormat::UncompressedIntegral(format) => format.is_supported(c),
            TextureFormat::UncompressedUnsigned(format) => format.is_supported(c),
            TextureFormat::Srgb(format) => format.is_supported(c),
            TextureFormat::CompressedFormat(format) => format.is_supported(c),
            TextureFormat::CompressedSrgbFormat(format) => format.is_supported(c),
            TextureFormat::DepthFormat(format) => format.is_supported(c),
            TextureFormat::StencilFormat(format) => format.is_supported_for_textures(c),
            TextureFormat::DepthStencilFormat(format) => format.is_supported(c),
        }
    }

    /// Returns true if this format is supported by the backend for renderbuffers.
    #[inline]
    pub fn is_supported_for_renderbuffers<C: ?Sized>(&self, c: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        match self {
            TextureFormat::UncompressedFloat(format) => format.is_supported(c),
            TextureFormat::UncompressedIntegral(format) => format.is_supported(c),
            TextureFormat::UncompressedUnsigned(format) => format.is_supported(c),
            TextureFormat::Srgb(format) => format.is_supported(c),
            TextureFormat::CompressedFormat(format) => format.is_supported(c),
            TextureFormat::CompressedSrgbFormat(format) => format.is_supported(c),
            TextureFormat::DepthFormat(format) => format.is_supported(c),
            TextureFormat::StencilFormat(format) => format.is_supported_for_renderbuffers(c),
            TextureFormat::DepthStencilFormat(format) => format.is_supported(c),
        }
    }

    /// Returns true if the format is color-renderable, depth-renderable, depth-stencil-renderable
    /// or stencil-renderable.
    #[inline]
    pub fn is_renderable<C: ?Sized>(&self, c: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        match self {
            TextureFormat::UncompressedFloat(format) => format.is_color_renderable(c),
            TextureFormat::UncompressedIntegral(format) => format.is_color_renderable(c),
            TextureFormat::UncompressedUnsigned(format) => format.is_color_renderable(c),
            TextureFormat::Srgb(format) => format.is_color_renderable(c),
            TextureFormat::CompressedFormat(_) => false,
            TextureFormat::CompressedSrgbFormat(_) => false,
            TextureFormat::DepthFormat(_) => true,
            TextureFormat::StencilFormat(_) => true,
            TextureFormat::DepthStencilFormat(_) => true,
        }
    }
}

impl ToGlEnum for TextureFormat {
    fn to_glenum(&self) -> gl::types::GLenum {
        match self {
            TextureFormat::UncompressedFloat(f) => f.to_glenum(),
            TextureFormat::UncompressedIntegral(f) => f.to_glenum(),
            TextureFormat::UncompressedUnsigned(f) => f.to_glenum(),
            TextureFormat::Srgb(f) => f.to_glenum(),
            TextureFormat::CompressedFormat(f) => f.to_glenum(),
            TextureFormat::CompressedSrgbFormat(f) => f.to_glenum(),
            TextureFormat::DepthFormat(f) => f.to_glenum(),
            TextureFormat::StencilFormat(f) => f.to_glenum(),
            TextureFormat::DepthStencilFormat(f) => f.to_glenum(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientFormatAny {
    ClientFormat(ClientFormat),
    CompressedFormat(CompressedFormat),
    CompressedSrgbFormat(CompressedSrgbFormat),
}

impl ClientFormatAny {
    /// Checks if this format is a compressed format.
    #[inline]
    pub fn is_compressed(&self) -> bool {
        match *self {
            ClientFormatAny::ClientFormat(_) => false,
            ClientFormatAny::CompressedFormat(_) => true,
            ClientFormatAny::CompressedSrgbFormat(_) => true,
        }
    }

    /// Gets the size in bytes of the buffer required to store a uncompressed image
    /// of the specified dimensions on this format.
    ///
    /// ## Panic
    ///
    /// Panics if the dimensions are invalid for this format.
    /// Returns the size in bytes of an image of these dimensions in this format, or `None` if
    /// the format can't have them: compressed formats are made of 4x4 blocks of two-dimensional
    /// images.
    pub fn get_buffer_size(
        &self,
        width: u32,
        height: Option<u32>,
        depth: Option<u32>,
        array_size: Option<u32>,
    ) -> Option<usize> {
        let block_bytes = match *self {
            ClientFormatAny::ClientFormat(ref format) => {
                return Some(
                    format.get_size()
                        * width as usize
                        * height.unwrap_or(1) as usize
                        * depth.unwrap_or(1) as usize
                        * array_size.unwrap_or(1) as usize,
                );
            }

            // 8 bytes per 4x4 block
            ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt1Alpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt1Alpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt1NoAlpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt1NoAlpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatU)
            | ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatI) => 8,

            // 16 bytes per 4x4 block
            ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt3Alpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt3Alpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt5Alpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt5Alpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::BptcUnorm4)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::Bptc)
            | ClientFormatAny::CompressedFormat(CompressedFormat::BptcSignedFloat3)
            | ClientFormatAny::CompressedFormat(CompressedFormat::BptcUnsignedFloat3)
            | ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatUU)
            | ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatII) => 16,
        };

        // the S3TC, RGTC and BPTC formats only exist for two-dimensional images and their arrays;
        // the blocks at the right and top edges of an image whose size isn't a multiple of 4 are
        // partial, but take the room of a whole block
        let height = height?;
        if depth.is_some() {
            return None;
        }
        let blocks = width.div_ceil(4) as usize
            * height.div_ceil(4) as usize
            * array_size.unwrap_or(1) as usize;
        Some(blocks * block_bytes)
    }

    /// Returns the number of components of the format.
    #[inline]
    pub fn get_num_components(&self) -> u8 {
        match *self {
            ClientFormatAny::ClientFormat(ref format) => format.get_num_components(),
            ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatU)
            | ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatI) => 1,
            ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatUU)
            | ClientFormatAny::CompressedFormat(CompressedFormat::RgtcFormatII) => 2,
            ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt1NoAlpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt1NoAlpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::BptcSignedFloat3)
            | ClientFormatAny::CompressedFormat(CompressedFormat::BptcUnsignedFloat3) => 3,
            ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt1Alpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt1Alpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt3Alpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt3Alpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::S3tcDxt5Alpha)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::S3tcDxt5Alpha)
            | ClientFormatAny::CompressedFormat(CompressedFormat::BptcUnorm4)
            | ClientFormatAny::CompressedSrgbFormat(CompressedSrgbFormat::Bptc) => 4,
        }
    }

    #[doc(hidden)]
    pub fn from_internal_compressed_format(internal: gl::types::GLenum) -> Option<ClientFormatAny> {
        match internal {
            gl::COMPRESSED_RGB_S3TC_DXT1_EXT => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::S3tcDxt1NoAlpha,
            )),
            gl::COMPRESSED_RGBA_S3TC_DXT1_EXT => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::S3tcDxt1Alpha,
            )),
            gl::COMPRESSED_RGBA_S3TC_DXT3_EXT => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::S3tcDxt3Alpha,
            )),
            gl::COMPRESSED_RGBA_S3TC_DXT5_EXT => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::S3tcDxt5Alpha,
            )),
            gl::COMPRESSED_SRGB_S3TC_DXT1_EXT => Some(ClientFormatAny::CompressedSrgbFormat(
                CompressedSrgbFormat::S3tcDxt1NoAlpha,
            )),
            gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT => Some(ClientFormatAny::CompressedSrgbFormat(
                CompressedSrgbFormat::S3tcDxt1Alpha,
            )),
            gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT => Some(ClientFormatAny::CompressedSrgbFormat(
                CompressedSrgbFormat::S3tcDxt3Alpha,
            )),
            gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT => Some(ClientFormatAny::CompressedSrgbFormat(
                CompressedSrgbFormat::S3tcDxt5Alpha,
            )),
            gl::COMPRESSED_RGBA_BPTC_UNORM => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::BptcUnorm4,
            )),
            gl::COMPRESSED_SRGB_ALPHA_BPTC_UNORM => Some(ClientFormatAny::CompressedSrgbFormat(
                CompressedSrgbFormat::Bptc,
            )),
            gl::COMPRESSED_RGB_BPTC_SIGNED_FLOAT => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::BptcSignedFloat3,
            )),
            gl::COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::BptcUnsignedFloat3,
            )),
            gl::COMPRESSED_RED_RGTC1 => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::RgtcFormatU,
            )),
            gl::COMPRESSED_SIGNED_RED_RGTC1 => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::RgtcFormatI,
            )),
            gl::COMPRESSED_RG_RGTC2 => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::RgtcFormatUU,
            )),
            gl::COMPRESSED_SIGNED_RG_RGTC2 => Some(ClientFormatAny::CompressedFormat(
                CompressedFormat::RgtcFormatII,
            )),
            _ => None,
        }
    }
}

/// Type of request.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RequestType {
    /// A format suitable for `glTexImage#D`.
    TexImage(Option<ClientFormatAny>),
    /// A format suitable for `glTexStorage#D`, with the client format of the data that will be
    /// put in the texture if it must match.
    TexStorage(Option<ClientFormatAny>),
    /// A format suitable for `glRenderbufferStorage`.
    Renderbuffer,
}

impl RequestType {
    /// Returns the client format of the data that will be put in the texture.
    #[inline]
    pub fn get_client_format(&self) -> Option<ClientFormatAny> {
        match self {
            RequestType::TexImage(f) => *f,
            RequestType::TexStorage(f) => *f,
            RequestType::Renderbuffer => None,
        }
    }
}

/// Checks that the texture format is supported and compatible with the client format.
///
/// Returns two `GLenum`s. The first one can be unsized and is suitable for the internal format
/// of `glTexImage#D`. The second one is always sized and is suitable for `glTexStorage*D` or
/// `glRenderbufferStorage`.
pub fn format_request_to_glenum(
    context: &Context,
    format: TextureFormatRequest,
    rq_ty: RequestType,
) -> Result<gl::types::GLenum, FormatNotSupportedError> {
    let version = context.get_opengl_version();
    let extensions = context.get_extensions();

    let is_client_compressed = match rq_ty.get_client_format() {
        Some(ref client) => client.is_compressed(),
        None => false,
    };

    Ok(match format {
        /*******************************************************************/
        /*                           REGULAR                               */
        /*******************************************************************/
        TextureFormatRequest::AnyFloatingPoint => {
            let size = rq_ty.get_client_format().map(|c| c.get_num_components());

            match (rq_ty, size) {
                (RequestType::TexImage(_), Some(1)) => gl::RED,
                (RequestType::TexImage(_), Some(2)) => gl::RG,
                (RequestType::TexImage(_), Some(3)) => gl::RGB,
                (RequestType::TexImage(_), Some(4)) => gl::RGBA,
                (RequestType::TexImage(_), None) => gl::RGBA,
                (_, Some(1)) => gl::R8,
                (_, Some(2)) => gl::RG8,
                (_, Some(3)) => gl::RGB8,
                (_, Some(4)) => gl::RGBA8,
                (_, None) => gl::RGBA8,
                _ => unreachable!(),
            }
        }

        TextureFormatRequest::Specific(TextureFormat::UncompressedFloat(format)) => {
            if format.is_supported(context)
                && (!matches!(rq_ty, RequestType::Renderbuffer)
                    || format.is_color_renderable(context))
            {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        /*******************************************************************/
        /*                         COMPRESSED                              */
        /*******************************************************************/
        TextureFormatRequest::AnyCompressed if is_client_compressed => {
            // Note: client is always Some here. When refactoring this function it'd be a good idea
            // to let the client participate on the matching process.
            let newformat = TextureFormat::CompressedFormat(match rq_ty.get_client_format() {
                Some(ClientFormatAny::CompressedFormat(format)) => format,
                _ => unreachable!(),
            });
            format_request_to_glenum(context, TextureFormatRequest::Specific(newformat), rq_ty)?
        }

        // OpenGL ES has no generic compressed formats, so the texture isn't compressed there
        TextureFormatRequest::AnyCompressed if version.0 == Api::GlEs => {
            format_request_to_glenum(context, TextureFormatRequest::AnyFloatingPoint, rq_ty)?
        }

        TextureFormatRequest::AnyCompressed => {
            match rq_ty {
                RequestType::TexImage(client) => {
                    let size = client.map(|c| c.get_num_components());

                    match size {
                        Some(1) => gl::COMPRESSED_RED,
                        Some(2) => gl::COMPRESSED_RG,
                        Some(3) => gl::COMPRESSED_RGB,
                        Some(4) => gl::COMPRESSED_RGBA,
                        None => gl::COMPRESSED_RGBA,
                        _ => unreachable!(),
                    }
                }
                RequestType::TexStorage(_) | RequestType::Renderbuffer => {
                    return Err(FormatNotSupportedError)
                }
            }
        }

        TextureFormatRequest::Specific(TextureFormat::CompressedFormat(format)) => {
            if format.is_supported(context) {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        /*******************************************************************/
        /*                             SRGB                                */
        /*******************************************************************/
        TextureFormatRequest::AnySrgb => {
            let size = rq_ty.get_client_format().map(|c| c.get_num_components());

            match size {
                Some(1..=3) => gl::SRGB8,
                Some(4) => gl::SRGB8_ALPHA8,
                None => {
                    if let RequestType::TexImage(_) = rq_ty {
                        gl::SRGB8
                    } else {
                        gl::SRGB8_ALPHA8
                    }
                }
                _ => unreachable!(),
            }
        }

        TextureFormatRequest::Specific(TextureFormat::Srgb(format)) => {
            if format.is_supported(context)
                && (!matches!(rq_ty, RequestType::Renderbuffer)
                    || format.is_color_renderable(context))
            {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        /*******************************************************************/
        /*                        COMPRESSED SRGB                          */
        /*******************************************************************/
        TextureFormatRequest::AnyCompressedSrgb if is_client_compressed => {
            let newformat = TextureFormat::CompressedSrgbFormat(match rq_ty.get_client_format() {
                Some(ClientFormatAny::CompressedSrgbFormat(format)) => format,
                _ => unreachable!(),
            });
            format_request_to_glenum(context, TextureFormatRequest::Specific(newformat), rq_ty)?
        }

        TextureFormatRequest::AnyCompressedSrgb => {
            // OpenGL ES has no generic compressed formats, so the texture isn't compressed there
            if version.0 == Api::Gl {
                match rq_ty {
                    RequestType::TexImage(client) => match client.map(|c| c.get_num_components()) {
                        Some(1..=3) => gl::COMPRESSED_SRGB,
                        Some(4) => gl::COMPRESSED_SRGB_ALPHA,
                        None => gl::COMPRESSED_SRGB_ALPHA,
                        _ => unreachable!(),
                    },
                    RequestType::TexStorage(_) | RequestType::Renderbuffer => {
                        return Err(FormatNotSupportedError)
                    }
                }
            } else {
                format_request_to_glenum(context, TextureFormatRequest::AnySrgb, rq_ty)?
            }
        }

        TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(format)) => {
            if format.is_supported(context) {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        /*******************************************************************/
        /*                          INTEGRAL                               */
        /*******************************************************************/
        TextureFormatRequest::AnyIntegral => {
            let size = rq_ty.get_client_format().map(|c| c.get_num_components());

            // OpenGL ES only uploads data whose type matches the format, so the number of bits
            // comes from the data there; desktop OpenGL always gets 32 bits
            let bits = match (version.0, rq_ty.get_client_format()) {
                (Api::GlEs, Some(ClientFormatAny::ClientFormat(client))) => {
                    8 * client.get_size() / client.get_num_components() as usize
                }
                _ => 32,
            };

            match (size, bits) {
                (Some(1), 8) => gl::R8I,
                (Some(1), 16) => gl::R16I,
                (Some(1), _) => gl::R32I,
                (Some(2), 8) => gl::RG8I,
                (Some(2), 16) => gl::RG16I,
                (Some(2), _) => gl::RG32I,
                (Some(3), 8) => gl::RGB8I,
                (Some(3), 16) => gl::RGB16I,
                (Some(3), _) => gl::RGB32I,
                (Some(4), 8) | (None, 8) => gl::RGBA8I,
                (Some(4), 16) | (None, 16) => gl::RGBA16I,
                (Some(4), _) | (None, _) => gl::RGBA32I,
                _ => unreachable!(),
            }
        }

        TextureFormatRequest::Specific(TextureFormat::UncompressedIntegral(format)) => {
            if format.is_supported(context)
                && (!matches!(rq_ty, RequestType::Renderbuffer)
                    || format.is_color_renderable(context))
            {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        /*******************************************************************/
        /*                          UNSIGNED                               */
        /*******************************************************************/
        TextureFormatRequest::AnyUnsigned => {
            let size = rq_ty.get_client_format().map(|c| c.get_num_components());

            // OpenGL ES only uploads data whose type matches the format, so the number of bits
            // comes from the data there; desktop OpenGL always gets 32 bits
            let bits = match (version.0, rq_ty.get_client_format()) {
                (Api::GlEs, Some(ClientFormatAny::ClientFormat(client))) => {
                    8 * client.get_size() / client.get_num_components() as usize
                }
                _ => 32,
            };

            match (size, bits) {
                (Some(1), 8) => gl::R8UI,
                (Some(1), 16) => gl::R16UI,
                (Some(1), _) => gl::R32UI,
                (Some(2), 8) => gl::RG8UI,
                (Some(2), 16) => gl::RG16UI,
                (Some(2), _) => gl::RG32UI,
                (Some(3), 8) => gl::RGB8UI,
                (Some(3), 16) => gl::RGB16UI,
                (Some(3), _) => gl::RGB32UI,
                (Some(4), 8) | (None, 8) => gl::RGBA8UI,
                (Some(4), 16) | (None, 16) => gl::RGBA16UI,
                (Some(4), _) | (None, _) => gl::RGBA32UI,
                _ => unreachable!(),
            }
        }

        TextureFormatRequest::Specific(TextureFormat::UncompressedUnsigned(format)) => {
            if format.is_supported(context)
                && (!matches!(rq_ty, RequestType::Renderbuffer)
                    || format.is_color_renderable(context))
            {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        /*******************************************************************/
        /*                            DEPTH                                */
        /*******************************************************************/
        TextureFormatRequest::AnyDepth => {
            match rq_ty {
                RequestType::TexImage(_) => gl::DEPTH_COMPONENT,
                RequestType::TexStorage(_) | RequestType::Renderbuffer => gl::DEPTH_COMPONENT24,
            }
        }

        TextureFormatRequest::Specific(TextureFormat::DepthFormat(format)) => {
            if format.is_supported(context) {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        /*******************************************************************/
        /*                           STENCIL                               */
        /*******************************************************************/
        TextureFormatRequest::AnyStencil => {
            // TODO: we just request I8, but this could be more flexible
            return format_request_to_glenum(
                context,
                TextureFormatRequest::Specific(TextureFormat::StencilFormat(StencilFormat::I8)),
                rq_ty,
            );
        }

        TextureFormatRequest::Specific(TextureFormat::StencilFormat(format)) => match rq_ty {
            RequestType::TexImage(_) | RequestType::TexStorage(_) => {
                if format.is_supported_for_textures(context) {
                    format.to_glenum()
                } else {
                    return Err(FormatNotSupportedError);
                }
            }
            RequestType::Renderbuffer => {
                if format.is_supported_for_renderbuffers(context) {
                    format.to_glenum()
                } else {
                    return Err(FormatNotSupportedError);
                }
            }
        },

        /*******************************************************************/
        /*                        DEPTH-STENCIL                            */
        /*******************************************************************/
        TextureFormatRequest::AnyDepthStencil => {
            match rq_ty {
                RequestType::TexImage(_) => gl::DEPTH_STENCIL,
                RequestType::TexStorage(_) | RequestType::Renderbuffer => gl::DEPTH24_STENCIL8,
            }
        }

        TextureFormatRequest::Specific(TextureFormat::DepthStencilFormat(format)) => {
            if format.is_supported(context) {
                format.to_glenum()
            } else {
                return Err(FormatNotSupportedError);
            }
        }
    })
}

/// Returns the format and type to pass to `glTexImage#D` when a texture is created without
/// data.
///
/// No pixels are transferred, but OpenGL ES (and desktop OpenGL for integer, depth and stencil
/// textures) still requires a format and type that are compatible with the internal format.
pub fn default_client_format_to_glenum(
    format: TextureFormatRequest,
) -> (gl::types::GLenum, gl::types::GLenum) {
    use self::UncompressedFloatFormat as F;
    use self::UncompressedIntFormat as I;
    use self::UncompressedUintFormat as U;

    let color = |components: u8| match components {
        1 => gl::RED,
        2 => gl::RG,
        3 => gl::RGB,
        _ => gl::RGBA,
    };
    let integer = |components: u8| match components {
        1 => gl::RED_INTEGER,
        2 => gl::RG_INTEGER,
        3 => gl::RGB_INTEGER,
        _ => gl::RGBA_INTEGER,
    };

    match format {
        TextureFormatRequest::Specific(TextureFormat::UncompressedFloat(format)) => {
            let (components, ty) = match format {
                F::U8 | F::U16 => (1, gl::UNSIGNED_BYTE),
                F::I8 | F::I16 => (1, gl::BYTE),
                F::F16 | F::F32 => (1, gl::FLOAT),
                F::U8U8 | F::U16U16 => (2, gl::UNSIGNED_BYTE),
                F::I8I8 | F::I16I16 => (2, gl::BYTE),
                F::F16F16 | F::F32F32 => (2, gl::FLOAT),
                F::U3U3U2
                | F::U4U4U4
                | F::U5U5U5
                | F::U5U6U5
                | F::U8U8U8
                | F::U10U10U10
                | F::U12U12U12
                | F::U16U16U16 => (3, gl::UNSIGNED_BYTE),
                F::I8I8I8 | F::I16I16I16 => (3, gl::BYTE),
                F::F16F16F16 | F::F32F32F32 | F::F11F11F10 | F::F9F9F9 => (3, gl::FLOAT),
                F::I8I8I8I8 | F::I16I16I16I16 => (4, gl::BYTE),
                F::F16F16F16F16 | F::F32F32F32F32 => (4, gl::FLOAT),
                _ => (4, gl::UNSIGNED_BYTE),
            };
            (color(components), ty)
        }
        TextureFormatRequest::Specific(TextureFormat::UncompressedIntegral(format)) => {
            let components = match format {
                I::I8 | I::I16 | I::I32 => 1,
                I::I8I8 | I::I16I16 | I::I32I32 => 2,
                I::I8I8I8 | I::I16I16I16 | I::I32I32I32 => 3,
                I::I8I8I8I8 | I::I16I16I16I16 | I::I32I32I32I32 => 4,
            };
            (integer(components), gl::INT)
        }
        TextureFormatRequest::Specific(TextureFormat::UncompressedUnsigned(format)) => {
            let components = match format {
                U::U8 | U::U16 | U::U32 => 1,
                U::U8U8 | U::U16U16 | U::U32U32 => 2,
                U::U8U8U8 | U::U16U16U16 | U::U32U32U32 => 3,
                U::U8U8U8U8 | U::U16U16U16U16 | U::U32U32U32U32 | U::U10U10U10U2 => 4,
            };
            (integer(components), gl::UNSIGNED_INT)
        }
        TextureFormatRequest::Specific(TextureFormat::Srgb(SrgbFormat::U8U8U8)) => {
            (gl::RGB, gl::UNSIGNED_BYTE)
        }
        TextureFormatRequest::AnyIntegral => (gl::RGBA_INTEGER, gl::INT),
        TextureFormatRequest::AnyUnsigned => (gl::RGBA_INTEGER, gl::UNSIGNED_INT),
        TextureFormatRequest::AnyDepth
        | TextureFormatRequest::Specific(TextureFormat::DepthFormat(_)) => {
            (gl::DEPTH_COMPONENT, gl::FLOAT)
        }
        TextureFormatRequest::AnyDepthStencil
        | TextureFormatRequest::Specific(TextureFormat::DepthStencilFormat(
            DepthStencilFormat::I24I8,
        )) => (gl::DEPTH_STENCIL, gl::UNSIGNED_INT_24_8),
        TextureFormatRequest::Specific(TextureFormat::DepthStencilFormat(
            DepthStencilFormat::F32I8,
        )) => (gl::DEPTH_STENCIL, gl::FLOAT_32_UNSIGNED_INT_24_8_REV),
        TextureFormatRequest::AnyStencil
        | TextureFormatRequest::Specific(TextureFormat::StencilFormat(_)) => {
            (gl::STENCIL_INDEX, gl::UNSIGNED_BYTE)
        }
        _ => (gl::RGBA, gl::UNSIGNED_BYTE),
    }
}

/// Checks that the client texture format is supported.
///
/// If `inverted` is true, returns a format where the R, G and B components are flipped. OpenGL
/// ES has no such format, so this returns an error there.
///
/// Returns two GLenums suitable for `glTexImage#D` and `glTexSubImage#D`.
pub fn client_format_to_glenum(
    context: &Context,
    client: ClientFormatAny,
    format: TextureFormatRequest,
    inverted: bool,
) -> Result<(gl::types::GLenum, gl::types::GLenum), FormatNotSupportedError> {
    let value = match format {
        TextureFormatRequest::AnyCompressed if client.is_compressed() => match client {
            ClientFormatAny::CompressedFormat(client_format) => {
                if client_format.is_supported(context) {
                    let e = client_format.to_glenum();
                    Ok((e, e))
                } else {
                    return Err(FormatNotSupportedError);
                }
            }
            _ => unreachable!(),
        },

        TextureFormatRequest::AnyCompressedSrgb if client.is_compressed() => match client {
            ClientFormatAny::CompressedSrgbFormat(client_format) => {
                if client_format.is_supported(context) {
                    let e = client_format.to_glenum();
                    Ok((e, e))
                } else {
                    return Err(FormatNotSupportedError);
                }
            }
            _ => unreachable!(),
        },

        TextureFormatRequest::Specific(TextureFormat::CompressedFormat(format))
            if client.is_compressed() =>
        {
            if format.is_supported(context) {
                let e = format.to_glenum();
                Ok((e, e))
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(format))
            if client.is_compressed() =>
        {
            if format.is_supported(context) {
                let e = format.to_glenum();
                Ok((e, e))
            } else {
                return Err(FormatNotSupportedError);
            }
        }

        TextureFormatRequest::AnyFloatingPoint
        | TextureFormatRequest::AnyCompressed
        | TextureFormatRequest::AnySrgb
        | TextureFormatRequest::AnyCompressedSrgb
        | TextureFormatRequest::Specific(TextureFormat::UncompressedFloat(_))
        | TextureFormatRequest::Specific(TextureFormat::Srgb(_))
        | TextureFormatRequest::Specific(TextureFormat::CompressedFormat(_))
        | TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(_)) => {
            match client {
                ClientFormatAny::ClientFormat(ClientFormat::U8) => Ok((gl::RED, gl::UNSIGNED_BYTE)),
                ClientFormatAny::ClientFormat(ClientFormat::U8U8) => {
                    Ok((gl::RG, gl::UNSIGNED_BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U8U8U8) => {
                    Ok((gl::RGB, gl::UNSIGNED_BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U8U8U8U8) => {
                    Ok((gl::RGBA, gl::UNSIGNED_BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I8) => Ok((gl::RED, gl::BYTE)),
                ClientFormatAny::ClientFormat(ClientFormat::I8I8) => Ok((gl::RG, gl::BYTE)),
                ClientFormatAny::ClientFormat(ClientFormat::I8I8I8) => Ok((gl::RGB, gl::BYTE)),
                ClientFormatAny::ClientFormat(ClientFormat::I8I8I8I8) => Ok((gl::RGBA, gl::BYTE)),
                ClientFormatAny::ClientFormat(ClientFormat::U16) => {
                    Ok((gl::RED, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U16U16) => {
                    Ok((gl::RG, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U16U16U16) => {
                    Ok((gl::RGB, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U16U16U16U16) => {
                    Ok((gl::RGBA, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I16) => Ok((gl::RED, gl::SHORT)),
                ClientFormatAny::ClientFormat(ClientFormat::I16I16) => Ok((gl::RG, gl::SHORT)),
                ClientFormatAny::ClientFormat(ClientFormat::I16I16I16) => Ok((gl::RGB, gl::SHORT)),
                ClientFormatAny::ClientFormat(ClientFormat::I16I16I16I16) => {
                    Ok((gl::RGBA, gl::SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U32) => Ok((gl::RED, gl::UNSIGNED_INT)),
                ClientFormatAny::ClientFormat(ClientFormat::U32U32) => {
                    Ok((gl::RG, gl::UNSIGNED_INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U32U32U32) => {
                    Ok((gl::RGB, gl::UNSIGNED_INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U32U32U32U32) => {
                    Ok((gl::RGBA, gl::UNSIGNED_INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I32) => Ok((gl::RED, gl::INT)),
                ClientFormatAny::ClientFormat(ClientFormat::I32I32) => Ok((gl::RG, gl::INT)),
                ClientFormatAny::ClientFormat(ClientFormat::I32I32I32) => Ok((gl::RGB, gl::INT)),
                ClientFormatAny::ClientFormat(ClientFormat::I32I32I32I32) => {
                    Ok((gl::RGBA, gl::INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U3U3U2) => {
                    Ok((gl::RGB, gl::UNSIGNED_BYTE_3_3_2))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U5U6U5) => {
                    Ok((gl::RGB, gl::UNSIGNED_SHORT_5_6_5))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U4U4U4U4) => {
                    Ok((gl::RGBA, gl::UNSIGNED_SHORT_4_4_4_4))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U5U5U5U1) => {
                    Ok((gl::RGBA, gl::UNSIGNED_SHORT_5_5_5_1))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U1U5U5U5Reversed) => {
                    Ok((gl::RGBA, gl::UNSIGNED_SHORT_1_5_5_5_REV))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U10U10U10U2) => {
                    Ok((gl::RGBA, gl::UNSIGNED_INT_10_10_10_2))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F16) => Ok((gl::RED, gl::HALF_FLOAT)),
                ClientFormatAny::ClientFormat(ClientFormat::F16F16) => Ok((gl::RG, gl::HALF_FLOAT)),
                ClientFormatAny::ClientFormat(ClientFormat::F16F16F16) => {
                    Ok((gl::RGB, gl::HALF_FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F16F16F16F16) => {
                    Ok((gl::RGBA, gl::HALF_FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F32) => Ok((gl::RED, gl::FLOAT)),
                ClientFormatAny::ClientFormat(ClientFormat::F32F32) => Ok((gl::RG, gl::FLOAT)),
                ClientFormatAny::ClientFormat(ClientFormat::F32F32F32) => Ok((gl::RGB, gl::FLOAT)),
                ClientFormatAny::ClientFormat(ClientFormat::F32F32F32F32) => {
                    Ok((gl::RGBA, gl::FLOAT))
                }

                // this kind of situation shouldn't happen, it should have a special handling when
                // client is compressed.
                ClientFormatAny::CompressedFormat(_) => unreachable!(),
                ClientFormatAny::CompressedSrgbFormat(_) => unreachable!(),
            }
        }

        TextureFormatRequest::AnyIntegral
        | TextureFormatRequest::AnyUnsigned
        | TextureFormatRequest::Specific(TextureFormat::UncompressedIntegral(_))
        | TextureFormatRequest::Specific(TextureFormat::UncompressedUnsigned(_)) => {
            match client {
                ClientFormatAny::ClientFormat(ClientFormat::U8) => {
                    Ok((gl::RED_INTEGER, gl::UNSIGNED_BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U8U8) => {
                    Ok((gl::RG_INTEGER, gl::UNSIGNED_BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U8U8U8) => {
                    Ok((gl::RGB_INTEGER, gl::UNSIGNED_BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U8U8U8U8) => {
                    Ok((gl::RGBA_INTEGER, gl::UNSIGNED_BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I8) => Ok((gl::RED_INTEGER, gl::BYTE)),
                ClientFormatAny::ClientFormat(ClientFormat::I8I8) => Ok((gl::RG_INTEGER, gl::BYTE)),
                ClientFormatAny::ClientFormat(ClientFormat::I8I8I8) => {
                    Ok((gl::RGB_INTEGER, gl::BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I8I8I8I8) => {
                    Ok((gl::RGBA_INTEGER, gl::BYTE))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U16) => {
                    Ok((gl::RED_INTEGER, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U16U16) => {
                    Ok((gl::RG_INTEGER, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U16U16U16) => {
                    Ok((gl::RGB_INTEGER, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U16U16U16U16) => {
                    Ok((gl::RGBA_INTEGER, gl::UNSIGNED_SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I16) => {
                    Ok((gl::RED_INTEGER, gl::SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I16I16) => {
                    Ok((gl::RG_INTEGER, gl::SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I16I16I16) => {
                    Ok((gl::RGB_INTEGER, gl::SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I16I16I16I16) => {
                    Ok((gl::RGBA_INTEGER, gl::SHORT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U32) => {
                    Ok((gl::RED_INTEGER, gl::UNSIGNED_INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U32U32) => {
                    Ok((gl::RG_INTEGER, gl::UNSIGNED_INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U32U32U32) => {
                    Ok((gl::RGB_INTEGER, gl::UNSIGNED_INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U32U32U32U32) => {
                    Ok((gl::RGBA_INTEGER, gl::UNSIGNED_INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I32) => Ok((gl::RED_INTEGER, gl::INT)),
                ClientFormatAny::ClientFormat(ClientFormat::I32I32) => {
                    Ok((gl::RG_INTEGER, gl::INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I32I32I32) => {
                    Ok((gl::RGB_INTEGER, gl::INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::I32I32I32I32) => {
                    Ok((gl::RGBA_INTEGER, gl::INT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U3U3U2) => {
                    Ok((gl::RGB_INTEGER, gl::UNSIGNED_BYTE_3_3_2))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U5U6U5) => {
                    Ok((gl::RGB_INTEGER, gl::UNSIGNED_SHORT_5_6_5))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U4U4U4U4) => {
                    Ok((gl::RGBA_INTEGER, gl::UNSIGNED_SHORT_4_4_4_4))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U5U5U5U1) => {
                    Ok((gl::RGBA_INTEGER, gl::UNSIGNED_SHORT_5_5_5_1))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U1U5U5U5Reversed) => {
                    Ok((gl::RGBA_INTEGER, gl::UNSIGNED_SHORT_1_5_5_5_REV))
                }
                ClientFormatAny::ClientFormat(ClientFormat::U10U10U10U2) => {
                    Ok((gl::RGBA_INTEGER, gl::UNSIGNED_INT_10_10_10_2))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F16) => {
                    Ok((gl::RED_INTEGER, gl::HALF_FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F16F16) => {
                    Ok((gl::RG_INTEGER, gl::HALF_FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F16F16F16) => {
                    Ok((gl::RGB_INTEGER, gl::HALF_FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F16F16F16F16) => {
                    Ok((gl::RGBA_INTEGER, gl::HALF_FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F32) => {
                    Ok((gl::RED_INTEGER, gl::FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F32F32) => {
                    Ok((gl::RG_INTEGER, gl::FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F32F32F32) => {
                    Ok((gl::RGB_INTEGER, gl::FLOAT))
                }
                ClientFormatAny::ClientFormat(ClientFormat::F32F32F32F32) => {
                    Ok((gl::RGBA_INTEGER, gl::FLOAT))
                }

                // this kind of situation shouldn't happen, it should have a special handling when
                // client is compressed.
                ClientFormatAny::CompressedFormat(_) => unreachable!(),
                ClientFormatAny::CompressedSrgbFormat(_) => unreachable!(),
            }
        }

        TextureFormatRequest::AnyDepth
        | TextureFormatRequest::Specific(TextureFormat::DepthFormat(_)) => {
            if client != ClientFormatAny::ClientFormat(ClientFormat::F32) {
                panic!("Only ClientFormatAny::ClientFormat(ClientFormat::F32) can be used to upload on a depth texture");
            }

            Ok((gl::DEPTH_COMPONENT, gl::FLOAT))
        }

        TextureFormatRequest::AnyStencil
        | TextureFormatRequest::Specific(TextureFormat::StencilFormat(_)) => match client {
            ClientFormatAny::ClientFormat(ClientFormat::U8) => {
                Ok((gl::RED_INTEGER, gl::UNSIGNED_BYTE))
            }
            ClientFormatAny::ClientFormat(ClientFormat::I8) => Ok((gl::RED_INTEGER, gl::BYTE)),
            ClientFormatAny::ClientFormat(ClientFormat::U16) => {
                Ok((gl::RED_INTEGER, gl::UNSIGNED_SHORT))
            }
            ClientFormatAny::ClientFormat(ClientFormat::I16) => Ok((gl::RED_INTEGER, gl::SHORT)),
            ClientFormatAny::ClientFormat(ClientFormat::U32) => {
                Ok((gl::RED_INTEGER, gl::UNSIGNED_INT))
            }
            ClientFormatAny::ClientFormat(ClientFormat::I32) => Ok((gl::RED_INTEGER, gl::INT)),
            ClientFormatAny::ClientFormat(ClientFormat::F16) => {
                Ok((gl::RED_INTEGER, gl::HALF_FLOAT))
            }
            ClientFormatAny::ClientFormat(ClientFormat::F32) => Ok((gl::RED_INTEGER, gl::FLOAT)),
            _ => panic!("Can't upload to a stencil texture with more than one channel"),
        },

        TextureFormatRequest::AnyDepthStencil
        | TextureFormatRequest::Specific(TextureFormat::DepthStencilFormat(_)) => {
            unimplemented!();
        }
    };

    if inverted && context.get_opengl_version().0 != Api::Gl {
        Err(FormatNotSupportedError)
    } else if inverted {
        value.and_then(|(format, ty)| {
            let format = match format {
                gl::RGB => gl::BGR,
                gl::RGBA => gl::BGRA,
                f => return Err(FormatNotSupportedError),
            };

            Ok((format, ty))
        })
    } else {
        value
    }
}
