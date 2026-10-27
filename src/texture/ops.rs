//! Operations on whole images of textures: copying (OpenGL 4.3, OpenGL ES 3.2), clearing (OpenGL
//! 4.4) and the texture barrier (OpenGL 4.5), compiled from these levels.

use std::error::Error;
use std::fmt;
#[cfg(safegl_gl44)]
use std::os::raw::c_void;
use std::rc::Rc;

use glow::HasContext;

#[cfg(safegl_gl45)]
use crate::backend::Facade;
use crate::context::CommandContext;
use crate::framebuffer::RenderBufferAny;
use crate::gl;
#[cfg(safegl_gl44)]
use crate::image_format::{ClientFormat, TextureFormat, TextureFormatRequest};
use crate::texture::{Dimensions, TextureAnyMipmap};
#[cfg(safegl_gl44)]
use crate::texture::{PixelValue, TextureAny, TextureKind};
use crate::version::{Api, Version};
use crate::{CapabilitiesSource, ContextExt, GlObject, TextureAccess, TextureExt};

/// Error that can happen when clearing a texture.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
pub enum ClearTextureError {
    /// Clearing textures needs OpenGL 4.4, `GL_ARB_clear_texture` or `GL_EXT_clear_texture`.
    NotSupported,

    /// The value can't be written to this kind of texture: an integer texture needs an integer
    /// value, a depth texture an `f32`, a stencil texture a single integer, and compressed and
    /// depth-stencil textures can't be cleared with `clear`.
    FormatNotSupported,

    /// The region is outside of the mipmap.
    OutOfBounds,
}

#[cfg(safegl_gl44)]
impl fmt::Display for ClearTextureError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(match self {
            ClearTextureError::NotSupported => "Clearing textures is not supported by the backend",
            ClearTextureError::FormatNotSupported => {
                "The clear value doesn't match the format of the texture"
            }
            ClearTextureError::OutOfBounds => "The region is outside of the mipmap",
        })
    }
}

#[cfg(safegl_gl44)]
impl Error for ClearTextureError {}

/// Error that can happen when copying between images.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CopyImageError {
    /// Copying images needs OpenGL 4.3, OpenGL ES 3.2 or `GL_ARB_copy_image`.
    NotSupported,

    /// The formats of the two images aren't compatible. Uncompressed color formats are
    /// compatible when their texels have the same size in bits, a compressed format with an
    /// uncompressed one when a block of the first has the size of a texel of the second, two
    /// compressed formats when they are variants of the same format (sRGB or not, for example),
    /// and depth and stencil formats only with themselves.
    IncompatibleFormats,

    /// The two images don't have the same number of samples.
    SampleCountMismatch,

    /// A region is outside of its image.
    OutOfBounds,

    /// A region of a compressed image doesn't start, or doesn't end, on the edge of a 4x4 block,
    /// or of the image.
    UnalignedCompressedRegion,

    /// The source and the destination are the same image, and the regions overlap.
    OverlappingRegions,

    /// The destination belongs to another context than the source. OpenGL objects can only be
    /// used with the context that created them.
    ContextMismatch,
}

impl fmt::Display for CopyImageError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(match self {
            CopyImageError::NotSupported => "Copying images is not supported by the backend",
            CopyImageError::IncompatibleFormats => {
                "The formats of the two images aren't compatible"
            }
            CopyImageError::SampleCountMismatch => {
                "The two images don't have the same number of samples"
            }
            CopyImageError::OutOfBounds => "A region is outside of its image",
            CopyImageError::UnalignedCompressedRegion => {
                "A region of a compressed image isn't aligned on 4x4 blocks"
            }
            CopyImageError::OverlappingRegions => {
                "The source and destination regions of the same image overlap"
            }
            CopyImageError::ContextMismatch => {
                "The destination belongs to another context than the source"
            }
        })
    }
}

impl Error for CopyImageError {}

/// The texture barrier isn't supported by the backend.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg(safegl_gl45)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
pub struct TextureBarrierNotSupportedError;

#[cfg(safegl_gl45)]
impl fmt::Display for TextureBarrierNotSupportedError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("The texture barrier is not supported by the backend")
    }
}

#[cfg(safegl_gl45)]
impl Error for TextureBarrierNotSupportedError {}

