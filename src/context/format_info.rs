//! What the implementation supports for each internal format (`glGetInternalformativ` with the
//! queries of OpenGL 4.3 and `GL_ARB_internalformat_query2`).

use std::error::Error;
use std::fmt;

use glow::HasContext;

use crate::context::Context;
use crate::gl;
use crate::image_format::{self, ClientFormat, TextureFormat, TextureFormatRequest};
use crate::version::{Api, Version};
use crate::{CapabilitiesSource, ContextExt, ToGlEnum};

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

/// The type of a component of an internal format (`GL_INTERNALFORMAT_RED_TYPE`, ...).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ComponentType {
    /// The format doesn't have the component.
    None,
    /// Signed values normalized to [-1, 1].
    SignedNormalized,
    /// Unsigned values normalized to [0, 1].
    UnsignedNormalized,
    /// Floating-point values.
    Float,
    /// Signed integers.
    Int,
    /// Unsigned integers.
    UnsignedInt,
}

impl ComponentType {
    fn from_glenum(value: gl::types::GLenum) -> ComponentType {
        match value {
            gl::SIGNED_NORMALIZED => ComponentType::SignedNormalized,
            gl::UNSIGNED_NORMALIZED => ComponentType::UnsignedNormalized,
            gl::FLOAT => ComponentType::Float,
            gl::INT => ComponentType::Int,
            gl::UNSIGNED_INT => ComponentType::UnsignedInt,
            _ => ComponentType::None,
        }
    }
}

/// The size in bits and the type of each component of an internal format, `(0,
/// ComponentType::None)` for the components it doesn't have.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct FormatComponents {
    /// Red (`GL_INTERNALFORMAT_RED_SIZE` and `_TYPE`).
    pub red: (u32, ComponentType),
    /// Green.
    pub green: (u32, ComponentType),
    /// Blue.
    pub blue: (u32, ComponentType),
    /// Alpha.
    pub alpha: (u32, ComponentType),
    /// Depth.
    pub depth: (u32, ComponentType),
    /// Stencil.
    pub stencil: (u32, ComponentType),
    /// The bits of the exponent that the color components share, like in `F9F9F9`
    /// (`GL_INTERNALFORMAT_SHARED_SIZE`).
    pub shared_exponent: u32,
}

/// How the color components of a format are encoded (`GL_COLOR_ENCODING`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorEncoding {
    /// Linear values.
    Linear,
    /// sRGB values, converted to linear when sampled.
    Srgb,
}

/// The support of an operation in each shader stage.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct StageSupport {
    /// Vertex shaders.
    pub vertex: FormatSupport,
    /// Tessellation control shaders.
    pub tessellation_control: FormatSupport,
    /// Tessellation evaluation shaders.
    pub tessellation_evaluation: FormatSupport,
    /// Geometry shaders.
    pub geometry: FormatSupport,
    /// Fragment shaders.
    pub fragment: FormatSupport,
    /// Compute shaders.
    pub compute: FormatSupport,
}

/// The class of the image formats that an image unit of a texture of this format can use in
/// place of its own (`GL_IMAGE_COMPATIBILITY_CLASS`): the number and size of the components.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormatClass {
    /// Four 32-bit components.
    FourX32,
    /// Two 32-bit components.
    TwoX32,
    /// One 32-bit component.
    OneX32,
    /// Four 16-bit components.
    FourX16,
    /// Two 16-bit components.
    TwoX16,
    /// One 16-bit component.
    OneX16,
    /// Four 8-bit components.
    FourX8,
    /// Two 8-bit components.
    TwoX8,
    /// One 8-bit component.
    OneX8,
    /// 11, 11 and 10 bits.
    Bits11_11_10,
    /// 10, 10, 10 and 2 bits.
    Bits10_10_10_2,
}

/// Which image formats can be used in place of the format of a texture by image units
/// (`GL_IMAGE_FORMAT_COMPATIBILITY_TYPE`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageCompatibility {
    /// The formats of the same texel size.
    BySize,
    /// The formats of the same class (`ImageFormatClass`).
    ByClass,
}

