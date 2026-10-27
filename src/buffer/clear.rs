//! Filling buffers with a value (`glClearBufferSubData`, OpenGL 4.3).

use std::error::Error;
use std::fmt;

use crate::buffer::Content;
use crate::gl;

/// Error that can happen when clearing a buffer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClearBufferError {
    /// The backend doesn't support clearing buffers (OpenGL 4.3 or `GL_ARB_clear_buffer_object`).
    NotSupported,
}

impl fmt::Display for ClearBufferError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = match *self {
            ClearBufferError::NotSupported => "The backend doesn't support clearing buffers",
        };
        fmt.write_str(desc)
    }
}

impl Error for ClearBufferError {}

mod sealed {
    pub trait Sealed {}
}

/// A value that `clear` can fill a buffer with, bit for bit: an integer of 8, 16 or 32 bits, an
/// `f32`, or an array of 2 or 4 of them, or of 3 of the 32-bit ones (the formats of
/// `glClearBufferSubData`).
pub trait ClearValue: Copy + sealed::Sealed {
    /// The internal format, format and type of the value for `glClearBufferSubData`.
    #[doc(hidden)]
    fn clear_format() -> (gl::types::GLenum, gl::types::GLenum, gl::types::GLenum);
}

/// The content of a buffer that `clear` can fill: a `ClearValue`, or a slice of them.
pub trait ClearContent: Content + sealed::Sealed {
    /// The value that fills the content.
    type Value: ClearValue;
}

macro_rules! clear_value {
    ($ty:ty, $internal:ident, $format:ident, $data:ident) => {
        impl sealed::Sealed for $ty {}
        impl sealed::Sealed for [$ty] {}

        impl ClearValue for $ty {
            #[inline]
            fn clear_format() -> (gl::types::GLenum, gl::types::GLenum, gl::types::GLenum) {
                (gl::$internal, gl::$format, gl::$data)
            }
        }

        impl ClearContent for $ty {
            type Value = $ty;
        }

        impl ClearContent for [$ty] {
            type Value = $ty;
        }
    };
}

clear_value!(u8, R8UI, RED_INTEGER, UNSIGNED_BYTE);
clear_value!(i8, R8I, RED_INTEGER, BYTE);
clear_value!(u16, R16UI, RED_INTEGER, UNSIGNED_SHORT);
clear_value!(i16, R16I, RED_INTEGER, SHORT);
clear_value!(u32, R32UI, RED_INTEGER, UNSIGNED_INT);
clear_value!(i32, R32I, RED_INTEGER, INT);
clear_value!(f32, R32F, RED, FLOAT);
clear_value!([u8; 2], RG8UI, RG_INTEGER, UNSIGNED_BYTE);
clear_value!([i8; 2], RG8I, RG_INTEGER, BYTE);
clear_value!([u16; 2], RG16UI, RG_INTEGER, UNSIGNED_SHORT);
clear_value!([i16; 2], RG16I, RG_INTEGER, SHORT);
clear_value!([u32; 2], RG32UI, RG_INTEGER, UNSIGNED_INT);
clear_value!([i32; 2], RG32I, RG_INTEGER, INT);
clear_value!([f32; 2], RG32F, RG, FLOAT);
clear_value!([u32; 3], RGB32UI, RGB_INTEGER, UNSIGNED_INT);
clear_value!([i32; 3], RGB32I, RGB_INTEGER, INT);
clear_value!([f32; 3], RGB32F, RGB, FLOAT);
clear_value!([u8; 4], RGBA8UI, RGBA_INTEGER, UNSIGNED_BYTE);
clear_value!([i8; 4], RGBA8I, RGBA_INTEGER, BYTE);
clear_value!([u16; 4], RGBA16UI, RGBA_INTEGER, UNSIGNED_SHORT);
clear_value!([i16; 4], RGBA16I, RGBA_INTEGER, SHORT);
clear_value!([u32; 4], RGBA32UI, RGBA_INTEGER, UNSIGNED_INT);
clear_value!([i32; 4], RGBA32I, RGBA_INTEGER, INT);
clear_value!([f32; 4], RGBA32F, RGBA, FLOAT);