/// Size of a mipmap as `glClearTexSubImage` and `glCopyImageSubData` address it: the layers of
/// array textures and the faces of cubemaps count along the last used dimension.
fn image_extent(mipmap: &TextureAnyMipmap<'_>) -> (u32, u32, u32) {
    let width = mipmap.get_width();
    let height = mipmap.get_height().unwrap_or(1);
    match mipmap.get_texture().dimensions() {
        Dimensions::Texture1dArray { array_size, .. } => (width, array_size, 1),
        Dimensions::Texture2dArray { array_size, .. }
        | Dimensions::Texture2dMultisampleArray { array_size, .. } => (width, height, array_size),
        Dimensions::Texture3d { .. } => (width, height, mipmap.get_depth().unwrap_or(1)),
        Dimensions::Cubemap { .. } => (width, height, 6),
        Dimensions::CubemapArray { array_size, .. } => (width, height, array_size * 6),
        _ => (width, height, 1),
    }
}

/// Whether the box at `offset` of `size` fits in `extent`.
fn fits(offset: (u32, u32, u32), size: (u32, u32, u32), extent: (u32, u32, u32)) -> bool {
    let fits = |offset: u32, size: u32, extent: u32| {
        offset.checked_add(size).is_some_and(|end| end <= extent)
    };
    fits(offset.0, size.0, extent.0)
        && fits(offset.1, size.1, extent.1)
        && fits(offset.2, size.2, extent.2)
}

#[cfg(safegl_gl44)]
fn is_compressed_request(format: TextureFormatRequest) -> bool {
    matches!(
        format,
        TextureFormatRequest::AnyCompressed
            | TextureFormatRequest::AnyCompressedSrgb
            | TextureFormatRequest::Specific(TextureFormat::CompressedFormat(_))
            | TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(_))
    )
}

/// The `format` and `type` arguments of `glClearTexImage` for a value of `client` written to a
/// texture of `kind`, or `None` if they don't match.
#[cfg(safegl_gl44)]
fn clear_format(
    kind: TextureKind,
    client: ClientFormat,
) -> Option<(gl::types::GLenum, gl::types::GLenum)> {
    let (components, ty, integer) = match client {
        ClientFormat::U8 => (1, gl::UNSIGNED_BYTE, true),
        ClientFormat::U8U8 => (2, gl::UNSIGNED_BYTE, true),
        ClientFormat::U8U8U8 => (3, gl::UNSIGNED_BYTE, true),
        ClientFormat::U8U8U8U8 => (4, gl::UNSIGNED_BYTE, true),
        ClientFormat::I8 => (1, gl::BYTE, true),
        ClientFormat::I8I8 => (2, gl::BYTE, true),
        ClientFormat::I8I8I8 => (3, gl::BYTE, true),
        ClientFormat::I8I8I8I8 => (4, gl::BYTE, true),
        ClientFormat::U16 => (1, gl::UNSIGNED_SHORT, true),
        ClientFormat::U16U16 => (2, gl::UNSIGNED_SHORT, true),
        ClientFormat::U16U16U16 => (3, gl::UNSIGNED_SHORT, true),
        ClientFormat::U16U16U16U16 => (4, gl::UNSIGNED_SHORT, true),
        ClientFormat::I16 => (1, gl::SHORT, true),
        ClientFormat::I16I16 => (2, gl::SHORT, true),
        ClientFormat::I16I16I16 => (3, gl::SHORT, true),
        ClientFormat::I16I16I16I16 => (4, gl::SHORT, true),
        ClientFormat::U32 => (1, gl::UNSIGNED_INT, true),
        ClientFormat::U32U32 => (2, gl::UNSIGNED_INT, true),
        ClientFormat::U32U32U32 => (3, gl::UNSIGNED_INT, true),
        ClientFormat::U32U32U32U32 => (4, gl::UNSIGNED_INT, true),
        ClientFormat::I32 => (1, gl::INT, true),
        ClientFormat::I32I32 => (2, gl::INT, true),
        ClientFormat::I32I32I32 => (3, gl::INT, true),
        ClientFormat::I32I32I32I32 => (4, gl::INT, true),
        ClientFormat::F16 => (1, gl::HALF_FLOAT, false),
        ClientFormat::F16F16 => (2, gl::HALF_FLOAT, false),
        ClientFormat::F16F16F16 => (3, gl::HALF_FLOAT, false),
        ClientFormat::F16F16F16F16 => (4, gl::HALF_FLOAT, false),
        ClientFormat::F32 => (1, gl::FLOAT, false),
        ClientFormat::F32F32 => (2, gl::FLOAT, false),
        ClientFormat::F32F32F32 => (3, gl::FLOAT, false),
        ClientFormat::F32F32F32F32 => (4, gl::FLOAT, false),
        // packed formats
        _ => return None,
    };

    let color = |base: [gl::types::GLenum; 4]| base[components - 1];
    let format = match kind {
        // normalized integers are converted to 0..1 (or -1..1)
        TextureKind::Float => color([gl::RED, gl::RG, gl::RGB, gl::RGBA]),
        TextureKind::Integral | TextureKind::Unsigned if integer => color([
            gl::RED_INTEGER,
            gl::RG_INTEGER,
            gl::RGB_INTEGER,
            gl::RGBA_INTEGER,
        ]),
        TextureKind::Depth if client == ClientFormat::F32 => gl::DEPTH_COMPONENT,
        TextureKind::Stencil if integer && components == 1 => gl::STENCIL_INDEX,
        _ => return None,
    };
    Some((format, ty))
}

