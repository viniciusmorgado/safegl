use std::error::Error;
use std::fmt;
use std::{mem, slice};

use glow::HasContext;

use crate::image_format::{TextureFormat, TextureFormatRequest};
use crate::pixel_buffer::PixelBuffer;
use crate::texture::ClientFormat;
use crate::texture::PixelValue;
use crate::texture::TextureKind;

use crate::fbo;
use crate::fbo::FramebuffersContainer;

use crate::buffer::BufferAny;
use crate::context::CommandContext;
use crate::gl;
use crate::BufferExt;
use crate::Rect;

use crate::version::Api;

/// A source for reading pixels.
pub enum Source<'a> {
    /// A regular framebuffer attachment.
    Attachment(&'a fbo::RegularAttachment<'a>),
    // TODO: use a Rust enum
    DefaultFramebuffer(gl::types::GLenum),
}

impl<'a> From<&'a fbo::RegularAttachment<'a>> for Source<'a> {
    #[inline]
    fn from(a: &'a fbo::RegularAttachment<'a>) -> Source<'a> {
        Source::Attachment(a)
    }
}

/// A destination for reading pixels.
pub enum Destination<'a, P>
where
    P: PixelValue,
{
    Memory(&'a mut Vec<P>),
    PixelBuffer(&'a PixelBuffer<P>),
    // TODO: texture with glCopyTexSubImage2D
}

impl<'a, P> From<&'a mut Vec<P>> for Destination<'a, P>
where
    P: PixelValue,
{
    #[inline]
    fn from(mem: &'a mut Vec<P>) -> Destination<'a, P> {
        Destination::Memory(mem)
    }
}

impl<'a, P> From<&'a PixelBuffer<P>> for Destination<'a, P>
where
    P: PixelValue,
{
    #[inline]
    fn from(pb: &'a PixelBuffer<P>) -> Destination<'a, P> {
        Destination::PixelBuffer(pb)
    }
}

/// Error that can happen while reading.
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadError {
    /// The implementation doesn't support converting to the requested output format.
    ///
    /// OpenGL supports every possible format, but OpenGL ES only supports `(u8, u8, u8, u8)` and
    /// an implementation-defined format.
    OutputFormatNotSupported,

    /// The implementation doesn't support reading a depth, depth-stencil or stencil attachment.
    ///
    /// OpenGL ES only supports reading from color buffers by default. There are extensions that
    /// allow reading other types of attachments.
    AttachmentTypeNotSupported,

    /// Clamping the values is not supported by the implementation.
    ClampingNotSupported,

    /// Rect dimensions are too large for the buffer.
    DimensionsTooLarge, // TODO: context lost

    /// The buffer to read from doesn't exist, like the front buffer of most EGL window surfaces.
    AttachmentMissing,

    /// The OpenGL context has been lost, and the data is gone with it.
    ContextLost,

    /// The rectangle to read is outside of the image.
    OutOfBounds,

    /// The pixel buffer to read into is too small for the rectangle.
    BufferTooSmall,

    /// The pixel buffer to read into belongs to another context than the image. OpenGL objects
    /// can only be used with the context that created them.
    ContextMismatch,

    /// Reading this way isn't supported: boxes of textures need OpenGL 4.5 or
    /// `GL_ARB_get_texture_sub_image`, and multisample textures can't be read.
    NotSupported,

    /// The box to read from a compressed texture doesn't start on the blocks of its format, or
    /// doesn't end on them or on the edge of the image.
    MisalignedBlocks,
}

impl fmt::Display for ReadError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ReadError::*;
        let desc = match *self {
            OutputFormatNotSupported =>
                "The implementation doesn't support converting to the requested output format",
            AttachmentTypeNotSupported =>
                "The implementation doesn't support reading a depth, depth-stencil or stencil attachment",
            ClampingNotSupported =>
                "Clamping the values is not supported by the implementation",
            DimensionsTooLarge =>
                "Rect dimensions are too large for the buffer",
            AttachmentMissing =>
                "The buffer to read from doesn't exist",
            ContextLost =>
                "The OpenGL context has been lost",
            OutOfBounds =>
                "The rectangle to read is outside of the image",
            BufferTooSmall =>
                "The pixel buffer to read into is too small for the rectangle",
            ContextMismatch =>
                "The pixel buffer belongs to another context than the image",
            NotSupported =>
                "Reading this way is not supported by the backend",
            MisalignedBlocks =>
                "The box to read doesn't fit the blocks of the compressed format",
        };
        fmt.write_str(desc)
    }
}

impl Error for ReadError {}

/// Returns the only client format that OpenGL ES guarantees for reading from `source`.
fn es_read_format(source: &Source<'_>) -> ClientFormat {
    use crate::image_format::UncompressedFloatFormat as F;

    let requested = match source {
        Source::Attachment(fbo::RegularAttachment::Texture(tex)) => {
            Some(tex.get_texture().get_requested_format())
        }
        _ => None,
    };

    match requested {
        Some(TextureFormatRequest::AnyIntegral)
        | Some(TextureFormatRequest::Specific(TextureFormat::UncompressedIntegral(_))) => {
            ClientFormat::I32I32I32I32
        }
        Some(TextureFormatRequest::AnyUnsigned)
        | Some(TextureFormatRequest::Specific(TextureFormat::UncompressedUnsigned(_))) => {
            ClientFormat::U32U32U32U32
        }
        Some(TextureFormatRequest::Specific(TextureFormat::UncompressedFloat(
            F::F16
            | F::F16F16
            | F::F16F16F16
            | F::F16F16F16F16
            | F::F32
            | F::F32F32
            | F::F32F32F32
            | F::F32F32F32F32
            | F::F11F11F10,
        ))) => ClientFormat::F32F32F32F32,
        _ => ClientFormat::U8U8U8U8,
    }
}

/// Reads pixels from the source into the destination.
///
/// Panics if the destination is not large enough.
///
/// The `(u8, u8, u8, u8)` format is guaranteed to be supported.
// TODO: differentiate between GL_* and GL_*_INTEGER
#[inline]
pub fn read<'a, S, D, T>(
    mut ctxt: &mut CommandContext<'_>,
    source: S,
    rect: &Rect,
    dest: D,
    clamp: bool,
) -> Result<(), ReadError>
where
    S: Into<Source<'a>>,
    D: Into<Destination<'a, T>>,
    T: PixelValue,
{
    let source = source.into();
    let dest = dest.into();
    let output_pixel_format = <T as PixelValue>::get_format();

    if ctxt.state.lost_context {
        return Err(ReadError::ContextLost);
    }

    let pixels_to_read = rect
        .width
        .checked_mul(rect.height)
        .ok_or(ReadError::DimensionsTooLarge)?;

    // handling clamping
    if ctxt.version.0 == Api::Gl {
        unsafe {
            if clamp && ctxt.state.clamp_color != gl::TRUE as gl::types::GLenum {
                ctxt.extra
                    .ClampColor(gl::CLAMP_READ_COLOR, gl::TRUE as gl::types::GLenum);
                ctxt.state.clamp_color = gl::TRUE as gl::types::GLenum;
            } else if !clamp && ctxt.state.clamp_color != gl::FALSE as gl::types::GLenum {
                ctxt.extra
                    .ClampColor(gl::CLAMP_READ_COLOR, gl::FALSE as gl::types::GLenum);
                ctxt.state.clamp_color = gl::FALSE as gl::types::GLenum;
            }
        }
    } else if clamp {
        return Err(ReadError::ClampingNotSupported);
    }

    // TODO: check dimensions?

    // binding framebuffer
    match source {
        Source::Attachment(attachment) => {
            unsafe { FramebuffersContainer::bind_framebuffer_for_reading(&mut ctxt, attachment) };
        }
        Source::DefaultFramebuffer(read_buffer) => {
            // selecting a buffer that the default framebuffer doesn't have, such as the front
            // buffer of most EGL window surfaces, would raise an error
            if ctxt.version.0 == Api::Gl {
                unsafe { fbo::bind_framebuffer(ctxt, None, false, true) };
                let ty = unsafe {
                    ctxt.glow.get_framebuffer_attachment_parameter_i32(
                        gl::READ_FRAMEBUFFER,
                        read_buffer,
                        gl::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
                    )
                };
                if ty == gl::NONE as gl::types::GLint {
                    return Err(ReadError::AttachmentMissing);
                }
            }

            FramebuffersContainer::bind_default_framebuffer_for_reading(&mut ctxt, read_buffer);
        }
    };

    // determining what kind of data we are reading
    enum ReadSourceType {
        Color,
        Depth,
        Stencil,
        DepthStencil,
    }
    let kind = match source {
        Source::Attachment(fbo::RegularAttachment::Texture(ref tex)) => tex.get_texture().kind(),
        Source::Attachment(fbo::RegularAttachment::RenderBuffer(ref rb)) => rb.kind(),
        Source::DefaultFramebuffer(_) => TextureKind::Float,
    };
    let integer = matches!(kind, TextureKind::Integral | TextureKind::Unsigned);
    let read_src_type = match kind {
        TextureKind::Depth => ReadSourceType::Depth,
        TextureKind::Stencil => ReadSourceType::Stencil,
        TextureKind::DepthStencil => ReadSourceType::DepthStencil,
        TextureKind::Float | TextureKind::Integral | TextureKind::Unsigned => ReadSourceType::Color,
    };

    // the format read from OpenGL, when it isn't the output format: the pixels are then converted
    let mut read_format = None;

    // OpenGL ES doesn't support reading from depth, stencil or depth-stencil attachments by default
    if ctxt.version.0 == Api::GlEs {
        match read_src_type {
            // OpenGL supports every output format, while OpenGL ES only supports one per kind of
            // attachment, plus an implementation-defined format queried with
            // GL_IMPLEMENTATION_COLOR_READ_FORMAT: the guaranteed one is read and converted
            ReadSourceType::Color => {
                let es_format = es_read_format(&source);
                if output_pixel_format != es_format {
                    // integers are only converted to integers
                    if integer && component_type(output_pixel_format) == Some(Component::F32) {
                        return Err(ReadError::OutputFormatNotSupported);
                    }
                    if component_type(output_pixel_format).is_none() {
                        return Err(ReadError::OutputFormatNotSupported);
                    }
                    read_format = Some(es_format);
                }
            }
            ReadSourceType::Depth => {
                if !ctxt.extensions.gl_nv_read_depth {
                    return Err(ReadError::AttachmentTypeNotSupported);
                }
            }
            ReadSourceType::DepthStencil => {
                if !ctxt.extensions.gl_nv_read_depth_stencil {
                    return Err(ReadError::AttachmentTypeNotSupported);
                }
            }
            ReadSourceType::Stencil => {
                if !ctxt.extensions.gl_nv_read_stencil {
                    return Err(ReadError::AttachmentTypeNotSupported);
                }
            }
        }
    }
    // obtaining the client format and client type to be passed to `glReadPixels`
    let (format, gltype) = match read_src_type {
        ReadSourceType::Color => {
            client_format_to_gl_enum(&read_format.unwrap_or(output_pixel_format), integer)
        }
        ReadSourceType::Depth => match output_pixel_format {
            ClientFormat::F32 => (gl::DEPTH_COMPONENT, gl::FLOAT),
            ClientFormat::U16 => (gl::DEPTH_COMPONENT, gl::UNSIGNED_SHORT),
            ClientFormat::U32 => (gl::DEPTH_COMPONENT, gl::UNSIGNED_INT),
            _ => return Err(ReadError::OutputFormatNotSupported),
        },
        ReadSourceType::Stencil => match output_pixel_format {
            ClientFormat::U8 => (gl::STENCIL_INDEX, gl::UNSIGNED_BYTE),
            _ => return Err(ReadError::OutputFormatNotSupported),
        },
        // the 24 bits of depth and the 8 bits of stencil packed in a `u32`
        ReadSourceType::DepthStencil => match output_pixel_format {
            ClientFormat::U32 => (gl::DEPTH_STENCIL, gl::UNSIGNED_INT_24_8),
            _ => return Err(ReadError::OutputFormatNotSupported),
        },
    };

    // reading in another format, and converting
    if let Some(read_format) = read_format {
        let pixels = pixels_to_read as usize;
        let mut bytes = vec![0u8; pixels * read_format.get_size()];
        unsafe {
            BufferAny::unbind_pixel_unpack(ctxt);
            BufferAny::unbind_pixel_pack(ctxt);
            set_pack_alignment(ctxt, rect.width as usize * read_format.get_size());
            read_pixels_into(ctxt, rect, format, gltype, &mut bytes);
        }
        let converted = convert_pixels(&bytes, read_format, output_pixel_format);
        debug_assert_eq!(converted.len(), pixels * mem::size_of::<T>());

        match dest {
            Destination::Memory(dest) => {
                let mut buf: Vec<T> = Vec::with_capacity(pixels);
                unsafe {
                    // every bit pattern is a valid pixel
                    std::ptr::copy_nonoverlapping(
                        converted.as_ptr(),
                        buf.as_mut_ptr() as *mut u8,
                        converted.len(),
                    );
                    buf.set_len(pixels);
                }
                *dest = buf;
            }
            Destination::PixelBuffer(pixel_buffer) => {
                if pixel_buffer.len() < pixels {
                    return Err(ReadError::BufferTooSmall);
                }
                unsafe {
                    pixel_buffer.prepare_and_bind_for_pixel_unpack(ctxt);
                    ctxt.glow
                        .buffer_sub_data_u8_slice(glow::PIXEL_UNPACK_BUFFER, 0, &converted);
                }
                crate::pixel_buffer::store_infos(pixel_buffer, (rect.width, rect.height));
            }
        }

        return Ok(());
    }

    // reading
    unsafe {
        match dest {
            Destination::Memory(dest) => {
                let mut buf = Vec::with_capacity(pixels_to_read as usize);

                BufferAny::unbind_pixel_pack(ctxt);

                set_pack_alignment(ctxt, rect.width as usize * mem::size_of::<T>());

                // glow takes the destination as a byte slice, so the buffer is zeroed first to
                // avoid viewing uninitialized memory as `&mut [u8]`
                std::ptr::write_bytes(buf.as_mut_ptr(), 0, pixels_to_read as usize);
                let bytes = slice::from_raw_parts_mut(
                    buf.as_mut_ptr() as *mut u8,
                    pixels_to_read as usize * mem::size_of::<T>(),
                );
                read_pixels_into(ctxt, rect, format, gltype, bytes);
                buf.set_len(pixels_to_read as usize);

                *dest = buf;
            }

            Destination::PixelBuffer(pixel_buffer) => {
                if pixel_buffer.len() < pixels_to_read as usize {
                    return Err(ReadError::BufferTooSmall);
                }

                pixel_buffer.prepare_and_bind_for_pixel_pack(&mut ctxt);
                set_pack_alignment(ctxt, rect.width as usize * mem::size_of::<T>());
                ctxt.glow.read_pixels(
                    rect.left as gl::types::GLint,
                    rect.bottom as gl::types::GLint,
                    rect.width as gl::types::GLsizei,
                    rect.height as gl::types::GLsizei,
                    format,
                    gltype,
                    glow::PixelPackData::BufferOffset(0),
                );

                crate::pixel_buffer::store_infos(pixel_buffer, (rect.width, rect.height));
            }
        }
    };

    Ok(())
}

/// Reads the pixels of `rect` of the bound read framebuffer into `bytes`, with the robust
/// `glReadnPixels` where it exists, which never writes past the end of `bytes`.
unsafe fn read_pixels_into(
    ctxt: &mut CommandContext<'_>,
    rect: &Rect,
    format: gl::types::GLenum,
    gltype: gl::types::GLenum,
    bytes: &mut [u8],
) {
    use crate::version::Version;

    let (x, y) = (rect.left as gl::types::GLint, rect.bottom as gl::types::GLint);
    let (width, height) = (rect.width as gl::types::GLsizei, rect.height as gl::types::GLsizei);
    let size = bytes.len() as gl::types::GLsizei;
    let data = bytes.as_mut_ptr() as *mut std::ffi::c_void;
    if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.version >= &Version(Api::GlEs, 3, 2) {
        ctxt.extra
            .ReadnPixels(x, y, width, height, format, gltype, size, data);
    } else if ctxt.extensions.gl_khr_robustness && ctxt.version.0 == Api::GlEs {
        ctxt.extra
            .ReadnPixelsKHR(x, y, width, height, format, gltype, size, data);
    } else if ctxt.extensions.gl_khr_robustness {
        ctxt.extra
            .ReadnPixels(x, y, width, height, format, gltype, size, data);
    } else if ctxt.extensions.gl_arb_robustness {
        ctxt.extra
            .ReadnPixelsARB(x, y, width, height, format, gltype, size, data);
    } else if ctxt.extensions.gl_ext_robustness {
        ctxt.extra
            .ReadnPixelsEXT(x, y, width, height, format, gltype, size, data);
    } else {
        ctxt.glow.read_pixels(
            x,
            y,
            width,
            height,
            format,
            gltype,
            glow::PixelPackData::Slice(Some(bytes)),
        );
    }
}

/// Sets `GL_PACK_ALIGNMENT` so that OpenGL writes rows of `row_bytes` bytes without padding them,
/// since the destination holds the rows next to each other. A padded row would write past the
/// end of the destination.
unsafe fn set_pack_alignment(ctxt: &mut CommandContext<'_>, row_bytes: usize) {
    let alignment = [8, 4, 2]
        .into_iter()
        .find(|&alignment| row_bytes.is_multiple_of(alignment))
        .unwrap_or(1);

    if ctxt.state.pixel_store_pack_alignment != alignment as gl::types::GLint {
        ctxt.state.pixel_store_pack_alignment = alignment as gl::types::GLint;
        ctxt.glow
            .pixel_store_i32(glow::PACK_ALIGNMENT, alignment as gl::types::GLint);
    }
}

fn client_format_to_gl_enum(
    format: &ClientFormat,
    integer: bool,
) -> (gl::types::GLenum, gl::types::GLenum) {
    let (format, ty) = match *format {
        ClientFormat::U8 => (gl::RED, gl::UNSIGNED_BYTE),
        ClientFormat::U8U8 => (gl::RG, gl::UNSIGNED_BYTE),
        ClientFormat::U8U8U8 => (gl::RGB, gl::UNSIGNED_BYTE),
        ClientFormat::U8U8U8U8 => (gl::RGBA, gl::UNSIGNED_BYTE),
        ClientFormat::I8 => (gl::RED, gl::BYTE),
        ClientFormat::I8I8 => (gl::RG, gl::BYTE),
        ClientFormat::I8I8I8 => (gl::RGB, gl::BYTE),
        ClientFormat::I8I8I8I8 => (gl::RGBA, gl::BYTE),
        ClientFormat::U16 => (gl::RED, gl::UNSIGNED_SHORT),
        ClientFormat::U16U16 => (gl::RG, gl::UNSIGNED_SHORT),
        ClientFormat::U16U16U16 => (gl::RGB, gl::UNSIGNED_SHORT),
        ClientFormat::U16U16U16U16 => (gl::RGBA, gl::UNSIGNED_SHORT),
        ClientFormat::I16 => (gl::RED, gl::SHORT),
        ClientFormat::I16I16 => (gl::RG, gl::SHORT),
        ClientFormat::I16I16I16 => (gl::RGB, gl::SHORT),
        ClientFormat::I16I16I16I16 => (gl::RGBA, gl::SHORT),
        ClientFormat::U32 => (gl::RED, gl::UNSIGNED_INT),
        ClientFormat::U32U32 => (gl::RG, gl::UNSIGNED_INT),
        ClientFormat::U32U32U32 => (gl::RGB, gl::UNSIGNED_INT),
        ClientFormat::U32U32U32U32 => (gl::RGBA, gl::UNSIGNED_INT),
        ClientFormat::I32 => (gl::RED, gl::INT),
        ClientFormat::I32I32 => (gl::RG, gl::INT),
        ClientFormat::I32I32I32 => (gl::RGB, gl::INT),
        ClientFormat::I32I32I32I32 => (gl::RGBA, gl::INT),
        ClientFormat::U3U3U2 => (gl::RGB, gl::UNSIGNED_BYTE_3_3_2),
        ClientFormat::U5U6U5 => (gl::RGB, gl::UNSIGNED_SHORT_5_6_5),
        ClientFormat::U4U4U4U4 => (gl::RGBA, gl::UNSIGNED_SHORT_4_4_4_4),
        ClientFormat::U5U5U5U1 => (gl::RGBA, gl::UNSIGNED_SHORT_5_5_5_1),
        ClientFormat::U1U5U5U5Reversed => (gl::RGBA, gl::UNSIGNED_SHORT_1_5_5_5_REV),
        ClientFormat::U10U10U10U2 => (gl::RGBA, gl::UNSIGNED_INT_10_10_10_2),
        ClientFormat::F16 => (gl::RED, gl::HALF_FLOAT),
        ClientFormat::F16F16 => (gl::RG, gl::HALF_FLOAT),
        ClientFormat::F16F16F16 => (gl::RGB, gl::HALF_FLOAT),
        ClientFormat::F16F16F16F16 => (gl::RGBA, gl::HALF_FLOAT),
        ClientFormat::F32 => (gl::RED, gl::FLOAT),
        ClientFormat::F32F32 => (gl::RG, gl::FLOAT),
        ClientFormat::F32F32F32 => (gl::RGB, gl::FLOAT),
        ClientFormat::F32F32F32F32 => (gl::RGBA, gl::FLOAT),
    };

    let format = if integer {
        match format {
            gl::RED => gl::RED_INTEGER,
            gl::RG => gl::RG_INTEGER,
            gl::RGB => gl::RGB_INTEGER,
            gl::RGBA => gl::RGBA_INTEGER,
            _ => unreachable!(),
        }
    } else {
        format
    };

    (format, ty)
}

/// The type of the components of a client format.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Component {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
}

