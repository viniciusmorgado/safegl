use crate::gl;
use crate::GlObject;

use crate::backend::Facade;
use crate::context::CommandContext;
use crate::context::Context;
use crate::version::Api;
use crate::version::Version;
use crate::CapabilitiesSource;
use crate::ContextExt;
use crate::Rect;
use crate::TextureExt;
use crate::TextureMipmapExt;

use crate::image_format::{self, ClientFormatAny, CompressedFormat, TextureFormatRequest};
use crate::texture::pixel::PixelValue;
use crate::texture::pixel_buffer::PixelBuffer;
use crate::texture::Texture2dDataSink;
use crate::texture::TextureKind;
use crate::texture::{get_format, GetFormatError, InternalFormat};
use crate::texture::{CubeLayer, MipmapsOption, TextureCreationError, TextureFormat};
use crate::texture::{RawImage2d, Texture2dDataSource, UploadError};

use crate::fbo::ClearBufferData;

use crate::buffer::BufferAny;
use crate::buffer::BufferSlice;
use crate::BufferExt;
use crate::BufferSliceExt;

use std::borrow::Cow;
use std::cell::Cell;
use std::cmp;
use std::ffi::c_void;
use std::fmt;
use std::mem;
use std::ops::Range;
use std::rc::Rc;
use std::slice;

use glow::HasContext;

use crate::fbo;
use crate::ops;
use crate::ops::ReadError;

/// Type of a texture.
///
/// New kinds of textures can be added without a major version, so matches need a wildcard arm.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(missing_docs)] // TODO: document and remove
#[non_exhaustive]
pub enum Dimensions {
    Texture1d {
        width: u32,
    },
    Texture1dArray {
        width: u32,
        array_size: u32,
    },
    Texture2d {
        width: u32,
        height: u32,
    },
    /// A rectangle texture, sampled with coordinates in texels. Only desktop OpenGL has them.
    TextureRectangle {
        width: u32,
        height: u32,
    },
    Texture2dArray {
        width: u32,
        height: u32,
        array_size: u32,
    },
    Texture2dMultisample {
        width: u32,
        height: u32,
        samples: u32,
    },
    Texture2dMultisampleArray {
        width: u32,
        height: u32,
        array_size: u32,
        samples: u32,
    },
    Texture3d {
        width: u32,
        height: u32,
        depth: u32,
    },
    Cubemap {
        dimension: u32,
    },
    CubemapArray {
        dimension: u32,
        array_size: u32,
    },
}

/// A texture whose type isn't fixed at compile-time.
pub struct TextureAny {
    context: Rc<Context>,
    id: glow::Texture,
    requested_format: TextureFormatRequest,

    /// Cache for the actual format of the texture. The outer Option is None if the format hasn't
    /// been checked yet. The inner Result is Err if the format has been checked but is unknown.
    actual_format: Cell<Option<Result<InternalFormat, GetFormatError>>>,

    /// The internal format that SafeGL created the texture with, sized or not, or 0 for the
    /// textures that it didn't create.
    created_format: gl::types::GLenum,

    /// Type and dimensions of the texture.
    ty: Dimensions,

    /// Number of mipmap levels (`1` means just the main texture, `0` is not valid)
    levels: u32,
    /// Is automatic mipmap generation allowed for this texture?
    generate_mipmaps: bool,

    /// Whether the backend can generate the mipmaps of this texture.
    can_generate_mipmaps: bool,

    /// Is this texture owned by us? If not, we won't clean it up on drop.
    owned: bool,

    /// ID of the draw call where the texture was last written as a texture unit. Shared with the
    /// views of the same storage, which see the same writes.
    latest_shader_write: Rc<Cell<u64>>,

    /// The latest swizzle of the texture, `None` while unknown, for textures created outside
    /// SafeGL.
    pub(crate) swizzle: Cell<Option<[gl::types::GLenum; 4]>>,

    /// The latest `GL_TEXTURE_BASE_LEVEL` and `GL_TEXTURE_MAX_LEVEL` of the texture, `None` while
    /// unknown.
    pub(crate) mipmap_range: Cell<Option<(u32, u32)>>,

    /// The latest `GL_DEPTH_STENCIL_TEXTURE_MODE` of the texture, `None` while unknown.
    pub(crate) depth_stencil_mode: Cell<Option<gl::types::GLenum>>,

    /// Whether the samples of a multisample texture are at the same positions in every texel.
    fixed_sample_locations: bool,
}

fn extract_dimensions(ty: Dimensions) -> (u32, Option<u32>, Option<u32>, Option<u32>, Option<u32>) {
    match ty {
        Dimensions::Texture1d { width } => (width, None, None, None, None),
        Dimensions::Texture1dArray { width, array_size } => {
            (width, None, None, Some(array_size), None)
        }
        Dimensions::Texture2d { width, height } => (width, Some(height), None, None, None),
        Dimensions::TextureRectangle { width, height } => (width, Some(height), None, None, None),
        Dimensions::Texture2dArray {
            width,
            height,
            array_size,
        } => (width, Some(height), None, Some(array_size), None),
        Dimensions::Texture2dMultisample {
            width,
            height,
            samples,
        } => (width, Some(height), None, None, Some(samples)),
        Dimensions::Texture2dMultisampleArray {
            width,
            height,
            array_size,
            samples,
        } => (width, Some(height), None, Some(array_size), Some(samples)),
        Dimensions::Texture3d {
            width,
            height,
            depth,
        } => (width, Some(height), Some(depth), None, None),
        Dimensions::Cubemap { dimension } => (dimension, Some(dimension), None, None, None),
        Dimensions::CubemapArray {
            dimension,
            array_size,
        } => (dimension, Some(dimension), None, Some(array_size * 6), None),
    }
}

#[inline]
fn get_bind_point(ty: Dimensions) -> gl::types::GLenum {
    match ty {
        Dimensions::Texture1d { .. } => gl::TEXTURE_1D,
        Dimensions::Texture1dArray { .. } => gl::TEXTURE_1D_ARRAY,
        Dimensions::Texture2d { .. } => gl::TEXTURE_2D,
        Dimensions::TextureRectangle { .. } => gl::TEXTURE_RECTANGLE,
        Dimensions::Texture2dArray { .. } => gl::TEXTURE_2D_ARRAY,
        Dimensions::Texture2dMultisample { .. } => gl::TEXTURE_2D_MULTISAMPLE,
        Dimensions::Texture2dMultisampleArray { .. } => gl::TEXTURE_2D_MULTISAMPLE_ARRAY,
        Dimensions::Texture3d { .. } => gl::TEXTURE_3D,
        Dimensions::Cubemap { .. } => gl::TEXTURE_CUBE_MAP,
        Dimensions::CubemapArray { .. } => gl::TEXTURE_CUBE_MAP_ARRAY,
    }
}

/// Returns `TypeNotSupported` if the context has no textures of type `ty`, and
/// `DimensionsNotSupported` if the dimensions are above the context's limits.
fn check_dimensions(context: &Context, ty: Dimensions) -> Result<(), TextureCreationError> {
    let supported = match ty {
        Dimensions::Texture1d { .. } => super::ty_support::is_texture_1d_supported(context),
        Dimensions::Texture1dArray { .. } => {
            super::ty_support::is_texture_1d_array_supported(context)
        }
        Dimensions::Texture2dMultisample { .. } => {
            super::ty_support::is_texture_2d_multisample_supported(context)
        }
        Dimensions::Texture2dMultisampleArray { .. } => {
            super::ty_support::is_texture_2d_multisample_array_supported(context)
        }
        Dimensions::CubemapArray { .. } => super::ty_support::is_cubemap_arrays_supported(context),
        // core in OpenGL 3.1, absent from OpenGL ES
        Dimensions::TextureRectangle { .. } => context.get_opengl_version().0 == Api::Gl,
        _ => true,
    };

    if !supported {
        return Err(TextureCreationError::TypeNotSupported);
    }

    let capabilities = context.get_capabilities();
    // the limit for the width and height, and the one for the depth or number of layers
    let (max_size, max_depth) = match ty {
        Dimensions::Texture3d { .. } => (
            capabilities.max_3d_texture_size,
            capabilities.max_3d_texture_size,
        ),
        Dimensions::Cubemap { .. } | Dimensions::CubemapArray { .. } => (
            capabilities.max_cube_map_texture_size,
            capabilities.max_array_texture_layers,
        ),
        #[cfg(safegl_desktop)]
        Dimensions::TextureRectangle { .. } => (
            capabilities.limits.max_rectangle_texture_size.unwrap_or(0),
            capabilities.max_array_texture_layers,
        ),
        _ => (
            capabilities.max_texture_size,
            capabilities.max_array_texture_layers,
        ),
    };

    let fits = |value: Option<u32>, max: gl::types::GLint| match value {
        Some(value) => value <= cmp::max(max, 0) as u32,
        None => true,
    };

    let (width, height, depth, array_size, _) = extract_dimensions(ty);
    if !fits(Some(width), max_size)
        || !fits(height, max_size)
        || !fits(depth, max_depth)
        || !fits(array_size, max_depth)
    {
        return Err(TextureCreationError::DimensionsNotSupported);
    }

    Ok(())
}

/// Returns true if `glTexStorage1D`, `glTexStorage2D` and `glTexStorage3D` are available.
fn is_tex_storage_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version >= &Version(Api::Gl, 4, 2)
        || ctxt.version >= &Version(Api::GlEs, 3, 0)
        || ctxt.extensions.gl_arb_texture_storage
}

/// Returns true if `glTexStorage2DMultisample` is available.
fn is_tex_storage_multisample_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version >= &Version(Api::Gl, 4, 3)
        || ctxt.version >= &Version(Api::GlEs, 3, 1)
        || ctxt.extensions.gl_arb_texture_storage_multisample
}

/// Returns true if OpenGL ES can generate mipmaps for a texture of this internal format, which
/// must be unsized or both color-renderable and filterable.
fn is_es_mipmap_generation_supported(
    context: &Context,
    internal_format: gl::types::GLenum,
) -> bool {
    let float_renderable = context.get_opengl_version() >= &Version(Api::GlEs, 3, 2)
        || context.get_extensions().gl_ext_color_buffer_float;

    match internal_format {
        gl::RED | gl::RG | gl::RGB | gl::RGBA => true,
        gl::R8
        | gl::RG8
        | gl::RGB8
        | gl::RGB565
        | gl::RGBA4
        | gl::RGB5_A1
        | gl::RGBA8
        | gl::RGB10_A2
        | gl::SRGB8_ALPHA8 => true,
        gl::R16F | gl::RG16F | gl::RGBA16F => {
            float_renderable || context.get_extensions().gl_ext_color_buffer_half_float
        }
        gl::R11F_G11F_B10F => float_renderable,
        gl::R32F | gl::RG32F | gl::RGBA32F => {
            float_renderable && context.get_extensions().gl_oes_texture_float_linear
        }
        _ => false,
    }
}

unsafe fn generate_mipmaps(ctxt: &CommandContext<'_>, bind_point: gl::types::GLenum) {
    ctxt.glow.generate_mipmap(bind_point);
}

/// Returns `DimensionsNotSupported` if a multisample texture of `internal_format` can't have
/// `samples` samples: more than the largest count of the format, which OpenGL 4.2,
/// `GL_ARB_internalformat_query` and OpenGL ES 3.1 list, or than the limit of the kind of the
/// format (color, depth and stencil, or integer) otherwise. The driver would raise an error.
#[cfg(any(safegl_desktop, safegl_gles31))]
fn check_samples(
    context: &Context,
    target: gl::types::GLenum,
    internal_format: gl::types::GLenum,
    format: TextureFormatRequest,
    samples: u32,
) -> Result<(), TextureCreationError> {
    let version = context.get_opengl_version();
    let extensions = context.get_extensions();
    // OpenGL ES lists the samples of multisample arrays from 3.2 or with their extension
    let listed = !crate::version::is_webgl()
        && (version >= &Version(Api::Gl, 4, 2)
            || extensions.gl_arb_internalformat_query
            || version >= &Version(Api::GlEs, 3, 1))
        && (target == gl::TEXTURE_2D_MULTISAMPLE
            || version.0 == Api::Gl
            || version >= &Version(Api::GlEs, 3, 2)
            || extensions.gl_oes_texture_storage_multisample_2d_array);

    let max = if listed {
        let ctxt = context.make_current();
        unsafe {
            let mut count = [0];
            ctxt.glow.get_internal_format_i32_slice(
                target,
                internal_format,
                glow::NUM_SAMPLE_COUNTS,
                &mut count,
            );
            if count[0] > 0 {
                let mut counts = vec![0; count[0] as usize];
                ctxt.glow.get_internal_format_i32_slice(
                    target,
                    internal_format,
                    glow::SAMPLES,
                    &mut counts,
                );
                counts.into_iter().max().unwrap_or(0)
            } else {
                0
            }
        }
    } else {
        use crate::texture::TextureFormat as F;
        let limits = &context.get_capabilities().limits;
        let limit = match format {
            TextureFormatRequest::AnyDepth
            | TextureFormatRequest::AnyStencil
            | TextureFormatRequest::AnyDepthStencil
            | TextureFormatRequest::Specific(
                F::DepthFormat(_) | F::StencilFormat(_) | F::DepthStencilFormat(_),
            ) => limits.max_depth_texture_samples,
            TextureFormatRequest::AnyIntegral
            | TextureFormatRequest::AnyUnsigned
            | TextureFormatRequest::Specific(
                F::UncompressedIntegral(_) | F::UncompressedUnsigned(_),
            ) => limits.max_integer_samples,
            _ => limits.max_color_texture_samples,
        };
        limit.unwrap_or(0)
    };

    if samples == 0 || samples as gl::types::GLint > max {
        return Err(TextureCreationError::DimensionsNotSupported);
    }
    Ok(())
}