/// Returns true if textures can be cleared with `TextureAnyMipmap::clear`: OpenGL 4.4,
/// `GL_ARB_clear_texture`, or `GL_EXT_clear_texture` on OpenGL ES.
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
pub fn is_clear_texture_supported<C>(context: &C) -> bool
where
    C: ?Sized + CapabilitiesSource,
{
    context.get_version() >= &Version(Api::Gl, 4, 4)
        || context.get_extensions().gl_arb_clear_texture
        || context.get_extensions().gl_ext_clear_texture
}

/// Returns true if images can be copied with `copy_image`: OpenGL 4.3, OpenGL ES 3.2 or
/// `GL_ARB_copy_image`.
pub fn is_copy_image_supported<C>(context: &C) -> bool
where
    C: ?Sized + CapabilitiesSource,
{
    context.get_version() >= &Version(Api::Gl, 4, 3)
        || context.get_version() >= &Version(Api::GlEs, 3, 2)
        || context.get_extensions().gl_arb_copy_image
}

impl<'a> TextureAnyMipmap<'a> {
    /// Sets every texel of this mipmap level to `value` (`glClearTexImage`), without going
    /// through a framebuffer. All the layers of array textures and faces of cubemaps are
    /// cleared.
    ///
    /// The value is converted like data uploaded to the texture: `(f32, f32, f32, f32)` or
    /// `(u8, u8, u8, u8)` for color textures, a tuple of integers of the right sign for integer
    /// textures, `f32` for depth textures and `u8` for stencil textures. A value with fewer
    /// components than the texture leaves the others at 0 (alpha at 1). Use
    /// `clear_depth_stencil` for depth-stencil textures. Compressed textures can't be cleared.
    ///
    /// Unlike clearing a framebuffer, this works on every kind of texture, ignores the scissor
    /// box and the color mask, and different textures can be cleared to different values.
    ///
    /// Needs OpenGL 4.4, `GL_ARB_clear_texture` or `GL_EXT_clear_texture` (see
    /// `is_clear_texture_supported`).
    #[cfg(safegl_gl44)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
    pub fn clear<P: PixelValue>(&self, value: P) -> Result<(), ClearTextureError> {
        self.clear_region((0, 0, 0), image_extent(self), value)
    }

    /// Sets the texels of a box of this mipmap level to `value` (`glClearTexSubImage`).
    ///
    /// `offset` and `size` are in texels. The second coordinate is the layer for 1D array
    /// textures, and the third is the layer for 2D array textures, the face for cubemaps (in
    /// the order of `CubeLayer`) and `6 * layer + face` for cubemap arrays.
    ///
    /// See `clear` for the values and the requirements.
    #[cfg(safegl_gl44)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
    pub fn clear_region<P: PixelValue>(
        &self,
        offset: (u32, u32, u32),
        size: (u32, u32, u32),
        value: P,
    ) -> Result<(), ClearTextureError> {
        let texture = self.get_texture();
        if is_compressed_request(texture.get_requested_format()) {
            return Err(ClearTextureError::FormatNotSupported);
        }
        let (format, ty) = clear_format(texture.kind(), P::get_format())
            .ok_or(ClearTextureError::FormatNotSupported)?;
        if !fits(offset, size, image_extent(self)) {
            return Err(ClearTextureError::OutOfBounds);
        }

        let region = if offset == (0, 0, 0) && size == image_extent(self) {
            None
        } else {
            Some((offset, size))
        };
        unsafe { clear(texture, self.get_level(), region, format, ty, &value) }
    }