/// The class of the formats that texture views of a texture of this format can have
/// (`GL_VIEW_COMPATIBILITY_CLASS`): the formats of a class reinterpret each other's texels.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ViewClass {
    /// 128-bit texels.
    Bits128,
    /// 96-bit texels.
    Bits96,
    /// 64-bit texels.
    Bits64,
    /// 48-bit texels.
    Bits48,
    /// 32-bit texels.
    Bits32,
    /// 24-bit texels.
    Bits24,
    /// 16-bit texels.
    Bits16,
    /// 8-bit texels.
    Bits8,
    /// RGTC with one component.
    Rgtc1Red,
    /// RGTC with two components.
    Rgtc2Rg,
    /// BPTC of normalized values.
    BptcUnorm,
    /// BPTC of floats.
    BptcFloat,
    /// DXT1 without alpha.
    S3tcDxt1Rgb,
    /// DXT1 with alpha.
    S3tcDxt1Rgba,
    /// DXT3.
    S3tcDxt3Rgba,
    /// DXT5.
    S3tcDxt5Rgba,
    /// EAC with one component.
    EacR11,
    /// EAC with two components.
    EacRg11,
    /// ETC2 without alpha.
    Etc2Rgb,
    /// ETC2 with one bit of alpha.
    Etc2Rgba,
    /// ETC2 with EAC alpha.
    Etc2EacRgba,
}

impl ViewClass {
    fn from_glenum(value: gl::types::GLenum) -> Option<ViewClass> {
        Some(match value {
            gl::VIEW_CLASS_128_BITS => ViewClass::Bits128,
            gl::VIEW_CLASS_96_BITS => ViewClass::Bits96,
            gl::VIEW_CLASS_64_BITS => ViewClass::Bits64,
            gl::VIEW_CLASS_48_BITS => ViewClass::Bits48,
            gl::VIEW_CLASS_32_BITS => ViewClass::Bits32,
            gl::VIEW_CLASS_24_BITS => ViewClass::Bits24,
            gl::VIEW_CLASS_16_BITS => ViewClass::Bits16,
            gl::VIEW_CLASS_8_BITS => ViewClass::Bits8,
            gl::VIEW_CLASS_RGTC1_RED => ViewClass::Rgtc1Red,
            gl::VIEW_CLASS_RGTC2_RG => ViewClass::Rgtc2Rg,
            gl::VIEW_CLASS_BPTC_UNORM => ViewClass::BptcUnorm,
            gl::VIEW_CLASS_BPTC_FLOAT => ViewClass::BptcFloat,
            gl::VIEW_CLASS_S3TC_DXT1_RGB => ViewClass::S3tcDxt1Rgb,
            gl::VIEW_CLASS_S3TC_DXT1_RGBA => ViewClass::S3tcDxt1Rgba,
            gl::VIEW_CLASS_S3TC_DXT3_RGBA => ViewClass::S3tcDxt3Rgba,
            gl::VIEW_CLASS_S3TC_DXT5_RGBA => ViewClass::S3tcDxt5Rgba,
            gl::VIEW_CLASS_EAC_R11 => ViewClass::EacR11,
            gl::VIEW_CLASS_EAC_RG11 => ViewClass::EacRg11,
            gl::VIEW_CLASS_ETC2_RGB => ViewClass::Etc2Rgb,
            gl::VIEW_CLASS_ETC2_RGBA => ViewClass::Etc2Rgba,
            gl::VIEW_CLASS_ETC2_EAC_RGBA => ViewClass::Etc2EacRgba,
            _ => return None,
        })
    }
}