/// The targets of the images of a level of a texture bound at `bind_point`: the six faces of a
/// cube map, and the bind point itself otherwise.
pub(crate) fn image_targets(bind_point: gl::types::GLenum) -> Vec<gl::types::GLenum> {
    if bind_point == gl::TEXTURE_CUBE_MAP {
        (0..6).map(|face| gl::TEXTURE_CUBE_MAP_POSITIVE_X + face).collect()
    } else {
        vec![bind_point]
    }
}

/// Allocates the levels from 1 to `levels - 1` of a texture whose level 0 was allocated with
/// `glTexImage*`, which allocates a single level, so that a texture without texture storage is
/// complete. The levels are empty.
///
/// `size` is the width, the height and the depth or number of layers of level 0, as the texture
/// type gives them. The levels of compressed data are allocated with zeroed blocks, since the
/// compressed formats of client data can't be allocated without data everywhere.
unsafe fn allocate_upper_levels(
    ctxt: &CommandContext<'_>,
    bind_point: gl::types::GLenum,
    levels: u32,
    internal_format: gl::types::GLenum,
    size: (u32, Option<u32>, Option<u32>),
    (client_format, client_type): (gl::types::GLenum, gl::types::GLenum),
    compressed: Option<ClientFormatAny>,
) {
    let (width, height, depth) = size;
    let halve = |size: u32, level: u32| cmp::max(1, size >> level) as gl::types::GLsizei;
    // layers keep their number at every level
    let layered_height = bind_point == gl::TEXTURE_1D_ARRAY;
    let layered_depth =
        bind_point == gl::TEXTURE_2D_ARRAY || bind_point == gl::TEXTURE_CUBE_MAP_ARRAY;
    for level in 1..levels {
        let w = halve(width, level);
        let h = match height {
            Some(h) if layered_height => h as gl::types::GLsizei,
            Some(h) => halve(h, level),
            None => 1,
        };
        let d = match depth {
            Some(d) if layered_depth || layered_height => d as gl::types::GLsizei,
            Some(d) => halve(d, level),
            None => 1,
        };
        let level = level as i32;
        // zeroed blocks covering the level, in its layers or slices
        let blocks = compressed.map(|format| {
            let rows = if bind_point == gl::TEXTURE_1D || layered_height {
                1
            } else {
                h
            };
            let size = format.get_buffer_size(w as u32, Some(rows as u32), None, Some(d as u32));
            vec![0u8; size.unwrap_or(0)]
        });

        if bind_point == gl::TEXTURE_1D {
            match blocks {
                Some(ref zeroes) => ctxt.glow.compressed_tex_image_1d(
                    bind_point,
                    level,
                    internal_format as i32,
                    w,
                    0,
                    zeroes.len() as i32,
                    zeroes,
                ),
                None => ctxt.glow.tex_image_1d(
                    bind_point,
                    level,
                    internal_format as i32,
                    w,
                    0,
                    client_format,
                    client_type,
                    glow::PixelUnpackData::Slice(None),
                ),
            }
        } else if bind_point == gl::TEXTURE_3D || layered_depth {
            match blocks {
                Some(ref zeroes) => ctxt.glow.compressed_tex_image_3d(
                    bind_point,
                    level,
                    internal_format as i32,
                    w,
                    h,
                    d,
                    0,
                    zeroes.len() as i32,
                    zeroes,
                ),
                None => ctxt.glow.tex_image_3d(
                    bind_point,
                    level,
                    internal_format as i32,
                    w,
                    h,
                    d,
                    0,
                    client_format,
                    client_type,
                    glow::PixelUnpackData::Slice(None),
                ),
            }
        } else {
            // 2D, 1D arrays (whose layers are the height) and the faces of cube maps
            let h = if layered_height { d } else { h };
            for target in image_targets(bind_point) {
                match blocks {
                    Some(ref zeroes) => ctxt.glow.compressed_tex_image_2d(
                        target,
                        level,
                        internal_format as i32,
                        w,
                        h,
                        0,
                        zeroes.len() as i32,
                        zeroes,
                    ),
                    None => ctxt.glow.tex_image_2d(
                        target,
                        level,
                        internal_format as i32,
                        w,
                        h,
                        0,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::Slice(None),
                    ),
                }
            }
        }
    }
}

/// Builds a new texture.
pub fn new_texture<'a, F: ?Sized, P>(
    facade: &F,
    format: TextureFormatRequest,
    data: Option<(ClientFormatAny, Cow<'a, [P]>)>,
    mipmaps: MipmapsOption,
    ty: Dimensions,
) -> Result<TextureAny, TextureCreationError>
where
    P: Send + Clone + 'a,
    F: Facade,
{
    new_texture_with_sample_locations(facade, format, data, mipmaps, ty, true)
}