    /// Sets every texel of this mipmap level of a depth-stencil texture to `depth` and
    /// `stencil` (`glClearTexImage`).
    ///
    /// See `clear` for the requirements.
    #[cfg(safegl_gl44)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
    pub fn clear_depth_stencil(&self, depth: f32, stencil: u8) -> Result<(), ClearTextureError> {
        let texture = self.get_texture();
        if texture.kind() != TextureKind::DepthStencil {
            return Err(ClearTextureError::FormatNotSupported);
        }
        // `GL_UNSIGNED_INT_24_8` packs a 24-bit normalized depth above an 8-bit stencil, and
        // is converted to whatever depth-stencil format the texture has
        let depth = (depth.clamp(0.0, 1.0) as f64 * 0xff_ffff as f64).round() as u32;
        let value = (depth << 8) | stencil as u32;

        unsafe {
            clear(
                texture,
                self.get_level(),
                None,
                gl::DEPTH_STENCIL,
                gl::UNSIGNED_INT_24_8,
                &value,
            )
        }
    }

    /// Copies this whole mipmap level to `destination`, which must have the same dimensions
    /// (`glCopyImageSubData`).
    ///
    /// See `copy_image` for the requirements.
    pub fn copy_to(&self, destination: &TextureAnyMipmap<'_>) -> Result<(), CopyImageError> {
        let extent = image_extent(self);
        if image_extent(destination) != extent {
            return Err(CopyImageError::OutOfBounds);
        }
        copy_image(*self, (0, 0, 0), *destination, (0, 0, 0), extent)
    }
}

/// Clears a box of a mipmap of `texture`, given as offset and size, or the whole mipmap.
#[cfg(safegl_gl44)]
unsafe fn clear<P>(
    texture: &TextureAny,
    level: u32,
    region: Option<((u32, u32, u32), (u32, u32, u32))>,
    format: gl::types::GLenum,
    ty: gl::types::GLenum,
    value: &P,
) -> Result<(), ClearTextureError> {
    let mut ctxt = texture.get_context().make_current();
    // the core functions, or those of `GL_EXT_clear_texture`
    let core = ctxt.version >= &Version(Api::Gl, 4, 4) || ctxt.extensions.gl_arb_clear_texture;
    if !core && !ctxt.extensions.gl_ext_clear_texture {
        return Err(ClearTextureError::NotSupported);
    }

    // shader writes made through image units must be done before the texture is changed
    texture.prepare_for_access(&mut ctxt, TextureAccess::Update);

    let id = texture.get_id().0.get();
    let data = value as *const P as *const c_void;
    let level = level as gl::types::GLint;
    match region {
        Some((offset, size)) => {
            let clear_sub_image = if core {
                gl::Extra::ClearTexSubImage
            } else {
                gl::Extra::ClearTexSubImageEXT
            };
            clear_sub_image(
                ctxt.extra,
                id,
                level,
                offset.0 as gl::types::GLint,
                offset.1 as gl::types::GLint,
                offset.2 as gl::types::GLint,
                size.0 as gl::types::GLsizei,
                size.1 as gl::types::GLsizei,
                size.2 as gl::types::GLsizei,
                format,
                ty,
                data,
            );
        }
        None if core => ctxt.extra.ClearTexImage(id, level, format, ty, data),
        None => ctxt.extra.ClearTexImageEXT(id, level, format, ty, data),
    }
    Ok(())
}