/// Whether textures of a format can be sampled, and their depth or stencil tested or written
/// by the draw, at the same time (`GL_SIMULTANEOUS_TEXTURE_AND_*`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SimultaneousUse {
    /// Sampled while used by the depth test.
    pub depth_test: FormatSupport,
    /// Sampled while used by the stencil test.
    pub stencil_test: FormatSupport,
    /// Sampled while depths are written.
    pub depth_write: FormatSupport,
    /// Sampled while stencil values are written.
    pub stencil_write: FormatSupport,
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
    /// The format that the implementation prefers to store this one as, if it knows a better one
    /// (`GL_INTERNALFORMAT_PREFERRED`).
    pub preferred_format: Option<TextureFormat>,
    /// The sizes and types of the components (`GL_INTERNALFORMAT_RED_SIZE`, ...).
    pub components: FormatComponents,
    /// Whether the format has color, depth and stencil components (`GL_COLOR_COMPONENTS`, ...).
    pub has_components: (bool, bool, bool),
    /// The largest number of texels of a texture of this format (`GL_MAX_COMBINED_DIMENSIONS`).
    pub max_combined_dimensions: u64,
    /// Whether layered framebuffers can have it (`GL_FRAMEBUFFER_RENDERABLE_LAYERED`).
    pub framebuffer_renderable_layered: FormatSupport,
    /// Whether it can be read with `glReadPixels` (`GL_READ_PIXELS`), and the client format
    /// that reads it best, if SafeGL has it (`GL_READ_PIXELS_FORMAT` and `_TYPE`).
    pub read_pixels: (FormatSupport, Option<ClientFormat>),
    /// The client format that uploads to it best, if SafeGL has it
    /// (`GL_TEXTURE_IMAGE_FORMAT` and `_TYPE`).
    pub upload_format: Option<ClientFormat>,
    /// The client format that reads its levels best, if SafeGL has it
    /// (`GL_GET_TEXTURE_IMAGE_FORMAT` and `_TYPE`).
    pub download_format: Option<ClientFormat>,
    /// Whether its mipmaps can be generated by draws that write each level
    /// (`GL_MANUAL_GENERATE_MIPMAP`).
    pub manual_mipmap_generation: FormatSupport,
    /// How the color components are encoded, `None` for formats without color.
    pub color_encoding: Option<ColorEncoding>,
    /// Whether it converts from sRGB when sampled and to sRGB when drawn into
    /// (`GL_SRGB_READ`, `GL_SRGB_WRITE`).
    pub srgb: (FormatSupport, FormatSupport),
    /// Whether shaders of each stage can sample it (`GL_VERTEX_TEXTURE`, ...).
    pub sampling: StageSupport,
    /// Whether depth comparisons, gathers and gathers of comparisons work when sampling it
    /// (`GL_TEXTURE_SHADOW`, `GL_TEXTURE_GATHER`, `GL_TEXTURE_GATHER_SHADOW`).
    pub shadow_and_gather: (FormatSupport, FormatSupport, FormatSupport),
    /// The size of a texel in image units (`GL_IMAGE_TEXEL_SIZE`), 0 if they can't use it. The
    /// specification gives it in bits, as Mesa does; NVIDIA gives it in bytes.
    pub image_texel_size: u32,
    /// The class of the image formats that image units can use for it, as the driver reports
    /// it: NVIDIA reports four 32-bit components for formats of four 8-bit components.
    pub image_class: Option<ImageFormatClass>,
    /// The client format of the texels that image units see, if SafeGL has it
    /// (`GL_IMAGE_PIXEL_FORMAT` and `_TYPE`).
    pub image_pixel_format: Option<ClientFormat>,
    /// Which image formats image units can use in its place.
    pub image_compatibility: Option<ImageCompatibility>,
    /// Whether it can be sampled while its depth or stencil is tested or written.
    pub simultaneous: SimultaneousUse,
    /// Whether it is compressed (`GL_TEXTURE_COMPRESSED`), and the width and height in texels and
    /// the size in bytes of its blocks if it is. For the compressed formats of SafeGL these are
    /// the sizes of the format, since drivers report them differently
    /// (`GL_TEXTURE_COMPRESSED_BLOCK_*`: NVIDIA gives the size in bits, Mesa smaller blocks).
    pub compressed: Option<(u32, u32, u32)>,
    /// Whether buffers of this format can be cleared (`GL_CLEAR_BUFFER`).
    pub clear_buffer: FormatSupport,
    /// Whether textures of this format can be cleared (`GL_CLEAR_TEXTURE`, OpenGL 4.4 or
    /// `GL_ARB_clear_texture`; `None` without them).
    pub clear_texture: Option<FormatSupport>,
    /// Whether textures of this format can have views (`GL_TEXTURE_VIEW`), and the class of the
    /// formats of the views.
    pub views: (FormatSupport, Option<ViewClass>),
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