/// Builds a new texture, multisampled with fixed sample locations or not.
pub fn new_texture_with_sample_locations<'a, F: ?Sized, P>(
    facade: &F,
    format: TextureFormatRequest,
    data: Option<(ClientFormatAny, Cow<'a, [P]>)>,
    mipmaps: MipmapsOption,
    ty: Dimensions,
    fixed_sample_locations: bool,
) -> Result<TextureAny, TextureCreationError>
where
    P: Send + Clone + 'a,
    F: Facade,
{
    // compressed data gives its format to the textures of any compressed format, which then
    // have that format
    let format = match (format, data.as_ref().map(|&(client, _)| client)) {
        (TextureFormatRequest::AnyCompressed, Some(ClientFormatAny::CompressedFormat(f))) => {
            TextureFormatRequest::Specific(TextureFormat::CompressedFormat(f))
        }
        (
            TextureFormatRequest::AnyCompressedSrgb,
            Some(ClientFormatAny::CompressedSrgbFormat(f)),
        ) => TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(f)),
        (format, _) => format,
    };

    // getting the width, height, depth, array_size, samples from the type
    let (width, height, depth, array_size, samples) = extract_dimensions(ty);
    let (is_client_compressed, data_bufsize) = match data {
        Some((client_format, _)) => (
            client_format.is_compressed(),
            client_format
                .get_buffer_size(width, height, depth, array_size)
                .ok_or(TextureCreationError::FormatNotSupported)?,
        ),
        None => (false, 0),
    };

    if let Some((_, ref data)) = data {
        if data.len() * mem::size_of::<P>() != data_bufsize {
            return Err(TextureCreationError::DataSizeMismatch);
        }
    }

    // getting the `GLenum` corresponding to this texture type
    let bind_point = get_bind_point(ty);
    if bind_point == gl::TEXTURE_CUBE_MAP || bind_point == gl::TEXTURE_CUBE_MAP_ARRAY {
        assert!(data.is_none()); // TODO: not supported
    }
    check_dimensions(facade.get_context(), ty)?;

    // ETC1 and PVRTC textures have no texture storage: their levels are given whole, the
    // levels of PVRTC textures have power-of-two sizes
    let written_whole = match format {
        TextureFormatRequest::Specific(TextureFormat::CompressedFormat(format)) => {
            Some(ClientFormatAny::CompressedFormat(format)).filter(|f| f.is_written_whole())
        }
        _ => None,
    };
    let power_of_two = |size: Option<u32>| size.is_none_or(u32::is_power_of_two);
    let etc1 = ClientFormatAny::CompressedFormat(CompressedFormat::Etc1Rgb8);
    if written_whole.is_some_and(|f| f != etc1)
        && !(power_of_two(Some(width)) && power_of_two(height))
    {
        return Err(TextureCreationError::DimensionsNotSupported);
    }
    // the S3TC, RGTC and BPTC extensions of WebGL need a level 0 of whole blocks
    let compressed = match format {
        TextureFormatRequest::Specific(TextureFormat::CompressedFormat(format)) => {
            Some(ClientFormatAny::CompressedFormat(format))
        }
        TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(format)) => {
            Some(ClientFormatAny::CompressedSrgbFormat(format))
        }
        _ => None,
    };
    let whole_blocks = |size: Option<u32>| size.is_none_or(|size| size % 4 == 0);
    if cfg!(safegl_browser)
        && compressed.is_some_and(|f| f.needs_whole_blocks_on_webgl())
        && !(whole_blocks(Some(width)) && whole_blocks(height))
    {
        return Err(TextureCreationError::DimensionsNotSupported);
    }

    let should_generate_mipmaps = mipmaps.should_generate();
    let texture_levels = mipmaps.num_levels(width, height, depth) as gl::types::GLsizei;
    // rectangle textures have a single level
    if bind_point == gl::TEXTURE_RECTANGLE && texture_levels > 1 {
        return Err(TextureCreationError::MipmapsNotSupported);
    }

    let teximg_internal_format = image_format::format_request_to_glenum(
        facade.get_context(),
        format,
        image_format::RequestType::TexImage(data.as_ref().map(|&(c, _)| c)),
    )?;
    // OpenGL ES only uploads data whose components match the storage format, so the storage
    // format is chosen from the data there
    let storage_client = match facade.get_context().get_opengl_version().0 {
        Api::Gl => None,
        _ => data.as_ref().map(|&(c, _)| c),
    };
    let storage_internal_format = image_format::format_request_to_glenum(
        facade.get_context(),
        format,
        image_format::RequestType::TexStorage(storage_client),
    )
    .ok()
    .filter(|_| written_whole.is_none());

    let (client_format, client_type) = match (&data, format) {
        (&Some((client_format, _)), f) => {
            image_format::client_format_to_glenum(facade.get_context(), client_format, f, false)?
        }
        (&None, f) => image_format::default_client_format_to_glenum(f),
    };

    // OpenGL ES only accepts depth data whose type matches the internal format, so a depth
    // texture of any format initialized with floats or shorts gets the matching format
    let storage_internal_format = match storage_internal_format {
        Some(gl::DEPTH_COMPONENT24)
            if matches!(format, TextureFormatRequest::AnyDepth)
                && data.is_some()
                && facade.get_context().get_opengl_version().0 != Api::Gl =>
        {
            match client_type {
                gl::FLOAT => Some(gl::DEPTH_COMPONENT32F),
                gl::UNSIGNED_SHORT => Some(gl::DEPTH_COMPONENT16),
                _ => storage_internal_format,
            }
        }
        other => other,
    };

    let is_multisampled = matches!(
        ty,
        Dimensions::Texture2dMultisample { .. } | Dimensions::Texture2dMultisampleArray { .. }
    );

    // OpenGL ES only generates the mipmaps of formats that are color-renderable and filterable
    let can_generate_mipmaps = !is_multisampled
        && (facade.get_context().get_opengl_version().0 == Api::Gl
            || is_es_mipmap_generation_supported(
                facade.get_context(),
                storage_internal_format.unwrap_or(teximg_internal_format),
            ));
    if should_generate_mipmaps && !can_generate_mipmaps {
        return Err(TextureCreationError::MipmapsNotSupported);
    }

    // the sample count must be one that the format has
    #[cfg(any(safegl_desktop, safegl_gles31))]
    if let Some(samples) = samples {
        check_samples(
            facade.get_context(),
            bind_point,
            storage_internal_format.unwrap_or(teximg_internal_format),
            format,
            samples,
        )?;
    }

    // OpenGL ES can only create multisample textures with `glTexStorage*Multisample`
    if is_multisampled
        && storage_internal_format.is_none()
        && facade.get_context().get_opengl_version().0 != Api::Gl
    {
        return Err(TextureCreationError::FormatNotSupported);
    }

    let (filtering, mipmap_filtering) = match format {
        TextureFormatRequest::Specific(TextureFormat::UncompressedIntegral(_)) => {
            (gl::NEAREST, gl::NEAREST_MIPMAP_NEAREST)
        }
        TextureFormatRequest::Specific(TextureFormat::UncompressedUnsigned(_)) => {
            (gl::NEAREST, gl::NEAREST_MIPMAP_NEAREST)
        }
        TextureFormatRequest::Specific(TextureFormat::StencilFormat(_)) => {
            (gl::NEAREST, gl::NEAREST_MIPMAP_NEAREST)
        }
        TextureFormatRequest::AnyIntegral => (gl::NEAREST, gl::NEAREST_MIPMAP_NEAREST),
        TextureFormatRequest::AnyUnsigned => (gl::NEAREST, gl::NEAREST_MIPMAP_NEAREST),
        TextureFormatRequest::AnyStencil => (gl::NEAREST, gl::NEAREST_MIPMAP_NEAREST),
        _ => (gl::LINEAR, gl::LINEAR_MIPMAP_LINEAR),
    };

    let mut ctxt = facade.get_context().make_current();

    let id = unsafe {
        let has_mipmaps = texture_levels > 1;
        let data = data;
        // the size of `data` in bytes has been checked against `data_bufsize` above
        let data_raw: Option<&[u8]> = if let Some((_, ref data)) = data {
            Some(slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data_bufsize,
            ))
        } else {
            None
        };

        if ctxt.state.pixel_store_unpack_alignment != 1 {
            ctxt.state.pixel_store_unpack_alignment = 1;
            ctxt.glow.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        }

        BufferAny::unbind_pixel_unpack(&mut ctxt);

        // the objects of a lost context are gone, whether the browser creates them or not
        if ctxt.objects_lost() {
            return Err(TextureCreationError::ContextLost);
        }
        let id = ctxt
            .glow
            .create_texture()
            .map_err(|_| TextureCreationError::ContextLost)?;

        {
            ctxt.glow.bind_texture(bind_point, Some(id));
            let act = ctxt.state.active_texture as usize;
            ctxt.state.texture_units[act].texture = Some(id);
        }

        // rectangle textures can only be clamped
        let wrap = if bind_point == gl::TEXTURE_RECTANGLE {
            gl::CLAMP_TO_EDGE
        } else {
            gl::REPEAT
        };
        if !is_multisampled {
            ctxt.glow
                .tex_parameter_i32(bind_point, glow::TEXTURE_WRAP_S, wrap as i32);
            ctxt.glow
                .tex_parameter_i32(bind_point, glow::TEXTURE_MAG_FILTER, filtering as i32);
        }

        match ty {
            Dimensions::Texture1d { .. } => (),
            Dimensions::Texture2dMultisample { .. } => (),
            Dimensions::Texture2dMultisampleArray { .. } => (),
            _ => {
                ctxt.glow
                    .tex_parameter_i32(bind_point, glow::TEXTURE_WRAP_T, wrap as i32);
            }
        };

        // multisample textures have no sampler state
        match ty {
            Dimensions::Texture1d { .. } => (),
            Dimensions::Texture2d { .. } => (),
            Dimensions::TextureRectangle { .. } => (),
            Dimensions::Texture2dMultisample { .. } => (),
            Dimensions::Texture2dMultisampleArray { .. } => (),
            _ => {
                ctxt.glow
                    .tex_parameter_i32(bind_point, glow::TEXTURE_WRAP_R, gl::REPEAT as i32);
            }
        };

        if has_mipmaps {
            ctxt.glow.tex_parameter_i32(
                bind_point,
                glow::TEXTURE_MIN_FILTER,
                mipmap_filtering as i32,
            );
        } else if !is_multisampled {
            ctxt.glow
                .tex_parameter_i32(bind_point, glow::TEXTURE_MIN_FILTER, filtering as i32);
        }

        if !has_mipmaps {
            ctxt.glow
                .tex_parameter_i32(bind_point, glow::TEXTURE_BASE_LEVEL, 0);
            ctxt.glow
                .tex_parameter_i32(bind_point, glow::TEXTURE_MAX_LEVEL, 0);
        }

        if bind_point == gl::TEXTURE_3D
            || bind_point == gl::TEXTURE_2D_ARRAY
            || bind_point == gl::TEXTURE_CUBE_MAP_ARRAY
        {
            let mut data_raw = data_raw;

            let width = match width as gl::types::GLsizei {
                0 => {
                    data_raw = None;
                    1
                }
                a => a,
            };

            let height = match height.unwrap() as gl::types::GLsizei {
                0 => {
                    data_raw = None;
                    1
                }
                a => a,
            };

            let depth = match depth.or(array_size).unwrap() as gl::types::GLsizei {
                0 => {
                    data_raw = None;
                    1
                }
                a => a,
            };

            if storage_internal_format.is_some() && is_tex_storage_supported(&ctxt) {
                ctxt.glow.tex_storage_3d(
                    bind_point,
                    texture_levels,
                    storage_internal_format.unwrap() as gl::types::GLenum,
                    width,
                    height,
                    depth,
                );

                if let Some(data_raw) = data_raw {
                    if is_client_compressed {
                        ctxt.glow.compressed_tex_sub_image_3d(
                            bind_point,
                            0,
                            0,
                            0,
                            0,
                            width,
                            height,
                            depth,
                            teximg_internal_format as u32,
                            glow::CompressedPixelUnpackData::Slice(data_raw),
                        );
                    } else {
                        ctxt.glow.tex_sub_image_3d(
                            bind_point,
                            0,
                            0,
                            0,
                            0,
                            width,
                            height,
                            depth,
                            client_format,
                            client_type,
                            glow::PixelUnpackData::Slice(Some(data_raw)),
                        );
                    }
                }
            } else if let (true, Some(data_raw)) = (is_client_compressed, data_raw) {
                ctxt.glow.compressed_tex_image_3d(
                    bind_point,
                    0,
                    teximg_internal_format as i32,
                    width,
                    height,
                    depth,
                    0,
                    data_bufsize as i32,
                    data_raw,
                );
            } else {
                ctxt.glow.tex_image_3d(
                    bind_point,
                    0,
                    teximg_internal_format as i32,
                    width,
                    height,
                    depth,
                    0,
                    client_format as u32,
                    client_type,
                    glow::PixelUnpackData::Slice(data_raw),
                );
            }
        } else if bind_point == gl::TEXTURE_2D
            || bind_point == gl::TEXTURE_RECTANGLE
            || bind_point == gl::TEXTURE_1D_ARRAY
            || bind_point == gl::TEXTURE_CUBE_MAP
        {
            let mut data_raw = data_raw;

            let width = match width as gl::types::GLsizei {
                0 => {
                    data_raw = None;
                    1
                }
                a => a,
            };

            let height = match height.or(array_size).unwrap() as gl::types::GLsizei {
                0 => {
                    data_raw = None;
                    1
                }
                a => a,
            };

            if storage_internal_format.is_some() && is_tex_storage_supported(&ctxt) {
                ctxt.glow.tex_storage_2d(
                    bind_point,
                    texture_levels,
                    storage_internal_format.unwrap() as gl::types::GLenum,
                    width,
                    height,
                );

                if let Some(data_raw) = data_raw {
                    if is_client_compressed {
                        ctxt.glow.compressed_tex_sub_image_2d(
                            bind_point,
                            0,
                            0,
                            0,
                            width,
                            height,
                            teximg_internal_format as u32,
                            glow::CompressedPixelUnpackData::Slice(data_raw),
                        );
                    } else {
                        ctxt.glow.tex_sub_image_2d(
                            bind_point,
                            0,
                            0,
                            0,
                            width,
                            height,
                            client_format,
                            client_type,
                            glow::PixelUnpackData::Slice(Some(data_raw)),
                        );
                    }
                }
            } else if let (true, Some(data_raw)) = (is_client_compressed, data_raw) {
                ctxt.glow.compressed_tex_image_2d(
                    bind_point,
                    0,
                    teximg_internal_format as i32,
                    width,
                    height,
                    0,
                    data_bufsize as i32,
                    data_raw,
                );
            } else if let Some(format) = written_whole {
                // an empty ETC1 or PVRTC level is made of zeroed blocks
                let size = format.get_buffer_size(width as u32, Some(height as u32), None, None);
                let zeroes = vec![0u8; size.unwrap_or(0)];
                for target in image_targets(bind_point) {
                    ctxt.glow.compressed_tex_image_2d(
                        target,
                        0,
                        teximg_internal_format as i32,
                        width,
                        height,
                        0,
                        zeroes.len() as i32,
                        &zeroes,
                    );
                }
            } else {
                // the six faces of a cube map are separate images
                for target in image_targets(bind_point) {
                    ctxt.glow.tex_image_2d(
                        target,
                        0,
                        teximg_internal_format as i32,
                        width,
                        height,
                        0,
                        client_format as u32,
                        client_type,
                        glow::PixelUnpackData::Slice(data_raw),
                    );
                }
            }
        } else if bind_point == gl::TEXTURE_2D_MULTISAMPLE {
            assert!(data_raw.is_none());

            let width = match width as gl::types::GLsizei {
                0 => 1,
                a => a,
            };

            let height = match height.unwrap() as gl::types::GLsizei {
                0 => 1,
                a => a,
            };

            // `glTexStorage2DMultisample` is the only way to create one on OpenGL ES 3.1+
            if storage_internal_format.is_some() && is_tex_storage_multisample_supported(&ctxt) {
                ctxt.glow.tex_storage_2d_multisample(
                    glow::TEXTURE_2D_MULTISAMPLE,
                    samples.unwrap() as gl::types::GLsizei,
                    storage_internal_format.unwrap() as gl::types::GLenum,
                    width,
                    height,
                    fixed_sample_locations,
                );
            } else if ctxt.version.0 == Api::Gl {
                ctxt.glow.tex_image_2d_multisample(
                    glow::TEXTURE_2D_MULTISAMPLE,
                    samples.unwrap() as gl::types::GLsizei,
                    teximg_internal_format as i32,
                    width,
                    height,
                    fixed_sample_locations,
                );
            } else {
                unreachable!();
            }
        } else if bind_point == gl::TEXTURE_2D_MULTISAMPLE_ARRAY {
            assert!(data_raw.is_none());

            let width = match width as gl::types::GLsizei {
                0 => 1,
                a => a,
            };

            let height = match height.unwrap() as gl::types::GLsizei {
                0 => 1,
                a => a,
            };

            if storage_internal_format.is_some()
                && (ctxt.version >= &Version(Api::Gl, 4, 3)
                    || ctxt.version >= &Version(Api::GlEs, 3, 2)
                    || ctxt.extensions.gl_arb_texture_storage_multisample)
            {
                ctxt.extra.TexStorage3DMultisample(
                    gl::TEXTURE_2D_MULTISAMPLE_ARRAY,
                    samples.unwrap() as gl::types::GLsizei,
                    storage_internal_format.unwrap() as gl::types::GLenum,
                    width,
                    height,
                    array_size.unwrap() as gl::types::GLsizei,
                    fixed_sample_locations as u8,
                );
            } else if storage_internal_format.is_some()
                && ctxt.extensions.gl_oes_texture_storage_multisample_2d_array
            {
                ctxt.extra.TexStorage3DMultisampleOES(
                    gl::TEXTURE_2D_MULTISAMPLE_ARRAY,
                    samples.unwrap() as gl::types::GLsizei,
                    storage_internal_format.unwrap() as gl::types::GLenum,
                    width,
                    height,
                    array_size.unwrap() as gl::types::GLsizei,
                    fixed_sample_locations as u8,
                );
            } else if ctxt.version.0 == Api::Gl {
                ctxt.extra.TexImage3DMultisample(
                    gl::TEXTURE_2D_MULTISAMPLE_ARRAY,
                    samples.unwrap() as gl::types::GLsizei,
                    teximg_internal_format as gl::types::GLenum,
                    width,
                    height,
                    array_size.unwrap() as gl::types::GLsizei,
                    fixed_sample_locations as u8,
                );
            } else {
                unreachable!();
            }
        } else if bind_point == gl::TEXTURE_1D {
            let mut data_raw = data_raw;

            let width = match width as gl::types::GLsizei {
                0 => {
                    data_raw = None;
                    1
                }
                a => a,
            };

            if storage_internal_format.is_some() && is_tex_storage_supported(&ctxt) {
                ctxt.glow.tex_storage_1d(
                    bind_point,
                    texture_levels,
                    storage_internal_format.unwrap() as gl::types::GLenum,
                    width,
                );

                if let Some(data_raw) = data_raw {
                    if is_client_compressed {
                        ctxt.extra.CompressedTexSubImage1D(
                            bind_point,
                            0,
                            0,
                            width,
                            teximg_internal_format as u32,
                            data_bufsize as i32,
                            data_raw.as_ptr() as *const c_void,
                        );
                    } else {
                        ctxt.extra.TexSubImage1D(
                            bind_point,
                            0,
                            0,
                            width,
                            client_format,
                            client_type,
                            data_raw.as_ptr() as *const c_void,
                        );
                    }
                }
            } else if let (true, Some(data_raw)) = (is_client_compressed, data_raw) {
                ctxt.glow.compressed_tex_image_1d(
                    bind_point,
                    0,
                    teximg_internal_format as i32,
                    width,
                    0,
                    data_bufsize as i32,
                    data_raw,
                );
            } else {
                ctxt.glow.tex_image_1d(
                    bind_point,
                    0,
                    teximg_internal_format as i32,
                    width,
                    0,
                    client_format as u32,
                    client_type,
                    glow::PixelUnpackData::Slice(data_raw),
                );
            }
        } else {
            unreachable!();
        }

        // `glTexImage*` allocates one level: without texture storage, the other levels are
        // allocated one by one, empty, and the last level is the one that the texture has, so
        // that it is complete with fewer levels than down to 1x1, and that mipmap generation
        // stops there
        let has_storage = storage_internal_format.is_some() && is_tex_storage_supported(&ctxt);
        if texture_levels > 1 && !has_storage {
            let max_level = texture_levels - 1;
            ctxt.glow
                .tex_parameter_i32(bind_point, glow::TEXTURE_MAX_LEVEL, max_level);
        }
        if texture_levels > 1 && !should_generate_mipmaps && !has_storage {
            let compressed = data.as_ref().map(|&(c, _)| c).filter(|c| c.is_compressed());
            allocate_upper_levels(
                &ctxt,
                bind_point,
                texture_levels as u32,
                teximg_internal_format,
                (width, height, depth.or(array_size)),
                (client_format, client_type),
                compressed,
            );
        }

        // only generate mipmaps for color textures
        if should_generate_mipmaps {
            generate_mipmaps(&ctxt, bind_point);
        }

        id
    };

    let has_storage = storage_internal_format.is_some() && is_tex_storage_supported(&ctxt);
    let mipmap_max = match texture_levels {
        1 => 0,
        levels if !has_storage => levels as u32 - 1,
        _ => 1000,
    };
    Ok(TextureAny {
        context: facade.get_context().clone(),
        id,
        requested_format: format,
        actual_format: Cell::new(None),
        created_format: storage_internal_format.unwrap_or(teximg_internal_format),
        ty,
        levels: texture_levels as u32,
        generate_mipmaps: should_generate_mipmaps,
        can_generate_mipmaps,
        owned: true,
        latest_shader_write: Rc::new(Cell::new(0)),
        swizzle: Cell::new(Some([gl::RED, gl::GREEN, gl::BLUE, gl::ALPHA])),
        // the defaults of OpenGL, or the range set above for textures without mipmaps or
        // texture storage
        mipmap_range: Cell::new(Some((0, mipmap_max))),
        depth_stencil_mode: Cell::new(Some(gl::DEPTH_COMPONENT)),
        fixed_sample_locations,
    })
}