/// An image that `copy_image` can copy from or to: a mipmap level of a texture, or a render
/// buffer.
#[derive(Copy, Clone)]
pub enum CopyImageTarget<'a> {
    /// A mipmap level of a texture, with all its layers or faces.
    Texture(TextureAnyMipmap<'a>),
    /// A render buffer.
    RenderBuffer(&'a RenderBufferAny),
}

impl<'a> From<TextureAnyMipmap<'a>> for CopyImageTarget<'a> {
    #[inline]
    fn from(mipmap: TextureAnyMipmap<'a>) -> CopyImageTarget<'a> {
        CopyImageTarget::Texture(mipmap)
    }
}

impl<'a> From<&'a RenderBufferAny> for CopyImageTarget<'a> {
    #[inline]
    fn from(render_buffer: &'a RenderBufferAny) -> CopyImageTarget<'a> {
        CopyImageTarget::RenderBuffer(render_buffer)
    }
}

macro_rules! impl_copy_image_target_for_render_buffer {
    ($($ty:ty),+) => {
        $(
            impl<'a> From<&'a $ty> for CopyImageTarget<'a> {
                #[inline]
                fn from(render_buffer: &'a $ty) -> CopyImageTarget<'a> {
                    CopyImageTarget::RenderBuffer(render_buffer)
                }
            }
        )+
    };
}

impl_copy_image_target_for_render_buffer!(
    crate::framebuffer::RenderBuffer,
    crate::framebuffer::DepthRenderBuffer,
    crate::framebuffer::StencilRenderBuffer,
    crate::framebuffer::DepthStencilRenderBuffer
);

/// What decides whether two images can be copied to each other.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CopyClass {
    /// An uncompressed color format with texels of this many bits.
    Uncompressed(u32),
    /// A compressed format with 4x4 blocks of this many bytes, and the formats it is
    /// compatible with (the internal format of its non-sRGB variant).
    Compressed {
        block_bytes: u32,
        family: gl::types::GLenum,
    },
    /// A depth, stencil or depth-stencil format, compatible only with itself.
    DepthStencil(gl::types::GLenum),
}

/// The compressed formats that can be copied, with the size of their blocks and the format
/// that represents their family.
fn compressed_class(internal_format: gl::types::GLenum) -> Option<CopyClass> {
    let (block_bytes, family) = match internal_format {
        gl::COMPRESSED_RGB_S3TC_DXT1_EXT | gl::COMPRESSED_SRGB_S3TC_DXT1_EXT => {
            (8, gl::COMPRESSED_RGB_S3TC_DXT1_EXT)
        }
        gl::COMPRESSED_RGBA_S3TC_DXT1_EXT | gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT => {
            (8, gl::COMPRESSED_RGBA_S3TC_DXT1_EXT)
        }
        gl::COMPRESSED_RGBA_S3TC_DXT3_EXT | gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT => {
            (16, gl::COMPRESSED_RGBA_S3TC_DXT3_EXT)
        }
        gl::COMPRESSED_RGBA_S3TC_DXT5_EXT | gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT => {
            (16, gl::COMPRESSED_RGBA_S3TC_DXT5_EXT)
        }
        gl::COMPRESSED_RED_RGTC1 => (8, gl::COMPRESSED_RED_RGTC1),
        gl::COMPRESSED_SIGNED_RED_RGTC1 => (8, gl::COMPRESSED_SIGNED_RED_RGTC1),
        gl::COMPRESSED_RG_RGTC2 => (16, gl::COMPRESSED_RG_RGTC2),
        gl::COMPRESSED_SIGNED_RG_RGTC2 => (16, gl::COMPRESSED_SIGNED_RG_RGTC2),
        gl::COMPRESSED_RGBA_BPTC_UNORM | gl::COMPRESSED_SRGB_ALPHA_BPTC_UNORM => {
            (16, gl::COMPRESSED_RGBA_BPTC_UNORM)
        }
        gl::COMPRESSED_RGB_BPTC_SIGNED_FLOAT | gl::COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT => {
            (16, gl::COMPRESSED_RGB_BPTC_SIGNED_FLOAT)
        }
        gl::COMPRESSED_RGB8_ETC2 | gl::COMPRESSED_SRGB8_ETC2 => (8, gl::COMPRESSED_RGB8_ETC2),
        gl::COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2
        | gl::COMPRESSED_SRGB8_PUNCHTHROUGH_ALPHA1_ETC2 => {
            (8, gl::COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2)
        }
        gl::COMPRESSED_RGBA8_ETC2_EAC | gl::COMPRESSED_SRGB8_ALPHA8_ETC2_EAC => {
            (16, gl::COMPRESSED_RGBA8_ETC2_EAC)
        }
        gl::COMPRESSED_R11_EAC => (8, gl::COMPRESSED_R11_EAC),
        gl::COMPRESSED_SIGNED_R11_EAC => (8, gl::COMPRESSED_SIGNED_R11_EAC),
        gl::COMPRESSED_RG11_EAC => (16, gl::COMPRESSED_RG11_EAC),
        gl::COMPRESSED_SIGNED_RG11_EAC => (16, gl::COMPRESSED_SIGNED_RG11_EAC),
        _ => return None,
    };
    Some(CopyClass::Compressed {
        block_bytes,
        family,
    })
}