impl FormatInfo {
    /// The answers for a format that the context doesn't know.
    fn unsupported() -> FormatInfo {
        let none = FormatSupport::None;
        let component = (0, ComponentType::None);
        FormatInfo {
            supported: false,
            max_size: (0, 0, 0, 0),
            renderable: (false, false, false),
            framebuffer_renderable: none,
            framebuffer_blend: none,
            filter: none,
            mipmaps: false,
            mipmap_generation: none,
            image: (none, none, none),
            texture_views: false,
            samples: Vec::new(),
            preferred_format: None,
            components: FormatComponents {
                red: component,
                green: component,
                blue: component,
                alpha: component,
                depth: component,
                stencil: component,
                shared_exponent: 0,
            },
            has_components: (false, false, false),
            max_combined_dimensions: 0,
            framebuffer_renderable_layered: none,
            read_pixels: (none, None),
            upload_format: None,
            download_format: None,
            manual_mipmap_generation: none,
            color_encoding: None,
            srgb: (none, none),
            sampling: StageSupport {
                vertex: none,
                tessellation_control: none,
                tessellation_evaluation: none,
                geometry: none,
                fragment: none,
                compute: none,
            },
            shadow_and_gather: (none, none, none),
            image_texel_size: 0,
            image_class: None,
            image_pixel_format: None,
            image_compatibility: None,
            simultaneous: SimultaneousUse {
                depth_test: none,
                stencil_test: none,
                depth_write: none,
                stencil_write: none,
            },
            compressed: None,
            clear_buffer: none,
            clear_texture: None,
            views: (none, None),
        }
    }
}

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
            None => FormatInfo::unsupported(),
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
        let component = |size, ty| {
            (count(size), ComponentType::from_glenum(query(ty) as gl::types::GLenum))
        };
        // the client format of a pair of queries of a pixel format and type
        let client = |format, ty| {
            crate::ops::client_format_from_gl_enums(
                query(format) as gl::types::GLenum,
                query(ty) as gl::types::GLenum,
            )
        };
        let preferred = query(gl::INTERNALFORMAT_PREFERRED) as gl::types::GLenum;
        let preferred_format = TextureFormat::get_formats_list().into_iter().find(|candidate| {
            candidate.to_glenum() == preferred && preferred != format
        });
        let mut combined = [0i64];
        unsafe {
            ctxt.extra.GetInternalformati64v(
                target,
                format,
                gl::MAX_COMBINED_DIMENSIONS,
                1,
                combined.as_mut_ptr(),
            )
        };
        let compressed = query(gl::TEXTURE_COMPRESSED) != 0;
        let clear_texture = self.get_opengl_version() >= &Version(Api::Gl, 4, 4)
            || self.get_extensions().gl_arb_clear_texture;

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
            preferred_format,
            components: FormatComponents {
                red: component(gl::INTERNALFORMAT_RED_SIZE, gl::INTERNALFORMAT_RED_TYPE),
                green: component(gl::INTERNALFORMAT_GREEN_SIZE, gl::INTERNALFORMAT_GREEN_TYPE),
                blue: component(gl::INTERNALFORMAT_BLUE_SIZE, gl::INTERNALFORMAT_BLUE_TYPE),
                alpha: component(gl::INTERNALFORMAT_ALPHA_SIZE, gl::INTERNALFORMAT_ALPHA_TYPE),
                depth: component(gl::INTERNALFORMAT_DEPTH_SIZE, gl::INTERNALFORMAT_DEPTH_TYPE),
                stencil: component(
                    gl::INTERNALFORMAT_STENCIL_SIZE,
                    gl::INTERNALFORMAT_STENCIL_TYPE,
                ),
                shared_exponent: count(gl::INTERNALFORMAT_SHARED_SIZE),
            },
            has_components: (
                query(gl::COLOR_COMPONENTS) != 0,
                query(gl::DEPTH_COMPONENTS) != 0,
                query(gl::STENCIL_COMPONENTS) != 0,
            ),
            max_combined_dimensions: combined[0].max(0) as u64,
            framebuffer_renderable_layered: support(gl::FRAMEBUFFER_RENDERABLE_LAYERED),
            read_pixels: (
                support(gl::READ_PIXELS),
                client(gl::READ_PIXELS_FORMAT, gl::READ_PIXELS_TYPE),
            ),
            upload_format: client(gl::TEXTURE_IMAGE_FORMAT, gl::TEXTURE_IMAGE_TYPE),
            download_format: client(gl::GET_TEXTURE_IMAGE_FORMAT, gl::GET_TEXTURE_IMAGE_TYPE),
            manual_mipmap_generation: support(gl::MANUAL_GENERATE_MIPMAP),
            color_encoding: match query(gl::COLOR_ENCODING) as gl::types::GLenum {
                gl::LINEAR => Some(ColorEncoding::Linear),
                gl::SRGB => Some(ColorEncoding::Srgb),
                _ => None,
            },
            srgb: (support(gl::SRGB_READ), support(gl::SRGB_WRITE)),
            sampling: StageSupport {
                vertex: support(gl::VERTEX_TEXTURE),
                tessellation_control: support(gl::TESS_CONTROL_TEXTURE),
                tessellation_evaluation: support(gl::TESS_EVALUATION_TEXTURE),
                geometry: support(gl::GEOMETRY_TEXTURE),
                fragment: support(gl::FRAGMENT_TEXTURE),
                compute: support(gl::COMPUTE_TEXTURE),
            },
            shadow_and_gather: (
                support(gl::TEXTURE_SHADOW),
                support(gl::TEXTURE_GATHER),
                support(gl::TEXTURE_GATHER_SHADOW),
            ),
            image_texel_size: count(gl::IMAGE_TEXEL_SIZE),
            image_class: match query(gl::IMAGE_COMPATIBILITY_CLASS) as gl::types::GLenum {
                gl::IMAGE_CLASS_4_X_32 => Some(ImageFormatClass::FourX32),
                gl::IMAGE_CLASS_2_X_32 => Some(ImageFormatClass::TwoX32),
                gl::IMAGE_CLASS_1_X_32 => Some(ImageFormatClass::OneX32),
                gl::IMAGE_CLASS_4_X_16 => Some(ImageFormatClass::FourX16),
                gl::IMAGE_CLASS_2_X_16 => Some(ImageFormatClass::TwoX16),
                gl::IMAGE_CLASS_1_X_16 => Some(ImageFormatClass::OneX16),
                gl::IMAGE_CLASS_4_X_8 => Some(ImageFormatClass::FourX8),
                gl::IMAGE_CLASS_2_X_8 => Some(ImageFormatClass::TwoX8),
                gl::IMAGE_CLASS_1_X_8 => Some(ImageFormatClass::OneX8),
                gl::IMAGE_CLASS_11_11_10 => Some(ImageFormatClass::Bits11_11_10),
                gl::IMAGE_CLASS_10_10_10_2 => Some(ImageFormatClass::Bits10_10_10_2),
                _ => None,
            },
            image_pixel_format: client(gl::IMAGE_PIXEL_FORMAT, gl::IMAGE_PIXEL_TYPE),
            image_compatibility: match query(gl::IMAGE_FORMAT_COMPATIBILITY_TYPE)
                as gl::types::GLenum
            {
                gl::IMAGE_FORMAT_COMPATIBILITY_BY_SIZE => Some(ImageCompatibility::BySize),
                gl::IMAGE_FORMAT_COMPATIBILITY_BY_CLASS => Some(ImageCompatibility::ByClass),
                _ => None,
            },
            simultaneous: SimultaneousUse {
                depth_test: support(gl::SIMULTANEOUS_TEXTURE_AND_DEPTH_TEST),
                stencil_test: support(gl::SIMULTANEOUS_TEXTURE_AND_STENCIL_TEST),
                depth_write: support(gl::SIMULTANEOUS_TEXTURE_AND_DEPTH_WRITE),
                stencil_write: support(gl::SIMULTANEOUS_TEXTURE_AND_STENCIL_WRITE),
            },
            compressed: compressed.then(|| {
                match image_format::ClientFormatAny::from_internal_compressed_format(format)
                    .and_then(|known| known.get_buffer_size(4, Some(4), None, None))
                {
                    Some(bytes) => (4, 4, bytes as u32),
                    None => (
                        count(gl::TEXTURE_COMPRESSED_BLOCK_WIDTH),
                        count(gl::TEXTURE_COMPRESSED_BLOCK_HEIGHT),
                        count(gl::TEXTURE_COMPRESSED_BLOCK_SIZE),
                    ),
                }
            }),
            clear_buffer: support(gl::CLEAR_BUFFER),
            clear_texture: clear_texture.then(|| support(gl::CLEAR_TEXTURE)),
            views: (
                support(gl::TEXTURE_VIEW),
                ViewClass::from_glenum(query(gl::VIEW_COMPATIBILITY_CLASS) as gl::types::GLenum),
            ),
        }
    }
}