/// Builds a view of the levels `levels` and the layers `layers` of `source`, of the kind whose
/// bind point is `target`, with the format `format` (`glTextureView`).
#[cfg(safegl_gl43)]
pub fn new_texture_view(
    source: &TextureAny,
    target: gl::types::GLenum,
    format: TextureFormatRequest,
    levels: std::ops::Range<u32>,
    layers: std::ops::Range<u32>,
) -> Result<TextureAny, super::TextureViewError> {
    use super::view::{formats_compatible, targets_compatible};
    use super::TextureViewError;
    use crate::handles::RawName;

    let context = &source.context;
    let mut ctxt = context.make_current();
    if !(ctxt.version >= &Version(Api::Gl, 4, 3) || ctxt.extensions.gl_arb_texture_view) {
        return Err(TextureViewError::NotSupported);
    }
    let source_target = source.get_bind_point();
    if !targets_compatible(source_target, target) {
        return Err(TextureViewError::IncompatibleTarget);
    }

    // the levels, and the layers, which are the faces of cube maps
    if levels.is_empty() || levels.end > source.levels {
        return Err(TextureViewError::LevelsOutOfRange);
    }
    let source_layers = match source.ty {
        Dimensions::Texture1dArray { array_size, .. }
        | Dimensions::Texture2dArray { array_size, .. }
        | Dimensions::Texture2dMultisampleArray { array_size, .. } => array_size,
        Dimensions::Cubemap { .. } => 6,
        Dimensions::CubemapArray { array_size, .. } => 6 * array_size,
        _ => 1,
    };
    let count = layers.len() as u32;
    let fits_target = match target {
        gl::TEXTURE_1D_ARRAY | gl::TEXTURE_2D_ARRAY | gl::TEXTURE_2D_MULTISAMPLE_ARRAY => true,
        gl::TEXTURE_CUBE_MAP => count == 6,
        gl::TEXTURE_CUBE_MAP_ARRAY => count % 6 == 0,
        _ => count == 1,
    };
    if layers.is_empty() || layers.end > source_layers || !fits_target {
        return Err(TextureViewError::LayersOutOfRange);
    }

    // the formats, and the immutable storage that views need
    let view_format = image_format::format_request_to_glenum(
        context,
        format,
        image_format::RequestType::TexStorage(None),
    )
    .map_err(|_| TextureViewError::FormatNotSupported)?;
    let (source_format, immutable) = unsafe {
        let bind_point = source.bind_to_current(&mut ctxt);
        let face = if bind_point == gl::TEXTURE_CUBE_MAP {
            gl::TEXTURE_CUBE_MAP_POSITIVE_X
        } else {
            bind_point
        };
        (
            ctxt.glow
                .get_tex_level_parameter_i32(face, 0, gl::TEXTURE_INTERNAL_FORMAT)
                as gl::types::GLenum,
            ctxt.glow
                .get_tex_parameter_i32(bind_point, gl::TEXTURE_IMMUTABLE_FORMAT)
                != 0,
        )
    };
    if !immutable {
        return Err(TextureViewError::SourceNotImmutable);
    }
    if !formats_compatible(source_format, view_format) {
        return Err(TextureViewError::IncompatibleFormat);
    }

    // the view's level 0 is the source's level `levels.start`
    let size = |size: u32| cmp::max(1, size >> levels.start);
    let (width, height, depth, _, samples) = extract_dimensions(source.ty);
    let (width, height) = (size(width), height.map(size).unwrap_or(1));
    let ty = match target {
        gl::TEXTURE_1D => Dimensions::Texture1d { width },
        gl::TEXTURE_1D_ARRAY => Dimensions::Texture1dArray {
            width,
            array_size: count,
        },
        gl::TEXTURE_2D => Dimensions::Texture2d { width, height },
        gl::TEXTURE_RECTANGLE => Dimensions::TextureRectangle { width, height },
        gl::TEXTURE_2D_ARRAY => Dimensions::Texture2dArray {
            width,
            height,
            array_size: count,
        },
        gl::TEXTURE_2D_MULTISAMPLE => Dimensions::Texture2dMultisample {
            width,
            height,
            samples: samples.unwrap_or(1),
        },
        gl::TEXTURE_2D_MULTISAMPLE_ARRAY => Dimensions::Texture2dMultisampleArray {
            width,
            height,
            array_size: count,
            samples: samples.unwrap_or(1),
        },
        gl::TEXTURE_3D => Dimensions::Texture3d {
            width,
            height,
            depth: size(depth.unwrap_or(1)),
        },
        gl::TEXTURE_CUBE_MAP => Dimensions::Cubemap { dimension: width },
        gl::TEXTURE_CUBE_MAP_ARRAY => Dimensions::CubemapArray {
            dimension: width,
            array_size: count / 6,
        },
        _ => return Err(TextureViewError::IncompatibleTarget),
    };

    let id = unsafe {
        // `glTextureView` needs a name that was never bound
        let id = ctxt.glow.create_texture().expect("failed to create a texture");
        ctxt.extra.TextureView(
            id.raw_name(),
            target,
            source.id.raw_name(),
            view_format,
            levels.start,
            levels.len() as u32,
            layers.start,
            count,
        );

        // the sampling state of the view is set like the one of a new texture
        let integer = matches!(
            format,
            TextureFormatRequest::Specific(TextureFormat::UncompressedIntegral(_))
                | TextureFormatRequest::Specific(TextureFormat::UncompressedUnsigned(_))
                | TextureFormatRequest::Specific(TextureFormat::StencilFormat(_))
        );
        let filter = if integer { gl::NEAREST } else { gl::LINEAR };
        let mipmap_filter = if integer {
            gl::NEAREST_MIPMAP_NEAREST
        } else {
            gl::LINEAR_MIPMAP_LINEAR
        };
        ctxt.glow.bind_texture(target, Some(id));
        let unit = ctxt.state.active_texture as usize;
        ctxt.state.texture_units[unit].texture = Some(id);
        // the parameters that SafeGL caches start from their defaults, whatever the source has
        for (name, value) in [
            (gl::TEXTURE_SWIZZLE_R, gl::RED),
            (gl::TEXTURE_SWIZZLE_G, gl::GREEN),
            (gl::TEXTURE_SWIZZLE_B, gl::BLUE),
            (gl::TEXTURE_SWIZZLE_A, gl::ALPHA),
            (gl::DEPTH_STENCIL_TEXTURE_MODE, gl::DEPTH_COMPONENT),
        ] {
            ctxt.glow.tex_parameter_i32(target, name, value as i32);
        }
        if samples.is_none() {
            ctxt.glow.tex_parameter_i32(target, gl::TEXTURE_BASE_LEVEL, 0);
            ctxt.glow
                .tex_parameter_i32(target, gl::TEXTURE_MAX_LEVEL, levels.len() as i32 - 1);
            let wrap = if target == gl::TEXTURE_RECTANGLE {
                gl::CLAMP_TO_EDGE
            } else {
                gl::REPEAT
            };
            for name in [gl::TEXTURE_WRAP_S, gl::TEXTURE_WRAP_T, gl::TEXTURE_WRAP_R] {
                ctxt.glow.tex_parameter_i32(target, name, wrap as i32);
            }
            ctxt.glow
                .tex_parameter_i32(target, gl::TEXTURE_MAG_FILTER, filter as i32);
            let min_filter = if levels.len() > 1 { mipmap_filter } else { filter };
            ctxt.glow
                .tex_parameter_i32(target, gl::TEXTURE_MIN_FILTER, min_filter as i32);
        }
        id
    };

    Ok(TextureAny {
        context: context.clone(),
        id,
        requested_format: format,
        actual_format: Cell::new(None),
        created_format: view_format,
        ty,
        levels: levels.len() as u32,
        generate_mipmaps: false,
        can_generate_mipmaps: source.can_generate_mipmaps,
        owned: true,
        // the view and its source see the shader writes of each other
        latest_shader_write: source.latest_shader_write.clone(),
        swizzle: Cell::new(Some([gl::RED, gl::GREEN, gl::BLUE, gl::ALPHA])),
        mipmap_range: Cell::new(Some((0, levels.len() as u32 - 1))),
        depth_stencil_mode: Cell::new(Some(gl::DEPTH_COMPONENT)),
        fixed_sample_locations: source.fixed_sample_locations,
    })
}

/// Builds a new texture reference from an existing, externally created OpenGL texture.
/// If `owned` is true, this reference will take ownership of the texture and be responsible
/// for cleaning it up. Otherwise, the texture must be cleaned up externally, but only
/// after this reference's lifetime has ended.
pub unsafe fn from_id<F: Facade + ?Sized>(
    facade: &F,
    format: TextureFormatRequest,
    id: glow::Texture,
    owned: bool,
    mipmaps: MipmapsOption,
    ty: Dimensions,
) -> TextureAny {
    let (width, height, depth, array_size, samples) = extract_dimensions(ty);
    let mipmap_levels = mipmaps.num_levels(width, height, depth);
    let should_generate_mipmaps = mipmaps.should_generate();
    let texture = TextureAny {
        context: facade.get_context().clone(),
        id,
        requested_format: format,
        actual_format: Cell::new(None),
        created_format: 0,
        ty,
        levels: mipmap_levels,
        generate_mipmaps: should_generate_mipmaps,
        // the format of a texture created outside SafeGL isn't known
        can_generate_mipmaps: !matches!(
            ty,
            Dimensions::Texture2dMultisample { .. } | Dimensions::Texture2dMultisampleArray { .. }
        ),
        owned,
        latest_shader_write: Rc::new(Cell::new(0)),
        swizzle: Cell::new(None),
        mipmap_range: Cell::new(None),
        depth_stencil_mode: Cell::new(None),
        // the most common choice, which can't be queried below OpenGL ES 3.1
        fixed_sample_locations: true,
    };

    if should_generate_mipmaps {
        let mut ctxt = facade.get_context().make_current();
        // the mipmaps are generated for the texture bound to the bind point
        let bind_point = texture.bind_to_current(&mut ctxt);
        generate_mipmaps(&ctxt, bind_point);
    }

    texture
}

impl TextureAny {
    /// Returns the draw call that last wrote the texture through an image unit, shared with its
    /// views.
    #[cfg(safegl_desktop)]
    pub(crate) fn latest_shader_write(&self) -> Rc<Cell<u64>> {
        self.latest_shader_write.clone()
    }