/// An image as `glCopyImageSubData` sees it.
struct CopyImage {
    name: u32,
    target: gl::types::GLenum,
    level: u32,
    extent: (u32, u32, u32),
    samples: Option<u32>,
    /// `None` if the format can't be copied at all.
    class: Option<CopyClass>,
}

impl CopyImage {
    unsafe fn new(ctxt: &mut CommandContext<'_>, image: CopyImageTarget<'_>) -> CopyImage {
        match image {
            CopyImageTarget::Texture(mipmap) => {
                let texture = mipmap.get_texture();
                let level = mipmap.get_level();
                let target = texture.get_bind_point();
                texture.bind_to_current(ctxt);
                // the parameters of a cubemap are those of its faces
                let query_target = if target == gl::TEXTURE_CUBE_MAP {
                    gl::TEXTURE_CUBE_MAP_POSITIVE_X
                } else {
                    target
                };
                let parameter = |pname| {
                    ctxt.glow
                        .get_tex_level_parameter_i32(query_target, level as i32, pname)
                };
                let compressed = parameter(gl::TEXTURE_COMPRESSED) != 0;
                let sizes = [
                    gl::TEXTURE_RED_SIZE,
                    gl::TEXTURE_GREEN_SIZE,
                    gl::TEXTURE_BLUE_SIZE,
                    gl::TEXTURE_ALPHA_SIZE,
                    // the exponent shared by the components of `GL_RGB9_E5`
                    gl::TEXTURE_SHARED_SIZE,
                    gl::TEXTURE_DEPTH_SIZE,
                    gl::TEXTURE_STENCIL_SIZE,
                ]
                .map(|pname| parameter(pname).max(0) as u32);
                let internal_format = parameter(gl::TEXTURE_INTERNAL_FORMAT) as gl::types::GLenum;

                CopyImage {
                    name: texture.get_id().0.get(),
                    target,
                    level,
                    extent: image_extent(&mipmap),
                    samples: texture.get_samples(),
                    class: copy_class(compressed, sizes, internal_format),
                }
            }

            CopyImageTarget::RenderBuffer(render_buffer) => {
                let id = render_buffer.get_id();
                ctxt.glow.bind_renderbuffer(glow::RENDERBUFFER, Some(id));
                ctxt.state.renderbuffer = Some(id);
                let parameter = |pname| {
                    ctxt.glow
                        .get_renderbuffer_parameter_i32(glow::RENDERBUFFER, pname)
                };
                let size = |pname| parameter(pname).max(0) as u32;
                // render buffers can't have shared exponents
                let sizes = [
                    size(gl::RENDERBUFFER_RED_SIZE),
                    size(gl::RENDERBUFFER_GREEN_SIZE),
                    size(gl::RENDERBUFFER_BLUE_SIZE),
                    size(gl::RENDERBUFFER_ALPHA_SIZE),
                    0,
                    size(gl::RENDERBUFFER_DEPTH_SIZE),
                    size(gl::RENDERBUFFER_STENCIL_SIZE),
                ];
                let internal_format =
                    parameter(gl::RENDERBUFFER_INTERNAL_FORMAT) as gl::types::GLenum;
                let (width, height) = render_buffer.get_dimensions();

                CopyImage {
                    name: id.0.get(),
                    target: gl::RENDERBUFFER,
                    level: 0,
                    extent: (width, height, 1),
                    samples: render_buffer.get_samples(),
                    class: copy_class(false, sizes, internal_format),
                }
            }
        }
    }

