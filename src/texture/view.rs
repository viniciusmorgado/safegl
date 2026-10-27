//! Texture views: textures that share the storage of another texture, interpreted with another
//! target, format, range of levels or range of layers (`glTextureView`, OpenGL 4.3).

use std::error::Error;
use std::fmt;

use crate::gl;

/// An error of the `view` constructors of textures.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextureViewError {
    /// Texture views aren't supported: OpenGL 4.3 or `GL_ARB_texture_view` is needed.
    NotSupported,
    /// The source texture doesn't have immutable storage, which views need. SafeGL creates
    /// textures with immutable storage where `glTexStorage*` exists, which is always the case
    /// where views do, except for textures created outside SafeGL.
    SourceNotImmutable,
    /// A texture of this kind can't be a view of a texture of the kind of the source, such as a
    /// 3D texture and a two-dimensional one.
    IncompatibleTarget,
    /// The format of the view isn't compatible with the format of the source: their texels must
    /// have the same size, as listed by the view classes of OpenGL, or the formats must be the
    /// same.
    IncompatibleFormat,
    /// The range of levels is empty or isn't inside the levels of the source.
    LevelsOutOfRange,
    /// The range of layers is empty, isn't inside the layers of the source, or doesn't fit the
    /// kind of the view: one layer for a texture without layers, six for a cube map and a
    /// multiple of six for a cube map array.
    LayersOutOfRange,
    /// The format of the view isn't supported by the backend.
    FormatNotSupported,
}

impl fmt::Display for TextureViewError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::TextureViewError::*;
        fmt.write_str(match self {
            NotSupported => "Texture views are not supported by the backend",
            SourceNotImmutable => "The source of a texture view must have immutable storage",
            IncompatibleTarget => {
                "The kind of the view is incompatible with the kind of its source"
            }
            IncompatibleFormat => {
                "The format of the view is incompatible with the format of its source"
            }
            LevelsOutOfRange => "The levels of the view aren't inside the levels of its source",
            LayersOutOfRange => "The layers of the view don't fit its source or its kind",
            FormatNotSupported => "The format of the view is not supported by the backend",
        })
    }
}

impl Error for TextureViewError {}

/// Returns true if a texture of `view` target can view the storage of a texture of `source`
/// target (table 8.21 of OpenGL 4.6).
pub(crate) fn targets_compatible(source: gl::types::GLenum, view: gl::types::GLenum) -> bool {
    let compatible: &[gl::types::GLenum] = match source {
        gl::TEXTURE_1D | gl::TEXTURE_1D_ARRAY => &[gl::TEXTURE_1D, gl::TEXTURE_1D_ARRAY],
        gl::TEXTURE_2D => &[gl::TEXTURE_2D, gl::TEXTURE_2D_ARRAY],
        gl::TEXTURE_3D => &[gl::TEXTURE_3D],
        gl::TEXTURE_RECTANGLE => &[gl::TEXTURE_RECTANGLE],
        gl::TEXTURE_CUBE_MAP | gl::TEXTURE_2D_ARRAY | gl::TEXTURE_CUBE_MAP_ARRAY => &[
            gl::TEXTURE_CUBE_MAP,
            gl::TEXTURE_2D,
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_CUBE_MAP_ARRAY,
        ],
        gl::TEXTURE_2D_MULTISAMPLE | gl::TEXTURE_2D_MULTISAMPLE_ARRAY => {
            &[gl::TEXTURE_2D_MULTISAMPLE, gl::TEXTURE_2D_MULTISAMPLE_ARRAY]
        }
        _ => &[],
    };
    compatible.contains(&view)
}

/// Returns true if a view of `view` format can view the storage of a texture of `source` format:
/// the same format, or the same view class (table 8.22 of OpenGL 4.6, and the S3TC classes of
/// `GL_EXT_texture_compression_s3tc`).
pub(crate) fn formats_compatible(source: gl::types::GLenum, view: gl::types::GLenum) -> bool {
    source == view || view_class(source).is_some_and(|class| view_class(view) == Some(class))
}

fn view_class(format: gl::types::GLenum) -> Option<u32> {
    const CLASSES: &[&[gl::types::GLenum]] = &[
        &[gl::RGBA32F, gl::RGBA32UI, gl::RGBA32I],
        &[gl::RGB32F, gl::RGB32UI, gl::RGB32I],
        &[
            gl::RGBA16F,
            gl::RG32F,
            gl::RGBA16UI,
            gl::RG32UI,
            gl::RGBA16I,
            gl::RG32I,
            gl::RGBA16,
            gl::RGBA16_SNORM,
        ],
        &[
            gl::RGB16,
            gl::RGB16_SNORM,
            gl::RGB16F,
            gl::RGB16UI,
            gl::RGB16I,
        ],
        &[
            gl::RG16F,
            gl::R11F_G11F_B10F,
            gl::R32F,
            gl::RGB10_A2UI,
            gl::RGBA8UI,
            gl::RG16UI,
            gl::R32UI,
            gl::RGBA8I,
            gl::RG16I,
            gl::R32I,
            gl::RGB10_A2,
            gl::RGBA8,
            gl::RG16,
            gl::RGBA8_SNORM,
            gl::RG16_SNORM,
            gl::SRGB8_ALPHA8,
            gl::RGB9_E5,
        ],
        &[gl::RGB8, gl::RGB8_SNORM, gl::SRGB8, gl::RGB8UI, gl::RGB8I],
        &[
            gl::R16F,
            gl::RG8UI,
            gl::R16UI,
            gl::RG8I,
            gl::R16I,
            gl::RG8,
            gl::R16,
            gl::RG8_SNORM,
            gl::R16_SNORM,
        ],
        &[gl::R8UI, gl::R8I, gl::R8, gl::R8_SNORM],
        &[gl::COMPRESSED_RED_RGTC1, gl::COMPRESSED_SIGNED_RED_RGTC1],
        &[gl::COMPRESSED_RG_RGTC2, gl::COMPRESSED_SIGNED_RG_RGTC2],
        &[
            gl::COMPRESSED_RGBA_BPTC_UNORM,
            gl::COMPRESSED_SRGB_ALPHA_BPTC_UNORM,
        ],
        &[
            gl::COMPRESSED_RGB_BPTC_SIGNED_FLOAT,
            gl::COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT,
        ],
        &[
            gl::COMPRESSED_RGB_S3TC_DXT1_EXT,
            gl::COMPRESSED_SRGB_S3TC_DXT1_EXT,
        ],
        &[
            gl::COMPRESSED_RGBA_S3TC_DXT1_EXT,
            gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT,
        ],
        &[
            gl::COMPRESSED_RGBA_S3TC_DXT3_EXT,
            gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT,
        ],
        &[
            gl::COMPRESSED_RGBA_S3TC_DXT5_EXT,
            gl::COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT,
        ],
    ];
    CLASSES
        .iter()
        .position(|class| class.contains(&format))
        .map(|class| class as u32)
}