    /// Returns the width of the texture.
    #[inline]
    pub fn get_width(&self) -> u32 {
        match self.ty {
            Dimensions::Texture1d { width, .. } => width,
            Dimensions::Texture1dArray { width, .. } => width,
            Dimensions::Texture2d { width, .. } => width,
            Dimensions::TextureRectangle { width, .. } => width,
            Dimensions::Texture2dArray { width, .. } => width,
            Dimensions::Texture2dMultisample { width, .. } => width,
            Dimensions::Texture2dMultisampleArray { width, .. } => width,
            Dimensions::Texture3d { width, .. } => width,
            Dimensions::Cubemap { dimension, .. } => dimension,
            Dimensions::CubemapArray { dimension, .. } => dimension,
        }
    }

    /// Returns the height of the texture.
    #[inline]
    pub fn get_height(&self) -> Option<u32> {
        match self.ty {
            Dimensions::Texture1d { .. } => None,
            Dimensions::Texture1dArray { .. } => None,
            Dimensions::Texture2d { height, .. } => Some(height),
            Dimensions::TextureRectangle { height, .. } => Some(height),
            Dimensions::Texture2dArray { height, .. } => Some(height),
            Dimensions::Texture2dMultisample { height, .. } => Some(height),
            Dimensions::Texture2dMultisampleArray { height, .. } => Some(height),
            Dimensions::Texture3d { height, .. } => Some(height),
            Dimensions::Cubemap { dimension, .. } => Some(dimension),
            Dimensions::CubemapArray { dimension, .. } => Some(dimension),
        }
    }

    /// Returns the depth of the texture.
    #[inline]
    pub fn get_depth(&self) -> Option<u32> {
        match self.ty {
            Dimensions::Texture3d { depth, .. } => Some(depth),
            _ => None,
        }
    }

    /// The internal format that SafeGL created the texture with, sized or not, or 0 for the
    /// textures that it didn't create.
    pub(crate) fn created_format(&self) -> gl::types::GLenum {
        self.created_format
    }

    /// Returns the initial requested format.
    #[inline]
    #[doc(hidden)]
    pub fn get_requested_format(&self) -> TextureFormatRequest {
        self.requested_format
    }

    /// Returns the kind of texture.
    #[inline]
    pub fn kind(&self) -> TextureKind {
        match self.requested_format {
            TextureFormatRequest::Specific(TextureFormat::UncompressedFloat(_)) => {
                TextureKind::Float
            }
            TextureFormatRequest::Specific(TextureFormat::UncompressedIntegral(_)) => {
                TextureKind::Integral
            }
            TextureFormatRequest::Specific(TextureFormat::UncompressedUnsigned(_)) => {
                TextureKind::Unsigned
            }
            TextureFormatRequest::Specific(TextureFormat::Srgb(_)) => TextureKind::Float,
            TextureFormatRequest::Specific(TextureFormat::CompressedFormat(_)) => {
                TextureKind::Float
            }
            TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(_)) => {
                TextureKind::Float
            }
            TextureFormatRequest::Specific(TextureFormat::DepthFormat(_)) => TextureKind::Depth,
            TextureFormatRequest::Specific(TextureFormat::StencilFormat(_)) => TextureKind::Stencil,
            TextureFormatRequest::Specific(TextureFormat::DepthStencilFormat(_)) => {
                TextureKind::DepthStencil
            }
            TextureFormatRequest::AnyFloatingPoint => TextureKind::Float,
            TextureFormatRequest::AnyCompressed => TextureKind::Float,
            TextureFormatRequest::AnySrgb => TextureKind::Float,
            TextureFormatRequest::AnyCompressedSrgb => TextureKind::Float,
            TextureFormatRequest::AnyIntegral => TextureKind::Integral,
            TextureFormatRequest::AnyUnsigned => TextureKind::Unsigned,
            TextureFormatRequest::AnyDepth => TextureKind::Depth,
            TextureFormatRequest::AnyStencil => TextureKind::Stencil,
            TextureFormatRequest::AnyDepthStencil => TextureKind::DepthStencil,
        }
    }

    /// Returns the dimensions of the texture.
    #[inline]
    pub fn dimensions(&self) -> Dimensions {
        self.ty
    }

    /// Returns the array size of the texture.
    #[inline]
    pub fn get_array_size(&self) -> Option<u32> {
        match self.ty {
            Dimensions::Texture1d { .. } => None,
            Dimensions::Texture1dArray { array_size, .. } => Some(array_size),
            Dimensions::Texture2d { .. } => None,
            Dimensions::TextureRectangle { .. } => None,
            Dimensions::Texture2dArray { array_size, .. } => Some(array_size),
            Dimensions::Texture2dMultisample { .. } => None,
            Dimensions::Texture2dMultisampleArray { array_size, .. } => Some(array_size),
            Dimensions::Texture3d { .. } => None,
            Dimensions::Cubemap { .. } => None,
            Dimensions::CubemapArray { array_size, .. } => Some(array_size),
        }
    }

    /// Returns true if the samples of a multisample texture are at the same positions in every
    /// texel (`GL_TEXTURE_FIXED_SAMPLE_LOCATIONS`), which is the default. Returns true for other
    /// textures too.
    #[inline]
    pub fn has_fixed_sample_locations(&self) -> bool {
        self.fixed_sample_locations
    }

    /// Returns the number of samples of the texture if it is a multisampling texture.
    #[inline]
    pub fn get_samples(&self) -> Option<u32> {
        match self.ty {
            Dimensions::Texture2dMultisample { samples, .. } => Some(samples),
            Dimensions::Texture2dMultisampleArray { samples, .. } => Some(samples),
            _ => None,
        }
    }

    /// Returns a structure that represents the first layer of the texture. All textures have a
    /// first layer.
    #[inline]
    pub fn first_layer(&self) -> TextureAnyLayer<'_> {
        self.layer(0).unwrap()
    }

    /// Returns a structure that represents a specific layer of the texture.
    ///
    /// Non-array textures have only one layer. The number of layers can be queried with
    /// `get_array_size`.
    ///
    /// Returns `None` if out of range.
    #[inline]
    pub fn layer(&self, layer: u32) -> Option<TextureAnyLayer<'_>> {
        if layer >= self.get_array_size().unwrap_or(1) {
            return None;
        }

        Some(TextureAnyLayer {
            texture: self,
            layer,
        })
    }

    /// Returns the type of the texture (1D, 2D, 3D, etc.).
    #[inline]
    pub fn get_texture_type(&self) -> Dimensions {
        self.ty
    }

    /// Determines the internal format of this texture.
    #[inline]
    pub fn get_internal_format(&self) -> Result<InternalFormat, GetFormatError> {
        if let Some(format) = self.actual_format.get() {
            format
        } else {
            self.get_internal_format_with(&mut self.context.make_current())
        }
    }

    /// Same as `get_internal_format`, with the context already current.
    pub(crate) fn get_internal_format_with(
        &self,
        ctxt: &mut CommandContext<'_>,
    ) -> Result<InternalFormat, GetFormatError> {
        if let Some(format) = self.actual_format.get() {
            format
        } else {
            let format = get_format::get_format(ctxt, self);
            self.actual_format.set(Some(format));
            format
        }
    }

    /// Determines the number of depth and stencil bits in the format of this texture.
    pub fn get_depth_stencil_bits(&self) -> (u16, u16) {
        self.get_depth_stencil_bits_with(&mut self.context.make_current())
    }

    /// Same as `get_depth_stencil_bits`, with the context already current.
    pub(crate) fn get_depth_stencil_bits_with(&self, ctxt: &mut CommandContext<'_>) -> (u16, u16) {
        if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access {
            unsafe {
                let texture = self.id;
                let depth_bits: gl::types::GLint =
                    ctxt.glow
                        .get_texture_level_parameter_i32(texture, 0, glow::TEXTURE_DEPTH_SIZE);
                let stencil_bits: gl::types::GLint = ctxt.glow.get_texture_level_parameter_i32(
                    texture,
                    0,
                    glow::TEXTURE_STENCIL_SIZE,
                );
                (depth_bits as u16, stencil_bits as u16)
            }
        } else if !crate::version::is_webgl()
            && (ctxt.version.0 == Api::Gl || ctxt.version >= &Version(Api::GlEs, 3, 1))
        {
            unsafe {
                // the parameters of a cube map are those of its faces
                let bind_point = image_targets(self.bind_to_current(ctxt))[0];
                let depth_bits: gl::types::GLint =
                    ctxt.glow
                        .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_DEPTH_SIZE);
                let stencil_bits: gl::types::GLint = ctxt.glow.get_tex_level_parameter_i32(
                    bind_point,
                    0,
                    glow::TEXTURE_STENCIL_SIZE,
                );
                (depth_bits as u16, stencil_bits as u16)
            }
        } else {
            // `glGetTexLevelParameteriv` isn't available on OpenGL ES 3.0 and WebGL, so this
            // is deduced from the requested format
            match self.requested_format {
                TextureFormatRequest::Specific(TextureFormat::DepthStencilFormat(
                    image_format::DepthStencilFormat::F32I8,
                )) => (32, 8),
                _ => (24, 8),
            }
        }
    }

    /// Returns the number of mipmap levels of the texture.
    #[inline]
    pub fn get_mipmap_levels(&self) -> u32 {
        self.levels
    }

    /// Returns a structure that represents the main mipmap level of the texture.
    #[inline]
    pub fn main_level(&self) -> TextureAnyMipmap<'_> {
        self.mipmap(0).unwrap()
    }

    /// Returns a structure that represents a specific mipmap of the texture.
    ///
    /// Returns `None` if out of range.
    #[inline]
    pub fn mipmap(&self, level: u32) -> Option<TextureAnyMipmap<'_>> {
        if level >= self.levels {
            return None;
        }

        let pow = 2u32.pow(level);
        Some(TextureAnyMipmap {
            texture: self,
            level,
            width: cmp::max(1, self.get_width() / pow),
            height: self.get_height().map(|height| cmp::max(1, height / pow)),
            depth: self.get_depth().map(|depth| cmp::max(1, depth / pow)),
        })
    }

    /// Generates the mipmaps of the texture from its main level (`glGenerateMipmap`).
    ///
    /// Returns `TextureCreationError::MipmapsNotSupported` if the texture is multisampled, or if
    /// the backend can't generate mipmaps in its format.
    #[inline]
    pub fn generate_mipmaps(&self) -> Result<(), TextureCreationError> {
        if !self.can_generate_mipmaps {
            return Err(TextureCreationError::MipmapsNotSupported);
        }

        let mut ctxt = self.context.make_current();
        // shader writes made through image units must be done before the levels are generated
        self.prepare_for_access(&mut ctxt, crate::TextureAccess::Update);
        let bind_point = self.bind_to_current(&mut ctxt);
        unsafe { generate_mipmaps(&ctxt, bind_point) };
        Ok(())
    }
}

impl TextureExt for TextureAny {
    #[inline]
    fn get_texture_id(&self) -> glow::Texture {
        self.id
    }

    #[inline]
    fn is_integer(&self) -> bool {
        // stencil indices are sampled as unsigned integers
        match self.kind() {
            TextureKind::Integral | TextureKind::Unsigned | TextureKind::Stencil => true,
            TextureKind::DepthStencil => self.samples_stencil(),
            _ => false,
        }
    }

    #[inline]
    fn has_integer_filters(&self) -> bool {
        // SafeGL creates depth-stencil textures with the linear filters of depth textures
        self.kind() != TextureKind::DepthStencil
    }

    #[inline]
    fn is_depth(&self) -> bool {
        match self.kind() {
            TextureKind::Depth => true,
            TextureKind::DepthStencil => !self.samples_stencil(),
            _ => false,
        }
    }

    #[inline]
    fn is_float32(&self) -> bool {
        match self.requested_format {
            TextureFormatRequest::Specific(format) => format.is_float32(),
            _ => false,
        }
    }

    #[inline]
    fn sampled_levels(&self) -> (u32, u32) {
        self.mipmap_range()
    }

    #[inline]
    fn get_context(&self) -> &Rc<Context> {
        &self.context
    }

    #[inline]
    fn get_bind_point(&self) -> gl::types::GLenum {
        get_bind_point(self.ty)
    }

    fn bind_to_current(&self, ctxt: &mut CommandContext<'_>) -> gl::types::GLenum {
        let bind_point = self.get_bind_point();

        let texture_unit = ctxt.state.active_texture;
        if ctxt.state.texture_units[texture_unit as usize].texture != Some(self.id) {
            unsafe { ctxt.glow.bind_texture(bind_point, Some(self.id)) };
            ctxt.state.texture_units[texture_unit as usize].texture = Some(self.id);
        }

        bind_point
    }

    fn prepare_for_access(&self, ctxt: &mut CommandContext<'_>, access_type: crate::TextureAccess) {
        prepare_for_access(&self.latest_shader_write, ctxt, access_type)
    }
}

