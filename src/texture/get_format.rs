use glow::HasContext;

use crate::context::CommandContext;
use crate::gl;
use crate::version::is_webgl;
use crate::version::Api;
use crate::version::Version;

use std::error::Error;
use std::fmt;

use crate::texture::any::TextureAny;
use crate::GlObject;
use crate::TextureExt;

/// Error that can happen when retrieving the internal format of a texture.
#[derive(Copy, Clone, Debug)]
pub enum GetFormatError {
    /// The backend doesn't support retrieving the internal format.
    NotSupported,
}

impl fmt::Display for GetFormatError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::GetFormatError::*;
        let desc = match self {
            NotSupported => "The backend doesn't support retrieving the internal format",
        };
        fmt.write_str(desc)
    }
}

impl Error for GetFormatError {}

/// Internal format of a texture.
///
/// The actual format of a texture is not necessarily one of the predefined ones, so we have
/// to use a very generic description.
// TODO: change bits to be u16 for consistency with the rest of the library
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InternalFormat {
    /// The format has one component.
    OneComponent {
        /// Type of the first component of the format.
        ty1: InternalFormatType,
        /// Number of bits of the first component.
        bits1: usize,
    },

    /// The format has two components.
    TwoComponents {
        /// Type of the first component of the format.
        ty1: InternalFormatType,
        /// Number of bits of the first component.
        bits1: usize,
        /// Type of the second component.
        ty2: InternalFormatType,
        /// Number of bits of the second component.
        bits2: usize,
    },

    /// The format has three components.
    ThreeComponents {
        /// Type of the first component of the format.
        ty1: InternalFormatType,
        /// Number of bits of the first component.
        bits1: usize,
        /// Type of the second component.
        ty2: InternalFormatType,
        /// Number of bits of the second component.
        bits2: usize,
        /// Type of the third component.
        ty3: InternalFormatType,
        /// Number of bits of the third component.
        bits3: usize,
    },

    /// The format has four components.
    FourComponents {
        /// Type of the first component of the format.
        ty1: InternalFormatType,
        /// Number of bits of the first component.
        bits1: usize,
        /// Type of the second component.
        ty2: InternalFormatType,
        /// Number of bits of the second component.
        bits2: usize,
        /// Type of the third component.
        ty3: InternalFormatType,
        /// Number of bits of the third component.
        bits3: usize,
        /// Type of the fourth component.
        ty4: InternalFormatType,
        /// Number of bits of the fourth component.
        bits4: usize,
    },
}

impl InternalFormat {
    /// Returns the total number of bits of this format.
    #[inline]
    pub fn get_total_bits(&self) -> usize {
        match *self {
            InternalFormat::OneComponent { bits1, .. } => bits1,
            InternalFormat::TwoComponents { bits1, bits2, .. } => bits1 + bits2,
            InternalFormat::ThreeComponents {
                bits1,
                bits2,
                bits3,
                ..
            } => bits1 + bits2 + bits3,
            InternalFormat::FourComponents {
                bits1,
                bits2,
                bits3,
                bits4,
                ..
            } => bits1 + bits2 + bits3 + bits4,
        }
    }
}

/// Format of a component of an internal format.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InternalFormatType {
    /// Floating point texture with signed components.
    SignedNormalized,
    /// Floating point texture with unsigned components.
    UnsignedNormalized,
    /// Floating point texture with floats.
    Float,
    /// Integral texture.
    Int,
    /// Unsigned texture.
    UnsignedInt,
}

impl InternalFormatType {
    /// Builds the `InternalFormatType` of the GLenum. Panics if the value is not recognized.
    #[inline]
    fn from_glenum(val: gl::types::GLenum) -> InternalFormatType {
        match val {
            gl::SIGNED_NORMALIZED => InternalFormatType::SignedNormalized,
            gl::UNSIGNED_NORMALIZED => InternalFormatType::UnsignedNormalized,
            gl::FLOAT => InternalFormatType::Float,
            gl::INT => InternalFormatType::Int,
            gl::UNSIGNED_INT => InternalFormatType::UnsignedInt,
            gl::NONE => unreachable!(), // separating this case for easier debugging
            _ => unreachable!(),
        }
    }
}