    fn is_compressed(&self) -> bool {
        matches!(self.class, Some(CopyClass::Compressed { .. }))
    }

    /// Checks that a region of `size` texels of this image at `offset` starts and ends on the
    /// edges of compressed blocks, if the image is compressed.
    fn is_aligned(&self, offset: (u32, u32, u32), size: (u32, u32, u32)) -> bool {
        if !self.is_compressed() {
            return true;
        }
        let aligned = |offset: u32, size: u32, extent: u32| {
            offset.is_multiple_of(4) && (size.is_multiple_of(4) || offset + size == extent)
        };
        aligned(offset.0, size.0, self.extent.0) && aligned(offset.1, size.1, self.extent.1)
    }
}

fn copy_class(
    compressed: bool,
    sizes: [u32; 7],
    internal_format: gl::types::GLenum,
) -> Option<CopyClass> {
    let [red, green, blue, alpha, shared, depth, stencil] = sizes;
    if compressed {
        compressed_class(internal_format)
    } else if depth != 0 || stencil != 0 {
        Some(CopyClass::DepthStencil(internal_format))
    } else {
        Some(CopyClass::Uncompressed(red + green + blue + alpha + shared))
    }
}

/// Copies a box of texels from one image to another (`glCopyImageSubData`), without going
/// through a framebuffer and without any conversion: the bits of each texel are copied as
/// they are.
///
/// `source_offset`, `destination_offset` and `size` are in texels of the source. Their second
/// coordinate is the layer for 1D array textures, and their third is the layer for 2D array
/// textures, the face for cubemaps (in the order of `CubeLayer`) and `6 * layer + face` for
/// cubemap arrays. When exactly one of the images is compressed, a 4x4 block of the compressed
/// image corresponds to a texel of the other, so the box covers `size / 4` texels of the
/// uncompressed destination, or `size * 4` texels of the compressed destination.
///
/// The images must have compatible formats (see `CopyImageError::IncompatibleFormats`) and the
/// same number of samples. Regions of compressed images must be aligned on blocks, and a copy
/// within one image must not overlap itself.
///
/// Needs OpenGL 4.3, OpenGL ES 3.2 or `GL_ARB_copy_image` (see `is_copy_image_supported`).
///
/// ```no_run
/// # fn example(display: &safegl::Display<glutin::surface::WindowSurface>) {
/// use safegl::texture::{copy_image, Texture2d};
///
/// let source = Texture2d::empty(display, 256, 256).unwrap();
/// let destination = Texture2d::empty(display, 128, 128).unwrap();
/// // the top-right quarter of the source, to the bottom-left corner of the destination
/// copy_image(*source.main_level(), (128, 128, 0), *destination.main_level(), (0, 0, 0),
///            (128, 128, 1)).unwrap();
/// # }
/// ```
pub fn copy_image<'a, 'b>(
    source: impl Into<CopyImageTarget<'a>>,
    source_offset: (u32, u32, u32),
    destination: impl Into<CopyImageTarget<'b>>,
    destination_offset: (u32, u32, u32),
    size: (u32, u32, u32),
) -> Result<(), CopyImageError> {
    let source = source.into();
    let destination = destination.into();
    let context_of = |target: &CopyImageTarget<'_>| match *target {
        CopyImageTarget::Texture(mipmap) => mipmap.get_texture().get_context().clone(),
        CopyImageTarget::RenderBuffer(render_buffer) => render_buffer.get_context().clone(),
    };
    let context = context_of(&source);

    // OpenGL objects can only be used with the context that created them
    if !Rc::ptr_eq(&context, &context_of(&destination)) {
        return Err(CopyImageError::ContextMismatch);
    }

    let mut ctxt = context.make_current();
    if !is_copy_image_supported(&*context) {
        return Err(CopyImageError::NotSupported);
    }

    let (src, dst) = unsafe {
        (
            CopyImage::new(&mut ctxt, source),
            CopyImage::new(&mut ctxt, destination),
        )
    };

    if src.samples != dst.samples {
        return Err(CopyImageError::SampleCountMismatch);
    }

    // the size of the box in texels of the destination
    let destination_size = match (src.class, dst.class) {
        (Some(a), Some(b)) if a == b => size,
        (
            Some(CopyClass::Compressed {
                block_bytes,
                family: _,
            }),
            Some(CopyClass::Uncompressed(bits)),
        ) if block_bytes * 8 == bits => (size.0.div_ceil(4), size.1.div_ceil(4), size.2),
        (
            Some(CopyClass::Uncompressed(bits)),
            Some(CopyClass::Compressed {
                block_bytes,
                family: _,
            }),
        ) if block_bytes * 8 == bits => (size.0 * 4, size.1 * 4, size.2),
        _ => return Err(CopyImageError::IncompatibleFormats),
    };

    if !fits(source_offset, size, src.extent)
        || !fits(destination_offset, destination_size, dst.extent)
    {
        return Err(CopyImageError::OutOfBounds);
    }
    if !src.is_aligned(source_offset, size) || !dst.is_aligned(destination_offset, destination_size)
    {
        return Err(CopyImageError::UnalignedCompressedRegion);
    }
    if src.name == dst.name && src.target == dst.target && src.level == dst.level {
        let overlap = |a: u32, b: u32, size: u32| a < b + size && b < a + size;
        if overlap(source_offset.0, destination_offset.0, size.0)
            && overlap(source_offset.1, destination_offset.1, size.1)
            && overlap(source_offset.2, destination_offset.2, size.2)
        {
            return Err(CopyImageError::OverlappingRegions);
        }
    }

    // shader writes made through image units must be done before the copy reads or writes
    for image in [source, destination] {
        if let CopyImageTarget::Texture(mipmap) = image {
            mipmap
                .get_texture()
                .prepare_for_access(&mut ctxt, TextureAccess::Update);
        }
    }

    unsafe {
        ctxt.extra.CopyImageSubData(
            src.name,
            src.target,
            src.level as gl::types::GLint,
            source_offset.0 as gl::types::GLint,
            source_offset.1 as gl::types::GLint,
            source_offset.2 as gl::types::GLint,
            dst.name,
            dst.target,
            dst.level as gl::types::GLint,
            destination_offset.0 as gl::types::GLint,
            destination_offset.1 as gl::types::GLint,
            destination_offset.2 as gl::types::GLint,
            size.0 as gl::types::GLsizei,
            size.1 as gl::types::GLsizei,
            size.2 as gl::types::GLsizei,
        );
    }

    Ok(())
}