#[cfg(any(safegl_gl43, safegl_gles32))]
impl crate::debug::sealed::Labeled for TextureAny {
    fn labeled_object(
        &self,
    ) -> (&crate::context::Context, crate::debug::sealed::LabeledObject) {
        (
            &self.context,
            crate::debug::sealed::LabeledObject::Object(crate::gl::TEXTURE, self.id.0.get()),
        )
    }
}

impl GlObject for TextureAny {
    type Id = glow::Texture;

    #[inline]
    fn get_id(&self) -> glow::Texture {
        self.id
    }
}

impl fmt::Debug for TextureAny {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            fmt,
            "Texture {:?} (dimensions: {}x{}x{}x{})",
            self.id,
            self.get_width(),
            self.get_height().unwrap_or(1),
            self.get_depth().unwrap_or(1),
            self.get_array_size().unwrap_or(1)
        )
    }
}

impl Drop for TextureAny {
    fn drop(&mut self) {
        let mut ctxt = self.context.make_current();

        // removing FBOs which contain this texture
        fbo::FramebuffersContainer::purge_texture(&mut ctxt, self.id);

        // resetting the bindings
        for tex_unit in ctxt.state.texture_units.iter_mut() {
            if tex_unit.texture == Some(self.id) {
                tex_unit.texture = None;
            }
        }

        if self.owned {
            unsafe {
                if !ctxt.objects_lost() {
                    ctxt.glow.delete_texture(self.id);
                }
            }
        }
    }
}

/// Represents a specific layer of an array texture and 3D textures.
#[derive(Copy, Clone)]
pub struct TextureAnyLayer<'a> {
    /// The texture.
    texture: &'a TextureAny,
    /// The layer.
    layer: u32,
}

impl<'a> TextureAnyLayer<'a> {
    /// Returns the texture.
    #[inline]
    pub fn get_texture(&self) -> &'a TextureAny {
        self.texture
    }

    /// Returns the number of samples of the texture.
    #[inline]
    pub fn get_samples(&self) -> Option<u32> {
        self.texture.get_samples()
    }

    /// Returns the layer of the texture.
    #[inline]
    pub fn get_layer(&self) -> u32 {
        self.layer
    }

    /// Returns a structure that represents the main mipmap level of this layer of the texture.
    #[inline]
    pub fn main_level(&self) -> TextureAnyLayerMipmap<'a> {
        self.mipmap(0).unwrap()
    }

    /// Returns a structure that represents a specific mipmap of this layer of the texture.
    ///
    /// Returns `None` if out of range.
    #[inline]
    pub fn mipmap(&self, level: u32) -> Option<TextureAnyLayerMipmap<'a>> {
        if level >= self.texture.levels {
            return None;
        }

        let pow = 2u32.pow(level);

        Some(TextureAnyLayerMipmap {
            texture: self.texture,
            level,
            layer: self.layer,
            width: cmp::max(1, self.texture.get_width() / pow),
            height: self
                .texture
                .get_height()
                .map(|height| cmp::max(1, height / pow)),
        })
    }
}

/// Represents a specific mipmap of a texture.
#[derive(Copy, Clone)]
pub struct TextureAnyMipmap<'a> {
    /// The texture.
    texture: &'a TextureAny,

    /// Mipmap level.
    level: u32,

    /// Width of this mipmap level.
    width: u32,
    /// Height of this mipmap level.
    height: Option<u32>,
    /// Depth of this mipmap level.
    depth: Option<u32>,
}

impl<'a> TextureAnyMipmap<'a> {
    /// Returns the width of the mipmap.
    #[inline]
    pub fn get_width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the mipmap.
    #[inline]
    pub fn get_height(&self) -> Option<u32> {
        self.height
    }

    /// Returns the depth of the mipmap.
    #[inline]
    pub fn get_depth(&self) -> Option<u32> {
        self.depth
    }

    /// Computes a tuple (width, height, depth) of mipmap dimensions,
    /// using 1 for unused dimensions.
    ///
    /// In the case of 1D texture arrays, use array size as width.
    /// In the case of 2D texture arrays, use array size as depth.
    #[inline]
    fn get_mipmap_dimensions(&self) -> (u32, u32, u32) {
        // the faces of cubemaps are layers too
        let tex_depth = match self.texture.ty {
            Dimensions::Texture2dArray { array_size, .. } => array_size,
            Dimensions::Cubemap { .. } => 6,
            Dimensions::CubemapArray { array_size, .. } => 6 * array_size,
            _ => self.depth.unwrap_or(1),
        };
        let tex_height = match self.texture.ty {
            Dimensions::Texture1dArray { array_size, .. } => array_size,
            _ => self.height.unwrap_or(1),
        };
        (self.width, tex_height, tex_depth)
    }

    /// Returns the number of samples of the texture.
    #[inline]
    pub fn get_samples(&self) -> Option<u32> {
        self.texture.get_samples()
    }

    /// Returns the texture.
    #[inline]
    pub fn get_texture(&self) -> &'a TextureAny {
        self.texture
    }

    /// Returns the level of the texture.
    #[inline]
    pub fn get_level(&self) -> u32 {
        self.level
    }

    /// Returns a structure that represents the first layer of this mipmap of the texture. All
    /// textures have a first layer.
    #[inline]
    pub fn first_layer(&self) -> TextureAnyLayerMipmap<'a> {
        self.layer(0).unwrap()
    }

    /// Returns a structure that represents a specific layer of this mipmap of the texture.
    ///
    /// Non-array textures have only one layer. The number of layers can be queried with
    /// `get_array_size`.
    ///
    /// Returns `None` if out of range.
    #[inline]
    pub fn layer(&self, layer: u32) -> Option<TextureAnyLayerMipmap<'a>> {
        if let Some(array_size) = self.texture.get_array_size() {
            if layer >= array_size {
                return None;
            }
        }

        if let Some(depth) = self.depth {
            if layer >= depth {
                return None;
            }
        }

        if layer >= 1 && self.depth.is_none() && self.texture.get_array_size().is_none() {
            return None;
        }

        Some(TextureAnyLayerMipmap {
            texture: self.texture,
            layer,
            level: self.level,
            width: self.width,
            height: self.height,
        })
    }

    /// Returns the array size of the texture.
    #[inline]
    pub fn get_array_size(&self) -> Option<u32> {
        self.texture.get_array_size()
    }

    /// Uploads data to the texture from a buffer.
    ///
    /// `x`, `y` and `z` are the ranges of texels to write. For arrays of one-dimensional textures,
    /// `y` is the range of layers, for arrays of two-dimensional textures `z` is, and for cubemaps
    /// `z` is the range of faces (`6 * layer + face` for cubemap arrays).
    ///
    /// Returns an error if the ranges are outside of the texture, if the buffer is not big enough
    /// to hold the data, or if the texture is multisampled.
    #[inline]
    pub fn raw_upload_from_pixel_buffer<P>(
        &self,
        source: BufferSlice<'_, [P]>,
        x: Range<u32>,
        y: Range<u32>,
        z: Range<u32>,
    ) -> Result<(), UploadError>
    where
        P: PixelValue,
    {
        self.raw_upload_from_pixel_buffer_impl(source, x, y, z, false)
    }

    /// Uploads data to the texture from a buffer. The R, G and B components are flipped.
    ///
    /// See `raw_upload_from_pixel_buffer`. Returns `UploadError::FormatNotSupported` on OpenGL ES,
    /// which can't flip the components while uploading.
    #[inline]
    pub fn raw_upload_from_pixel_buffer_inverted<P>(
        &self,
        source: BufferSlice<'_, [P]>,
        x: Range<u32>,
        y: Range<u32>,
        z: Range<u32>,
    ) -> Result<(), UploadError>
    where
        P: PixelValue,
    {
        self.raw_upload_from_pixel_buffer_impl(source, x, y, z, true)
    }

    fn raw_upload_from_pixel_buffer_impl<P>(
        &self,
        source: BufferSlice<'_, [P]>,
        x: Range<u32>,
        y: Range<u32>,
        z: Range<u32>,
        inverted: bool,
    ) -> Result<(), UploadError>
    where
        P: PixelValue,
    {
        // OpenGL objects can only be used with the context that created them
        if !Rc::ptr_eq(source.get_context(), &self.texture.context) {
            return Err(UploadError::ContextMismatch);
        }

        if let Dimensions::Texture2dMultisample { .. } | Dimensions::Texture2dMultisampleArray { .. } =
            self.texture.ty
        {
            return Err(UploadError::MultisampleTexture);
        }

        let tex_dim = self.get_mipmap_dimensions();
        if x.start > x.end
            || y.start > y.end
            || z.start > z.end
            || x.end > tex_dim.0
            || y.end > tex_dim.1
            || z.end > tex_dim.2
        {
            return Err(UploadError::OutOfBounds);
        }

        let width = x.end - x.start;
        let height = y.end - y.start;
        let depth = z.end - z.start;

        let texels = (width * height * depth) as usize;
        if source.len() < texels {
            return Err(UploadError::BufferTooSmall);
        }

        let (client_format, client_type) = image_format::client_format_to_glenum(
            &self.texture.context,
            ClientFormatAny::ClientFormat(P::get_format()),
            self.texture.requested_format,
            inverted,
        )
        .map_err(|_| UploadError::FormatNotSupported)?;

        let mut ctxt = self.texture.context.make_current();

        // shader writes made through image units must be done before the image is changed
        self.texture
            .prepare_for_access(&mut ctxt, crate::TextureAccess::Update);

        // binds the pixel buffer
        source.prepare_and_bind_for_pixel_unpack(&mut ctxt);
        let offset = source.get_offset_bytes();
        let level = self.level as gl::types::GLint;

        unsafe {
            if ctxt.state.pixel_store_unpack_alignment != 1 {
                ctxt.state.pixel_store_unpack_alignment = 1;
                ctxt.glow.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            }

            let bind_point = self.texture.bind_to_current(&mut ctxt);
            match self.texture.ty {
                Dimensions::Texture1d { .. } => {
                    ctxt.extra.TexSubImage1D(
                        bind_point,
                        level,
                        x.start as gl::types::GLint,
                        width as gl::types::GLsizei,
                        client_format,
                        client_type,
                        offset as *const c_void,
                    );
                }

                Dimensions::Texture1dArray { .. }
                | Dimensions::Texture2d { .. }
                | Dimensions::TextureRectangle { .. } => {
                    ctxt.glow.tex_sub_image_2d(
                        bind_point,
                        level,
                        x.start as gl::types::GLint,
                        y.start as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height as gl::types::GLsizei,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::BufferOffset(offset as u32),
                    );
                }

                // without direct state access, the faces of a cubemap are separate targets
                Dimensions::Cubemap { .. } => {
                    let face_bytes = (width * height) as usize * mem::size_of::<P>();
                    for (index, face) in z.clone().enumerate() {
                        ctxt.glow.tex_sub_image_2d(
                            gl::TEXTURE_CUBE_MAP_POSITIVE_X + face,
                            level,
                            x.start as gl::types::GLint,
                            y.start as gl::types::GLint,
                            width as gl::types::GLsizei,
                            height as gl::types::GLsizei,
                            client_format,
                            client_type,
                            glow::PixelUnpackData::BufferOffset(
                                (offset + index * face_bytes) as u32,
                            ),
                        );
                    }
                }

                Dimensions::Texture2dArray { .. }
                | Dimensions::Texture3d { .. }
                | Dimensions::CubemapArray { .. } => {
                    ctxt.glow.tex_sub_image_3d(
                        bind_point,
                        level,
                        x.start as gl::types::GLint,
                        y.start as gl::types::GLint,
                        z.start as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height as gl::types::GLsizei,
                        depth as gl::types::GLsizei,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::BufferOffset(offset as u32),
                    );
                }

                Dimensions::Texture2dMultisample { .. }
                | Dimensions::Texture2dMultisampleArray { .. } => unreachable!(),
            }
        }

        // handling synchronization for the buffer
        if let Some(fence) = source.add_fence() {
            fence.insert(&mut ctxt);
        }

        Ok(())
    }
}