/// Returns the type of the components of a format whose components are all of the same type.
fn component_type(format: ClientFormat) -> Option<Component> {
    use ClientFormat::*;
    Some(match format {
        U8 | U8U8 | U8U8U8 | U8U8U8U8 => Component::U8,
        I8 | I8I8 | I8I8I8 | I8I8I8I8 => Component::I8,
        U16 | U16U16 | U16U16U16 | U16U16U16U16 => Component::U16,
        I16 | I16I16 | I16I16I16 | I16I16I16I16 => Component::I16,
        U32 | U32U32 | U32U32U32 | U32U32U32U32 => Component::U32,
        I32 | I32I32 | I32I32I32 | I32I32I32I32 => Component::I32,
        F32 | F32F32 | F32F32F32 | F32F32F32F32 => Component::F32,
        _ => return None,
    })
}

/// Converts pixels read in one of the formats that OpenGL ES guarantees (`U8U8U8U8`,
/// `F32F32F32F32`, `I32I32I32I32` or `U32U32U32U32`) to `output`, as `glReadPixels` would have.
///
/// Normalized values become floats between 0 and 1 (or -1 and 1), floats are clamped to the range
/// of the normalized output type, and integers are saturated.
fn convert_pixels(bytes: &[u8], read: ClientFormat, output: ClientFormat) -> Vec<u8> {
    let read_type = component_type(read).unwrap();
    let output_type = component_type(output).unwrap();
    let components = output.get_num_components() as usize;

    // the four components of each pixel, as floats for normalized and float formats
    let read_size = read.get_size() / 4;
    let pixels = bytes.len() / read.get_size();
    let mut out = Vec::with_capacity(pixels * output.get_size());
    for pixel in bytes.chunks_exact(read.get_size()) {
        for component in pixel.chunks_exact(read_size).take(components) {
            enum Value {
                Float(f64),
                Integer(i64),
            }
            let value = match read_type {
                Component::U8 => Value::Float(component[0] as f64 / 255.0),
                Component::F32 => {
                    Value::Float(f32::from_ne_bytes(component.try_into().unwrap()) as f64)
                }
                Component::I32 => {
                    Value::Integer(i32::from_ne_bytes(component.try_into().unwrap()) as i64)
                }
                Component::U32 => {
                    Value::Integer(u32::from_ne_bytes(component.try_into().unwrap()) as i64)
                }
                _ => unreachable!(),
            };

            macro_rules! normalized {
                ($ty:ty, $min:expr) => {{
                    let float = match value {
                        Value::Float(float) => float.clamp($min, 1.0),
                        Value::Integer(integer) => {
                            out.extend_from_slice(
                                &(integer.clamp(<$ty>::MIN as i64, <$ty>::MAX as i64) as $ty)
                                    .to_ne_bytes(),
                            );
                            continue;
                        }
                    };
                    out.extend_from_slice(&((float * <$ty>::MAX as f64).round() as $ty).to_ne_bytes());
                }};
            }
            match output_type {
                Component::U8 => normalized!(u8, 0.0),
                Component::I8 => normalized!(i8, -1.0),
                Component::U16 => normalized!(u16, 0.0),
                Component::I16 => normalized!(i16, -1.0),
                Component::U32 => normalized!(u32, 0.0),
                Component::I32 => normalized!(i32, -1.0),
                Component::F32 => {
                    let float = match value {
                        Value::Float(float) => float,
                        Value::Integer(integer) => integer as f64,
                    };
                    out.extend_from_slice(&(float as f32).to_ne_bytes());
                }
            }
        }
    }
    out
}