/// Returns true if `texture_barrier` is supported: OpenGL 4.5, `GL_ARB_texture_barrier` or
/// `GL_NV_texture_barrier`.
#[cfg(safegl_gl45)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
pub fn is_texture_barrier_supported<C>(context: &C) -> bool
where
    C: ?Sized + CapabilitiesSource,
{
    context.get_version() >= &Version(Api::Gl, 4, 5)
        || context.get_extensions().gl_arb_texture_barrier
        || context.get_extensions().gl_nv_texture_barrier
}

/// Makes the texels that previous draw calls wrote visible to the draw calls that follow, when
/// they read them from a texture that is also attached to the framebuffer they draw to
/// (`glTextureBarrier`).
///
/// Reading a texture while it is attached to the framebuffer being drawn to (a feedback loop)
/// normally gives undefined results. It becomes well defined when the draw calls between two
/// barriers write each texel at most once, and read only the texels they write themselves, or
/// texels that no draw call since the last barrier wrote. This allows "programmable blending":
/// the fragment shader reads the texel under it with `texelFetch(texture, ivec2(gl_FragCoord.xy),
/// 0)` and writes the blended result, with a barrier between overlapping draws.
///
/// SafeGL doesn't check these rules: breaking them only makes the content of the texture
/// undefined.
///
/// Needs OpenGL 4.5, `GL_ARB_texture_barrier` or `GL_NV_texture_barrier`.
#[cfg(safegl_gl45)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
pub fn texture_barrier<F>(facade: &F) -> Result<(), TextureBarrierNotSupportedError>
where
    F: ?Sized + Facade,
{
    let ctxt = facade.get_context().make_current();
    unsafe {
        if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_texture_barrier {
            ctxt.extra.TextureBarrier();
        } else if ctxt.extensions.gl_nv_texture_barrier {
            ctxt.extra.TextureBarrierNV();
        } else {
            return Err(TextureBarrierNotSupportedError);
        }
    }
    Ok(())
}