impl<'t> TextureMipmapExt for TextureAnyMipmap<'t> {
    fn upload_texture<'d, P>(
        &self,
        x_offset: u32,
        y_offset: u32,
        z_offset: u32,
        (format, data): (ClientFormatAny, Cow<'d, [P]>),
        width: u32,
        height: Option<u32>,
        depth: Option<u32>,
        regen_mipmaps: bool,
    ) -> Result<(), UploadError>
    where
        P: Send + Copy + Clone + 'd,
    {
        let id = self.texture.id;
        let level = self.level;

        let (is_client_compressed, data_bufsize) = (
            format.is_compressed(),
            format
                .get_buffer_size(width, height, depth, None)
                .ok_or(UploadError::FormatNotSupported)?,
        );
        let regen_mipmaps = regen_mipmaps
            && self.texture.levels >= 2
            && self.texture.generate_mipmaps
            && !is_client_compressed;

        assert!(!regen_mipmaps || level == 0); // when regen_mipmaps is true, level must be 0!
        if x_offset as u64 + width as u64 > self.width as u64
            || y_offset as u64 + height.unwrap_or(1) as u64 > self.height.unwrap_or(1) as u64
            || z_offset as u64 + depth.unwrap_or(1) as u64 > self.depth.unwrap_or(1) as u64
        {
            return Err(UploadError::OutOfBounds);
        }

        // the regions of compressed textures are made of whole blocks, of 4x4 texels for the
        // formats that SafeGL chooses; ETC1 and PVRTC textures are written whole
        let texture_format = match self.texture.requested_format {
            TextureFormatRequest::Specific(TextureFormat::CompressedFormat(format)) => {
                Some(ClientFormatAny::CompressedFormat(format))
            }
            TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(format)) => {
                Some(ClientFormatAny::CompressedSrgbFormat(format))
            }
            _ => None,
        };
        let block = match (texture_format, self.texture.requested_format) {
            (Some(format), _) => format.block_size(),
            (
                None,
                TextureFormatRequest::AnyCompressed | TextureFormatRequest::AnyCompressedSrgb,
            ) => Some((4, 4, 0)),
            (None, _) => None,
        };
        let written_whole = texture_format.is_some_and(|format| format.is_written_whole());
        let whole = (x_offset, y_offset, width, height.unwrap_or(1))
            == (0, 0, self.width, self.height.unwrap_or(1));
        if written_whole && !whole {
            return Err(UploadError::NotSupported);
        }
        if let Some((block_width, block_height, _)) = block {
            let aligned = |offset: u32, size: u32, extent: u32, block: u32| {
                offset % block == 0 && (size % block == 0 || offset + size == extent)
            };
            if !(aligned(x_offset, width, self.width, block_width)
                && aligned(y_offset, height.unwrap_or(1), self.height.unwrap_or(1), block_height))
            {
                return Err(UploadError::MisalignedBlocks);
            }
        }

        if data.len() * mem::size_of::<P>() != data_bufsize {
            return Err(UploadError::DataSizeMismatch {
                expected: data_bufsize,
                obtained: data.len() * mem::size_of::<P>(),
            });
        }

        let (client_format, client_type) = image_format::client_format_to_glenum(
            &self.texture.context,
            format,
            self.texture.requested_format,
            false,
        )
        .map_err(|_| UploadError::FormatNotSupported)?;

        let mut ctxt = self.texture.context.make_current();

        // shader writes made through image units must be done before the image is changed
        self.texture
            .prepare_for_access(&mut ctxt, crate::TextureAccess::Update);

        unsafe {
            if ctxt.state.pixel_store_unpack_alignment != 1 {
                ctxt.state.pixel_store_unpack_alignment = 1;
                ctxt.glow.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            }

            BufferAny::unbind_pixel_unpack(&mut ctxt);
            let bind_point = self.texture.bind_to_current(&mut ctxt);

            // the size of `data` in bytes has been checked against `data_bufsize` above
            let data = slice::from_raw_parts(data.as_ptr() as *const u8, data_bufsize);

            if bind_point == gl::TEXTURE_CUBE_MAP || bind_point == gl::TEXTURE_CUBE_MAP_ARRAY {
                panic!("the faces of a cubemap are written one by one, through its images");
            } else if bind_point == gl::TEXTURE_3D || bind_point == gl::TEXTURE_2D_ARRAY {
                if is_client_compressed {
                    ctxt.glow.compressed_tex_sub_image_3d(
                        bind_point,
                        level as gl::types::GLint,
                        x_offset as gl::types::GLint,
                        y_offset as gl::types::GLint,
                        z_offset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height.unwrap_or(1) as gl::types::GLsizei,
                        depth.unwrap_or(1) as gl::types::GLsizei,
                        client_format,
                        glow::CompressedPixelUnpackData::Slice(data),
                    );
                } else {
                    ctxt.glow.tex_sub_image_3d(
                        bind_point,
                        level as gl::types::GLint,
                        x_offset as gl::types::GLint,
                        y_offset as gl::types::GLint,
                        z_offset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height.unwrap_or(1) as gl::types::GLsizei,
                        depth.unwrap_or(1) as gl::types::GLsizei,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::Slice(Some(data)),
                    );
                }
            } else if bind_point == gl::TEXTURE_2D
                || bind_point == gl::TEXTURE_RECTANGLE
                || bind_point == gl::TEXTURE_1D_ARRAY
            {
                assert!(z_offset == 0);
                // FIXME should glTexImage be used here somewhere or glTexSubImage does it just fine?
                if written_whole {
                    // ETC1 and PVRTC levels can only be given whole, with `glCompressedTexImage2D`
                    ctxt.glow.compressed_tex_image_2d(
                        bind_point,
                        level as gl::types::GLint,
                        client_format as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height.unwrap_or(1) as gl::types::GLsizei,
                        0,
                        data.len() as gl::types::GLint,
                        data,
                    );
                } else if is_client_compressed {
                    ctxt.glow.compressed_tex_sub_image_2d(
                        bind_point,
                        level as gl::types::GLint,
                        x_offset as gl::types::GLint,
                        y_offset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height.unwrap_or(1) as gl::types::GLsizei,
                        client_format,
                        glow::CompressedPixelUnpackData::Slice(data),
                    );
                } else {
                    ctxt.glow.tex_sub_image_2d(
                        bind_point,
                        level as gl::types::GLint,
                        x_offset as gl::types::GLint,
                        y_offset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        height.unwrap_or(1) as gl::types::GLsizei,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::Slice(Some(data)),
                    );
                }
            } else {
                assert!(z_offset == 0);
                assert!(y_offset == 0);

                if is_client_compressed {
                    ctxt.extra.CompressedTexSubImage1D(
                        bind_point,
                        level as gl::types::GLint,
                        x_offset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        client_format,
                        data_bufsize as gl::types::GLsizei,
                        data.as_ptr() as *const c_void,
                    );
                } else {
                    ctxt.extra.TexSubImage1D(
                        bind_point,
                        level as gl::types::GLint,
                        x_offset as gl::types::GLint,
                        width as gl::types::GLsizei,
                        client_format,
                        client_type,
                        data.as_ptr() as *const c_void,
                    );
                }
            }

            // regenerate mipmaps if there are some
            if regen_mipmaps {
                ctxt.glow.generate_mipmap(bind_point);
            }

            Ok(())
        }
    }

    fn download_compressed_data(&self) -> Option<(ClientFormatAny, Vec<u8>)> {
        let texture = self.texture;
        let level = self.level as i32;

        let mut ctxt = texture.context.make_current();
        // shader writes made through image units must be done before the image is read
        texture.prepare_for_access(&mut ctxt, crate::TextureAccess::Update);

        // `glGetCompressedTexImage` only exists on desktop OpenGL, and the compression
        // parameters of `glGetTexLevelParameteriv` aren't available on OpenGL ES and WebGL
        if ctxt.version.0 != Api::Gl {
            return None;
        }

        unsafe {
            let bind_point = texture.bind_to_current(&mut ctxt);
            // the images of a level of a cube map are its faces, which are queried and read
            // one by one
            let targets = image_targets(bind_point);

            let is_compressed =
                ctxt.glow
                    .get_tex_level_parameter_i32(targets[0], level, glow::TEXTURE_COMPRESSED);
            if is_compressed != 0 {
                let queried_size = ctxt.glow.get_tex_level_parameter_i32(
                    targets[0],
                    level,
                    glow::TEXTURE_COMPRESSED_IMAGE_SIZE,
                );
                let internal_format = ctxt.glow.get_tex_level_parameter_i32(
                    targets[0],
                    level,
                    glow::TEXTURE_INTERNAL_FORMAT,
                );

                match ClientFormatAny::from_internal_compressed_format(
                    internal_format as gl::types::GLenum,
                ) {
                    Some(known_format) => {
                        // the size of a face is that of its blocks: NVIDIA gives the size of
                        // the whole cube map for each face
                        let image_size = if targets.len() > 1 {
                            known_format
                                .get_buffer_size(self.width, self.height, None, None)
                                .unwrap_or(0) as gl::types::GLint
                        } else {
                            queried_size
                        };
                        let buffer_size = image_size as usize * targets.len();
                        let mut buf = vec![0u8; buffer_size];

                        BufferAny::unbind_pixel_pack(&mut ctxt);

                        // adjusting data alignement
                        let ptr = buf.as_ptr() as *const u8;
                        let ptr = ptr as usize;
                        if (ptr % 8) == 0 {
                        } else if (ptr % 4) == 0 && ctxt.state.pixel_store_pack_alignment != 4 {
                            ctxt.state.pixel_store_pack_alignment = 4;
                            ctxt.glow.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
                        } else if (ptr % 2) == 0 && ctxt.state.pixel_store_pack_alignment > 2 {
                            ctxt.state.pixel_store_pack_alignment = 2;
                            ctxt.glow.pixel_store_i32(glow::PACK_ALIGNMENT, 2);
                        } else if ctxt.state.pixel_store_pack_alignment != 1 {
                            ctxt.state.pixel_store_pack_alignment = 1;
                            ctxt.glow.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
                        }

                        // the robust entry points never write past the end of the buffer
                        let size = image_size as gl::types::GLsizei;
                        for (target, image) in
                            targets.iter().zip(buf.chunks_exact_mut(image_size as usize))
                        {
                            let data = image.as_mut_ptr() as *mut _;
                            if ctxt.version >= &Version(Api::Gl, 4, 5) {
                                ctxt.extra.GetnCompressedTexImage(*target, level, size, data);
                            } else if ctxt.extensions.gl_arb_robustness {
                                ctxt.extra.GetnCompressedTexImageARB(*target, level, size, data);
                            } else {
                                ctxt.extra.GetCompressedTexImage(*target, level, data);
                            }
                        }
                        Some((known_format, buf))
                    }
                    None => None,
                }
            } else {
                None
            }
        }
    }
}

/// Represents a specific layer of a specific mipmap. This is the same as `TextureAnyImage`, except
/// for 3D textures, cubemaps and cubemap arrays.
#[derive(Copy, Clone)]
pub struct TextureAnyLayerMipmap<'a> {
    /// The texture.
    texture: &'a TextureAny,

    /// Layer for array textures, or 0 for other textures.
    layer: u32,
    /// Mipmap level.
    level: u32,

    /// Width of this layer of mipmap.
    width: u32,
    /// Height of this layer of mipmap.
    height: Option<u32>,
}

impl<'a> TextureAnyLayerMipmap<'a> {
    /// Returns the texture.
    #[inline]
    pub fn get_texture(&self) -> &'a TextureAny {
        self.texture
    }

    /// Returns the level of the texture.
    #[inline]
    pub fn get_level(&self) -> u32 {
        self.level
    }

    /// Returns the layer of the texture.
    #[inline]
    pub fn get_layer(&self) -> u32 {
        self.layer
    }

    /// Returns the width of this texture slice.
    #[inline]
    pub fn get_width(&self) -> u32 {
        self.width
    }

    /// Returns the height of this texture slice.
    #[inline]
    pub fn get_height(&self) -> Option<u32> {
        self.height
    }

    /// Returns the number of samples of the texture.
    #[inline]
    pub fn get_samples(&self) -> Option<u32> {
        self.texture.get_samples()
    }

    /// Turns this into an image.
    ///
    /// Returns `None` if `cube_layer` is `None` and the texture is a cubemap. Returns `None` if
    /// `cube_layer` is `Some` and the texture is not a cubemap.
    #[inline]
    pub fn into_image(&self, cube_layer: Option<CubeLayer>) -> Option<TextureAnyImage<'a>> {
        match (self.texture.ty, cube_layer) {
            (Dimensions::Cubemap { .. }, Some(_)) => (),
            (Dimensions::CubemapArray { .. }, Some(_)) => (),
            (Dimensions::Cubemap { .. }, None) => return None,
            (Dimensions::CubemapArray { .. }, None) => return None,
            (_, Some(_)) => return None,
            _ => (),
        };

        Some(TextureAnyImage {
            texture: self.texture,
            layer: self.layer,
            level: self.level,
            cube_layer,
            width: self.width,
            height: self.height,
        })
    }
}

/// Represents a specific 2D image of a texture. 1D textures are considered as having a height of 1.
#[derive(Copy, Clone)]
pub struct TextureAnyImage<'a> {
    /// The texture.
    texture: &'a TextureAny,

    /// Layer for array textures, or 0 for other textures.
    layer: u32,
    /// Mipmap level.
    level: u32,
    /// The layer of the cubemap if relevant.
    cube_layer: Option<CubeLayer>,

    /// Width of this image.
    width: u32,
    /// Height of this image.
    height: Option<u32>,
}