/// Determines the format of a texture.
///
/// A `TextureAny` is guaranteed to have the same format for each mipmap.
pub fn get_format(
    ctxt: &mut CommandContext<'_>,
    texture: &TextureAny,
) -> Result<InternalFormat, GetFormatError> {
    // `glGetTexLevelParameteriv` is core in OpenGL ES 3.1, and doesn't exist on WebGL
    if is_webgl() || (ctxt.version.0 == Api::GlEs && ctxt.version < &Version(Api::GlEs, 3, 1)) {
        return Err(GetFormatError::NotSupported);
    }

    let (
        red_sz,
        red_ty,
        green_sz,
        green_ty,
        blue_sz,
        blue_ty,
        alpha_sz,
        alpha_ty,
        depth_sz,
        depth_ty,
    ) = unsafe {
        let red_sz;
        let red_ty;
        let green_sz;
        let green_ty;
        let blue_sz;
        let blue_ty;
        let alpha_sz;
        let alpha_ty;
        let depth_sz;
        let depth_ty;

        if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access {
            let id = texture.get_id();
            red_sz = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_RED_SIZE);
            red_ty = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_RED_TYPE);
            green_sz = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_GREEN_SIZE);
            green_ty = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_GREEN_TYPE);
            blue_sz = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_BLUE_SIZE);
            blue_ty = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_BLUE_TYPE);
            alpha_sz = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_ALPHA_SIZE);
            alpha_ty = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_ALPHA_TYPE);
            depth_sz = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_DEPTH_SIZE);
            depth_ty = ctxt
                .glow
                .get_texture_level_parameter_i32(id, 0, glow::TEXTURE_DEPTH_TYPE);
        } else {
            let bind_point = texture.bind_to_current(ctxt);
            red_sz = ctxt
                .glow
                .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_RED_SIZE);
            red_ty = ctxt
                .glow
                .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_RED_TYPE);
            green_sz =
                ctxt.glow
                    .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_GREEN_SIZE);
            green_ty =
                ctxt.glow
                    .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_GREEN_TYPE);
            blue_sz = ctxt
                .glow
                .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_BLUE_SIZE);
            blue_ty = ctxt
                .glow
                .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_BLUE_TYPE);
            alpha_sz =
                ctxt.glow
                    .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_ALPHA_SIZE);
            alpha_ty =
                ctxt.glow
                    .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_ALPHA_TYPE);
            depth_sz =
                ctxt.glow
                    .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_DEPTH_SIZE);
            depth_ty =
                ctxt.glow
                    .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_DEPTH_TYPE);
        }

        (
            red_sz as gl::types::GLenum,
            red_ty as gl::types::GLenum,
            green_sz as gl::types::GLenum,
            green_ty as gl::types::GLenum,
            blue_sz as gl::types::GLenum,
            blue_ty as gl::types::GLenum,
            alpha_sz as gl::types::GLenum,
            alpha_ty as gl::types::GLenum,
            depth_sz as gl::types::GLenum,
            depth_ty as gl::types::GLenum,
        )
    };

    Ok(
        match (
            red_sz, red_ty, green_sz, green_ty, blue_sz, blue_ty, alpha_sz, alpha_ty, depth_sz,
            depth_ty,
        ) {
            // stencil textures have neither color nor depth components
            (_, gl::NONE, _, _, _, _, _, _, _, gl::NONE) => {
                let stencil_sz = unsafe {
                    let bind_point = texture.bind_to_current(ctxt);
                    ctxt.glow
                        .get_tex_level_parameter_i32(bind_point, 0, glow::TEXTURE_STENCIL_SIZE)
                };
                if stencil_sz <= 0 {
                    return Err(GetFormatError::NotSupported);
                }
                InternalFormat::OneComponent {
                    ty1: InternalFormatType::UnsignedInt,
                    bits1: stencil_sz as usize,
                }
            }
            (_, gl::NONE, _, _, _, _, _, _, sz1, ty1) => InternalFormat::OneComponent {
                ty1: InternalFormatType::from_glenum(ty1),
                bits1: sz1 as usize,
            },
            (sz1, ty1, _, gl::NONE, _, _, _, _, _, _) => InternalFormat::OneComponent {
                ty1: InternalFormatType::from_glenum(ty1),
                bits1: sz1 as usize,
            },
            (sz1, ty1, sz2, ty2, _, gl::NONE, _, _, _, _) => InternalFormat::TwoComponents {
                ty1: InternalFormatType::from_glenum(ty1),
                bits1: sz1 as usize,
                ty2: InternalFormatType::from_glenum(ty2),
                bits2: sz2 as usize,
            },
            (sz1, ty1, sz2, ty2, sz3, ty3, _, gl::NONE, _, _) => InternalFormat::ThreeComponents {
                ty1: InternalFormatType::from_glenum(ty1),
                bits1: sz1 as usize,
                ty2: InternalFormatType::from_glenum(ty2),
                bits2: sz2 as usize,
                ty3: InternalFormatType::from_glenum(ty3),
                bits3: sz3 as usize,
            },
            (sz1, ty1, sz2, ty2, sz3, ty3, sz4, ty4, _, gl::NONE) => {
                InternalFormat::FourComponents {
                    ty1: InternalFormatType::from_glenum(ty1),
                    bits1: sz1 as usize,
                    ty2: InternalFormatType::from_glenum(ty2),
                    bits2: sz2 as usize,
                    ty3: InternalFormatType::from_glenum(ty3),
                    bits3: sz3 as usize,
                    ty4: InternalFormatType::from_glenum(ty4),
                    bits4: sz4 as usize,
                }
            }
            _ => unreachable!(),
        },
    )
}