impl<'a> TextureAnyImage<'a> {
    /// Returns the texture.
    #[inline]
    pub fn get_texture(&self) -> &'a TextureAny {
        self.texture
    }

    /// Returns the level of the texture.
    #[inline]
    pub fn get_level(&self) -> u32 {
        self.level
    }

    /// Returns the layer of the texture.
    #[inline]
    pub fn get_layer(&self) -> u32 {
        self.layer
    }

    /// Returns the cubemap layer of this image, or `None` if the texture is not a cubemap.
    #[inline]
    pub fn get_cubemap_layer(&self) -> Option<CubeLayer> {
        self.cube_layer
    }

    /// Returns the width of this texture slice.
    #[inline]
    pub fn get_width(&self) -> u32 {
        self.width
    }

    /// Returns the height of this texture slice.
    #[inline]
    pub fn get_height(&self) -> Option<u32> {
        self.height
    }

    /// Returns the number of samples of the texture.
    #[inline]
    pub fn get_samples(&self) -> Option<u32> {
        self.texture.get_samples()
    }

    /// Uploads data to a rectangle of the image. A layer of a one-dimensional texture array is
    /// an image with a height of 1.
    ///
    /// Returns an error if the rect is out of the image or its dimensions don't match the data,
    /// if the texture is multisampled, or if the format of the data isn't compatible with the
    /// texture's.
    pub fn raw_write<'d, D>(&self, rect: &Rect, data: D) -> Result<(), UploadError>
    where
        D: Texture2dDataSource<'d>,
    {
        let RawImage2d {
            data,
            width,
            height,
            format,
        } = data.into_raw();

        if let Dimensions::Texture2dMultisample { .. } | Dimensions::Texture2dMultisampleArray { .. } =
            self.texture.ty
        {
            return Err(UploadError::MultisampleTexture);
        }

        if rect.width != width
            || rect.height != height
            || rect.left as u64 + rect.width as u64 > self.width as u64
            || rect.bottom as u64 + rect.height as u64 > self.height.unwrap_or(1) as u64
        {
            return Err(UploadError::OutOfBounds);
        }

        let (client_format, client_type) = image_format::client_format_to_glenum(
            &self.texture.context,
            ClientFormatAny::ClientFormat(format),
            self.texture.requested_format,
            false,
        )
        .map_err(|_| UploadError::FormatNotSupported)?;

        let data_size = data.len() * mem::size_of::<D::Data>();
        let expected = ClientFormatAny::ClientFormat(format)
            .get_buffer_size(width, Some(height), None, None)
            .ok_or(UploadError::FormatNotSupported)?;
        if data_size != expected {
            return Err(UploadError::DataSizeMismatch {
                expected,
                obtained: data_size,
            });
        }
        // the size of `data` in bytes has been checked against the size of the rect above
        let data = unsafe { slice::from_raw_parts(data.as_ptr() as *const u8, data_size) };

        let mut ctxt = self.texture.context.make_current();

        // shader writes made through image units must be done before the image is changed
        self.texture
            .prepare_for_access(&mut ctxt, crate::TextureAccess::Update);

        unsafe {
            if ctxt.state.pixel_store_unpack_alignment != 1 {
                ctxt.state.pixel_store_unpack_alignment = 1;
                ctxt.glow.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            }

            BufferAny::unbind_pixel_unpack(&mut ctxt);
            let bind_point = self.texture.bind_to_current(&mut ctxt);
            let level = self.level as gl::types::GLint;
            let (x, y) = (
                rect.left as gl::types::GLint,
                rect.bottom as gl::types::GLint,
            );
            let (width, height) = (
                rect.width as gl::types::GLsizei,
                rect.height as gl::types::GLsizei,
            );

            match self.texture.ty {
                Dimensions::Texture1d { .. } => {
                    ctxt.extra.TexSubImage1D(
                        bind_point,
                        level,
                        x,
                        width,
                        client_format,
                        client_type,
                        data.as_ptr() as *const c_void,
                    );
                }
                Dimensions::Texture1dArray { .. } => {
                    ctxt.glow.tex_sub_image_2d(
                        bind_point,
                        level,
                        x,
                        self.layer as gl::types::GLint,
                        width,
                        1,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::Slice(Some(data)),
                    );
                }
                Dimensions::Texture2d { .. }
                | Dimensions::TextureRectangle { .. }
                | Dimensions::Cubemap { .. } => {
                    let target = match self.cube_layer {
                        Some(face) => {
                            gl::TEXTURE_CUBE_MAP_POSITIVE_X + face.get_layer_index() as u32
                        }
                        None => bind_point,
                    };
                    ctxt.glow.tex_sub_image_2d(
                        target,
                        level,
                        x,
                        y,
                        width,
                        height,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::Slice(Some(data)),
                    );
                }
                Dimensions::Texture2dArray { .. }
                | Dimensions::Texture3d { .. }
                | Dimensions::CubemapArray { .. } => {
                    let layer = match self.cube_layer {
                        Some(face) => self.layer * 6 + face.get_layer_index() as u32,
                        None => self.layer,
                    };
                    ctxt.glow.tex_sub_image_3d(
                        bind_point,
                        level,
                        x,
                        y,
                        layer as gl::types::GLint,
                        width,
                        height,
                        1,
                        client_format,
                        client_type,
                        glow::PixelUnpackData::Slice(Some(data)),
                    );
                }
                Dimensions::Texture2dMultisample { .. }
                | Dimensions::Texture2dMultisampleArray { .. } => unreachable!(),
            }
        }

        Ok(())
    }

    /// Returns true if the rect is inside of the image.
    fn contains(&self, rect: &Rect) -> bool {
        rect.left as u64 + rect.width as u64 <= self.width as u64
            && rect.bottom as u64 + rect.height as u64 <= self.height.unwrap_or(1) as u64
    }

    /// Reads the content of a rectangle of the image.
    ///
    /// Returns an error if the rect is out of the image, if the backend can't read the image in
    /// the format `P`, or if the context has been lost.
    pub fn raw_read<T, P>(&self, rect: &Rect) -> Result<T, ReadError>
    where
        T: Texture2dDataSink<P>,
        P: PixelValue,
    {
        if !self.contains(rect) {
            return Err(ReadError::OutOfBounds);
        }

        let mut ctxt = self.texture.context.make_current();

        let mut data = Vec::new();
        ops::read(
            &mut ctxt,
            &fbo::RegularAttachment::Texture(*self),
            rect,
            &mut data,
            false,
        )?;

        Ok(T::from_raw(Cow::Owned(data), rect.width, rect.height))
    }

    /// Reads the content of a rectangle of the image to a pixel buffer.
    ///
    /// Returns an error if the rect is out of the image, if the buffer is too small or belongs to
    /// another context, if the backend can't read the image in the format `P`, or if the context
    /// has been lost.
    pub fn raw_read_to_pixel_buffer<P>(
        &self,
        rect: &Rect,
        dest: &PixelBuffer<P>,
    ) -> Result<(), ReadError>
    where
        P: PixelValue,
    {
        if !self.contains(rect) {
            return Err(ReadError::OutOfBounds);
        }

        // OpenGL objects can only be used with the context that created them
        if !Rc::ptr_eq(dest.get_context(), &self.texture.context) {
            return Err(ReadError::ContextMismatch);
        }

        if dest.len() < rect.width as usize * rect.height as usize {
            return Err(ReadError::BufferTooSmall);
        }

        let mut ctxt = self.texture.context.make_current();
        ops::read(
            &mut ctxt,
            &fbo::RegularAttachment::Texture(*self),
            rect,
            dest,
            false,
        )
    }

    /// Sets every texel of the image to `data` (`glClearBuffer`), through a framebuffer.
    ///
    /// `data` must match the kind of the texture: `[f32; 4]` for float and normalized textures,
    /// `[i32; 4]` or `[u32; 4]` for integer textures, an `f32` depth for depth textures, and
    /// `ClearBufferData::Stencil` or `DepthStencil` for stencil and depth-stencil textures.
    /// Otherwise `ClearBufferError::KindMismatch` is returned. The scissor box, the write masks
    /// and the sRGB conversion don't apply.
    pub fn raw_clear_buffer<D>(&self, data: D) -> Result<(), crate::fbo::ClearBufferError>
    where
        D: Into<ClearBufferData>,
    {
        unsafe {
            let mut ctxt = self.texture.context.make_current();
            let attachment = fbo::RegularAttachment::Texture(*self);
            fbo::FramebuffersContainer::clear_buffer(&mut ctxt, &attachment, data.into())
        }
    }
}

/// Adds the memory barrier that an access to a texture needs after the shader writes made
/// through image units, `latest_shader_write` being the draw call that last made one.
pub(crate) fn prepare_for_access(
    latest_shader_write: &Cell<u64>,
    ctxt: &mut CommandContext<'_>,
    access_type: crate::TextureAccess,
) {
    match access_type {
        crate::TextureAccess::TextureFetch => {
            if latest_shader_write.get() >= ctxt.state.latest_memory_barrier_texture_fetch {
                unsafe {
                    ctxt.glow.memory_barrier(glow::TEXTURE_FETCH_BARRIER_BIT);
                }
                ctxt.state.latest_memory_barrier_texture_fetch = ctxt.state.next_draw_call_id;
            }
        }
        crate::TextureAccess::ImageUnit { will_write } => {
            if latest_shader_write.get() >= ctxt.state.latest_memory_barrier_shader_image_access {
                unsafe {
                    ctxt.glow
                        .memory_barrier(glow::SHADER_IMAGE_ACCESS_BARRIER_BIT);
                }
                ctxt.state.latest_memory_barrier_shader_image_access = ctxt.state.next_draw_call_id;
            }

            if will_write {
                latest_shader_write.set(ctxt.state.next_draw_call_id);
            }
        }
        crate::TextureAccess::Framebuffer => {
            if latest_shader_write.get() >= ctxt.state.latest_memory_barrier_framebuffer {
                unsafe {
                    ctxt.glow.memory_barrier(glow::FRAMEBUFFER_BARRIER_BIT);
                }
                ctxt.state.latest_memory_barrier_framebuffer = ctxt.state.next_draw_call_id;
            }
        }
        crate::TextureAccess::Update => {
            if latest_shader_write.get() >= ctxt.state.latest_memory_barrier_texture_update {
                unsafe {
                    ctxt.glow.memory_barrier(glow::TEXTURE_UPDATE_BARRIER_BIT);
                }
                ctxt.state.latest_memory_barrier_texture_update = ctxt.state.next_draw_call_id;
            }
        }
    }
}

/// Size of a mipmap as `glClearTexSubImage` and `glCopyImageSubData` address it: the layers of
/// array textures and the faces of cubemaps count along the last used dimension.
pub(crate) fn image_extent(mipmap: &TextureAnyMipmap<'_>) -> (u32, u32, u32) {
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
pub(crate) fn fits(
    offset: (u32, u32, u32),
    size: (u32, u32, u32),
    extent: (u32, u32, u32),
) -> bool {
    let fits = |offset: u32, size: u32, extent: u32| {
        offset.checked_add(size).is_some_and(|end| end <= extent)
    };
    fits(offset.0, size.0, extent.0)
        && fits(offset.1, size.1, extent.1)
        && fits(offset.2, size.2, extent.2)
}


impl<'a> TextureAnyMipmap<'a> {
    /// Reads the whole level of a 2D or rectangle texture: with `glGetTexImage` on desktop
    /// OpenGL, which reads every format, and through a framebuffer otherwise.
    pub(crate) fn read_2d<T, P>(&self) -> Result<T, ReadError>
    where
        T: Texture2dDataSink<P>,
        P: PixelValue,
    {
        let (width, height) = (self.get_width(), self.get_height().unwrap_or(1));
        #[cfg(safegl_desktop)]
        if self.texture.context.get_opengl_version().0 == Api::Gl {
            let data = self.read_level::<P>()?;
            return Ok(T::from_raw(Cow::Owned(data), width, height));
        }
        let rect = Rect {
            left: 0,
            bottom: 0,
            width,
            height,
        };
        self.first_layer().into_image(None).unwrap().raw_read(&rect)
    }

    /// Reads the whole level of a 2D or rectangle texture into a pixel buffer, like `read_2d`.
    pub(crate) fn read_2d_to_pixel_buffer<P>(&self) -> Result<PixelBuffer<P>, ReadError>
    where
        P: PixelValue,
    {
        let (width, height) = (self.get_width(), self.get_height().unwrap_or(1));
        let texels = width as usize * height as usize;
        let buffer = PixelBuffer::new_empty(&self.texture.context, texels);
        #[cfg(safegl_desktop)]
        if self.texture.context.get_opengl_version().0 == Api::Gl {
            self.read_level_to_pixel_buffer(&buffer)?;
            return Ok(buffer);
        }
        let rect = Rect {
            left: 0,
            bottom: 0,
            width,
            height,
        };
        self.first_layer()
            .into_image(None)
            .unwrap()
            .raw_read_to_pixel_buffer(&rect, &buffer)?;
        Ok(buffer)
    }
}
