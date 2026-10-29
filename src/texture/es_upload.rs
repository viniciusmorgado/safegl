//! The uploads of OpenGL ES and WebGL, which only take the client data whose format and type
//! match the internal format of the texture (table 3.2 of OpenGL ES 3.0, and the formats of
//! extensions), where desktop OpenGL converts any data. SafeGL converts the other data on the
//! CPU, the way desktop OpenGL does: the components that the texture doesn't have are dropped,
//! the missing ones are 0, 0, 0 and 1, normalized values are rounded to the nearest, and values
//! out of range are clamped. Integer textures take integer data only.

use crate::gl;
use crate::image_format::ClientFormat;

/// How the texels of an internal format are stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    /// Unsigned normalized, sRGB included.
    Unorm,
    /// Signed normalized.
    Snorm,
    /// Floating-point.
    Float,
    /// Unsigned integers.
    Uint,
    /// Signed integers.
    Sint,
}

/// The layout of the data uploaded after a conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encoding {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    /// RGBA in `GL_UNSIGNED_INT_2_10_10_10_REV`.
    U2101010Rev,
}

/// The internal formats of OpenGL ES that take uncompressed data: their family, their number of
/// components, the client formats that they take, and the encoding of the converted data.
fn es_format(
    internal: gl::types::GLenum,
) -> Option<(Family, usize, &'static [ClientFormat], Encoding)> {
    use ClientFormat as C;
    use Encoding as E;
    use Family as F;
    Some(match internal {
        gl::R8 | gl::SR8_EXT => (F::Unorm, 1, &[C::U8], E::U8),
        gl::RG8 | gl::SRG8_EXT => (F::Unorm, 2, &[C::U8U8], E::U8),
        gl::RGB8 | gl::SRGB8 => (F::Unorm, 3, &[C::U8U8U8], E::U8),
        gl::RGB565 => (F::Unorm, 3, &[C::U8U8U8, C::U5U6U5], E::U8),
        gl::RGBA8 | gl::SRGB8_ALPHA8 => (F::Unorm, 4, &[C::U8U8U8U8], E::U8),
        gl::RGB5_A1 => (F::Unorm, 4, &[C::U8U8U8U8, C::U5U5U5U1], E::U8),
        gl::RGBA4 => (F::Unorm, 4, &[C::U8U8U8U8, C::U4U4U4U4], E::U8),
        gl::RGB10_A2 => (F::Unorm, 4, &[], E::U2101010Rev),
        gl::R16 => (F::Unorm, 1, &[C::U16], E::U16),
        gl::RG16 => (F::Unorm, 2, &[C::U16U16], E::U16),
        gl::RGB16 => (F::Unorm, 3, &[C::U16U16U16], E::U16),
        gl::RGBA16 => (F::Unorm, 4, &[C::U16U16U16U16], E::U16),
        gl::R8_SNORM => (F::Snorm, 1, &[C::I8], E::I8),
        gl::RG8_SNORM => (F::Snorm, 2, &[C::I8I8], E::I8),
        gl::RGB8_SNORM => (F::Snorm, 3, &[C::I8I8I8], E::I8),
        gl::RGBA8_SNORM => (F::Snorm, 4, &[C::I8I8I8I8], E::I8),
        gl::R16_SNORM => (F::Snorm, 1, &[C::I16], E::I16),
        gl::RG16_SNORM => (F::Snorm, 2, &[C::I16I16], E::I16),
        gl::RGB16_SNORM => (F::Snorm, 3, &[C::I16I16I16], E::I16),
        gl::RGBA16_SNORM => (F::Snorm, 4, &[C::I16I16I16I16], E::I16),
        gl::R16F => (F::Float, 1, &[C::F16, C::F32], E::F32),
        gl::R32F => (F::Float, 1, &[C::F32], E::F32),
        gl::RG16F => (F::Float, 2, &[C::F16F16, C::F32F32], E::F32),
        gl::RG32F => (F::Float, 2, &[C::F32F32], E::F32),
        gl::R11F_G11F_B10F | gl::RGB9_E5 | gl::RGB16F => {
            (F::Float, 3, &[C::F16F16F16, C::F32F32F32], E::F32)
        }
        gl::RGB32F => (F::Float, 3, &[C::F32F32F32], E::F32),
        gl::RGBA16F => (F::Float, 4, &[C::F16F16F16F16, C::F32F32F32F32], E::F32),
        gl::RGBA32F => (F::Float, 4, &[C::F32F32F32F32], E::F32),
        gl::R8UI => (F::Uint, 1, &[C::U8], E::U8),
        gl::RG8UI => (F::Uint, 2, &[C::U8U8], E::U8),
        gl::RGB8UI => (F::Uint, 3, &[C::U8U8U8], E::U8),
        gl::RGBA8UI => (F::Uint, 4, &[C::U8U8U8U8], E::U8),
        gl::R16UI => (F::Uint, 1, &[C::U16], E::U16),
        gl::RG16UI => (F::Uint, 2, &[C::U16U16], E::U16),
        gl::RGB16UI => (F::Uint, 3, &[C::U16U16U16], E::U16),
        gl::RGBA16UI => (F::Uint, 4, &[C::U16U16U16U16], E::U16),
        gl::R32UI => (F::Uint, 1, &[C::U32], E::U32),
        gl::RG32UI => (F::Uint, 2, &[C::U32U32], E::U32),
        gl::RGB32UI => (F::Uint, 3, &[C::U32U32U32], E::U32),
        gl::RGBA32UI => (F::Uint, 4, &[C::U32U32U32U32], E::U32),
        gl::RGB10_A2UI => (F::Uint, 4, &[], E::U2101010Rev),
        gl::R8I => (F::Sint, 1, &[C::I8], E::I8),
        gl::RG8I => (F::Sint, 2, &[C::I8I8], E::I8),
        gl::RGB8I => (F::Sint, 3, &[C::I8I8I8], E::I8),
        gl::RGBA8I => (F::Sint, 4, &[C::I8I8I8I8], E::I8),
        gl::R16I => (F::Sint, 1, &[C::I16], E::I16),
        gl::RG16I => (F::Sint, 2, &[C::I16I16], E::I16),
        gl::RGB16I => (F::Sint, 3, &[C::I16I16I16], E::I16),
        gl::RGBA16I => (F::Sint, 4, &[C::I16I16I16I16], E::I16),
        gl::R32I => (F::Sint, 1, &[C::I32], E::I32),
        gl::RG32I => (F::Sint, 2, &[C::I32I32], E::I32),
        gl::RGB32I => (F::Sint, 3, &[C::I32I32I32], E::I32),
        gl::RGBA32I => (F::Sint, 4, &[C::I32I32I32I32], E::I32),
        _ => return None,
    })
}

/// Data converted for an upload: its bytes, and the format and the type to give OpenGL.
pub(crate) struct Converted {
    pub data: Vec<u8>,
    pub format: gl::types::GLenum,
    pub ty: gl::types::GLenum,
}

/// What an upload of `client` data into a texture of the internal format `internal` needs on
/// OpenGL ES and WebGL.
pub(crate) enum Upload {
    /// The data is uploaded as it is: OpenGL ES takes it, or the internal format isn't one that
    /// SafeGL converts data for.
    AsIs,
    /// The data must be converted (`convert`).
    Convert,
    /// No exact conversion exists: floating-point data into an integer texture, or integer data
    /// into a normalized or floating-point one of another signedness.
    Refused,
}

/// Returns what an upload of `client` data into the internal format `internal` needs.
pub(crate) fn upload(internal: gl::types::GLenum, client: ClientFormat) -> Upload {
    let Some((family, _, valid, _)) = es_format(internal) else {
        return Upload::AsIs;
    };
    if valid.contains(&client) {
        return Upload::AsIs;
    }
    let integer_data = !matches!(
        client,
        ClientFormat::F16
            | ClientFormat::F16F16
            | ClientFormat::F16F16F16
            | ClientFormat::F16F16F16F16
            | ClientFormat::F32
            | ClientFormat::F32F32
            | ClientFormat::F32F32F32
            | ClientFormat::F32F32F32F32
    );
    match family {
        Family::Uint | Family::Sint if !integer_data => Upload::Refused,
        _ => Upload::Convert,
    }
}

/// The value of each component of a pixel, normalized for the normalized and floating-point
/// textures, and raw for the integer ones.
type Pixel = [f64; 4];

/// Reads the pixels of `data`, of the format `client`, as normalized values if `normalize`,
/// otherwise as raw values. The components that `client` doesn't have are 0, 0, 0 and 1.
fn decode(data: &[u8], client: ClientFormat, normalize: bool) -> Vec<Pixel> {
    use ClientFormat as C;

    let unorm = |value: f64, max: f64| if normalize { value / max } else { value };
    let snorm = |value: f64, max: f64| {
        if normalize {
            (value / max).max(-1.0)
        } else {
            value
        }
    };
    let (count, size) = (client.get_num_components() as usize, client.get_size());
    data.chunks_exact(size)
        .map(|texel| {
            let mut pixel = [0.0, 0.0, 0.0, 1.0];
            let u16_at = |i: usize| u16::from_ne_bytes([texel[2 * i], texel[2 * i + 1]]);
            let u32_at = |i: usize| {
                u32::from_ne_bytes([
                    texel[4 * i],
                    texel[4 * i + 1],
                    texel[4 * i + 2],
                    texel[4 * i + 3],
                ])
            };
            match client {
                C::U8 | C::U8U8 | C::U8U8U8 | C::U8U8U8U8 => {
                    for i in 0..count {
                        pixel[i] = unorm(texel[i] as f64, 255.0);
                    }
                }
                C::I8 | C::I8I8 | C::I8I8I8 | C::I8I8I8I8 => {
                    for i in 0..count {
                        pixel[i] = snorm(texel[i] as i8 as f64, 127.0);
                    }
                }
                C::U16 | C::U16U16 | C::U16U16U16 | C::U16U16U16U16 => {
                    for i in 0..count {
                        pixel[i] = unorm(u16_at(i) as f64, 65535.0);
                    }
                }
                C::I16 | C::I16I16 | C::I16I16I16 | C::I16I16I16I16 => {
                    for i in 0..count {
                        pixel[i] = snorm(u16_at(i) as i16 as f64, 32767.0);
                    }
                }
                C::U32 | C::U32U32 | C::U32U32U32 | C::U32U32U32U32 => {
                    for i in 0..count {
                        pixel[i] = unorm(u32_at(i) as f64, u32::MAX as f64);
                    }
                }
                C::I32 | C::I32I32 | C::I32I32I32 | C::I32I32I32I32 => {
                    for i in 0..count {
                        pixel[i] = snorm(u32_at(i) as i32 as f64, i32::MAX as f64);
                    }
                }
                C::F16 | C::F16F16 | C::F16F16F16 | C::F16F16F16F16 => {
                    for i in 0..count {
                        pixel[i] = half_to_f32(u16_at(i)) as f64;
                    }
                }
                C::F32 | C::F32F32 | C::F32F32F32 | C::F32F32F32F32 => {
                    for i in 0..count {
                        pixel[i] = f32::from_bits(u32_at(i)) as f64;
                    }
                }
                // the packed formats put their first component in the highest bits, and their
                // reversed ones in the lowest bits
                C::U3U3U2 => {
                    let v = texel[0] as u32;
                    pixel[..3].copy_from_slice(&[
                        unorm((v >> 5) as f64, 7.0),
                        unorm((v >> 2 & 7) as f64, 7.0),
                        unorm((v & 3) as f64, 3.0),
                    ]);
                }
                C::U5U6U5 => {
                    let v = u16_at(0) as u32;
                    pixel[..3].copy_from_slice(&[
                        unorm((v >> 11) as f64, 31.0),
                        unorm((v >> 5 & 63) as f64, 63.0),
                        unorm((v & 31) as f64, 31.0),
                    ]);
                }
                C::U4U4U4U4 => {
                    let v = u16_at(0) as u32;
                    for (i, shift) in [12, 8, 4, 0].into_iter().enumerate() {
                        pixel[i] = unorm((v >> shift & 15) as f64, 15.0);
                    }
                }
                C::U5U5U5U1 => {
                    let v = u16_at(0) as u32;
                    pixel = [
                        unorm((v >> 11) as f64, 31.0),
                        unorm((v >> 6 & 31) as f64, 31.0),
                        unorm((v >> 1 & 31) as f64, 31.0),
                        unorm((v & 1) as f64, 1.0),
                    ];
                }
                C::U1U5U5U5Reversed => {
                    let v = u16_at(0) as u32;
                    pixel = [
                        unorm((v & 31) as f64, 31.0),
                        unorm((v >> 5 & 31) as f64, 31.0),
                        unorm((v >> 10 & 31) as f64, 31.0),
                        unorm((v >> 15) as f64, 1.0),
                    ];
                }
                C::U10U10U10U2 => {
                    let v = u32_at(0);
                    pixel = [
                        unorm((v >> 22) as f64, 1023.0),
                        unorm((v >> 12 & 1023) as f64, 1023.0),
                        unorm((v >> 2 & 1023) as f64, 1023.0),
                        unorm((v & 3) as f64, 3.0),
                    ];
                }
            }
            pixel
        })
        .collect()
}

/// Converts `data` of the format `client` for an upload into the internal format `internal`,
/// for which `upload` returned `Upload::Convert`.
pub(crate) fn convert(data: &[u8], client: ClientFormat, internal: gl::types::GLenum) -> Converted {
    let (family, count, _, encoding) = es_format(internal).expect("a format that SafeGL converts");
    let integer = matches!(family, Family::Uint | Family::Sint);
    let pixels = decode(data, client, !integer);

    let unorm = |value: f64, max: f64| (value.clamp(0.0, 1.0) * max).round();
    let snorm = |value: f64, max: f64| (value.clamp(-1.0, 1.0) * max).round();
    let mut out = Vec::with_capacity(pixels.len() * count * 4);
    for pixel in &pixels {
        let components = &pixel[..count];
        match (encoding, family) {
            (Encoding::U8, Family::Unorm) => {
                out.extend(components.iter().map(|&c| unorm(c, 255.0) as u8))
            }
            (Encoding::U8, _) => out.extend(components.iter().map(|&c| c.clamp(0.0, 255.0) as u8)),
            (Encoding::I8, Family::Snorm) => {
                out.extend(components.iter().map(|&c| snorm(c, 127.0) as i8 as u8))
            }
            (Encoding::I8, _) => out.extend(
                components
                    .iter()
                    .map(|&c| c.clamp(-128.0, 127.0) as i8 as u8),
            ),
            (Encoding::U16, Family::Unorm) => {
                for &c in components {
                    out.extend((unorm(c, 65535.0) as u16).to_ne_bytes());
                }
            }
            (Encoding::U16, _) => {
                for &c in components {
                    out.extend((c.clamp(0.0, 65535.0) as u16).to_ne_bytes());
                }
            }
            (Encoding::I16, Family::Snorm) => {
                for &c in components {
                    out.extend((snorm(c, 32767.0) as i16).to_ne_bytes());
                }
            }
            (Encoding::I16, _) => {
                for &c in components {
                    out.extend((c.clamp(-32768.0, 32767.0) as i16).to_ne_bytes());
                }
            }
            (Encoding::U32, _) => {
                for &c in components {
                    out.extend((c.clamp(0.0, u32::MAX as f64) as u32).to_ne_bytes());
                }
            }
            (Encoding::I32, _) => {
                for &c in components {
                    let value = c.clamp(i32::MIN as f64, i32::MAX as f64) as i32;
                    out.extend(value.to_ne_bytes());
                }
            }
            (Encoding::F32, _) => {
                for &c in components {
                    out.extend((c as f32).to_ne_bytes());
                }
            }
            (Encoding::U2101010Rev, _) => {
                let (rgb, alpha) = match family {
                    Family::Unorm => (
                        [
                            unorm(pixel[0], 1023.0),
                            unorm(pixel[1], 1023.0),
                            unorm(pixel[2], 1023.0),
                        ],
                        unorm(pixel[3], 3.0),
                    ),
                    _ => (
                        [
                            pixel[0].clamp(0.0, 1023.0),
                            pixel[1].clamp(0.0, 1023.0),
                            pixel[2].clamp(0.0, 1023.0),
                        ],
                        pixel[3].clamp(0.0, 3.0),
                    ),
                };
                let value = rgb[0] as u32
                    | (rgb[1] as u32) << 10
                    | (rgb[2] as u32) << 20
                    | (alpha as u32) << 30;
                out.extend(value.to_ne_bytes());
            }
        }
    }

    let format = match (count, integer) {
        (1, false) => gl::RED,
        (2, false) => gl::RG,
        (3, false) => gl::RGB,
        (_, false) => gl::RGBA,
        (1, true) => gl::RED_INTEGER,
        (2, true) => gl::RG_INTEGER,
        (3, true) => gl::RGB_INTEGER,
        (_, true) => gl::RGBA_INTEGER,
    };
    let ty = match encoding {
        Encoding::U8 => gl::UNSIGNED_BYTE,
        Encoding::I8 => gl::BYTE,
        Encoding::U16 => gl::UNSIGNED_SHORT,
        Encoding::I16 => gl::SHORT,
        Encoding::U32 => gl::UNSIGNED_INT,
        Encoding::I32 => gl::INT,
        Encoding::F32 => gl::FLOAT,
        Encoding::U2101010Rev => gl::UNSIGNED_INT_2_10_10_10_REV,
    };
    Converted {
        data: out,
        format,
        ty,
    }
}

/// Converts the bytes of a half-precision float to a float.
fn half_to_f32(half: u16) -> f32 {
    let sign = if half & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = (half >> 10) & 0x1F;
    let mantissa = (half & 0x3FF) as f32;
    sign * match exponent {
        0 => mantissa * 2f32.powi(-24),
        0x1F if mantissa == 0.0 => f32::INFINITY,
        0x1F => f32::NAN,
        _ => (1.0 + mantissa / 1024.0) * 2f32.powi(exponent as i32 - 15),
    }
}

/// The bytes of the data of an upload.
pub(crate) fn bytes<P>(data: &[P]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data)) }
}

#[cfg(test)]
mod tests {
    use super::{convert, upload, Upload};
    use crate::gl;
    use crate::image_format::ClientFormat;

    #[test]
    fn rgba_bytes_into_one_component() {
        assert!(matches!(
            upload(gl::R8, ClientFormat::U8U8U8U8),
            Upload::Convert
        ));
        assert!(matches!(upload(gl::R8, ClientFormat::U8), Upload::AsIs));
        let converted = convert(
            &[10, 20, 30, 40, 50, 60, 70, 80],
            ClientFormat::U8U8U8U8,
            gl::R8,
        );
        assert_eq!(converted.data, [10, 50]);
        assert_eq!(
            (converted.format, converted.ty),
            (gl::RED, gl::UNSIGNED_BYTE)
        );
    }

    #[test]
    fn floats_into_bytes() {
        let data: Vec<u8> = [0.0f32, 0.5, 1.0, 2.0]
            .iter()
            .flat_map(|f| f.to_ne_bytes())
            .collect();
        let converted = convert(&data, ClientFormat::F32F32F32F32, gl::RGBA8);
        assert_eq!(converted.data, [0, 128, 255, 255]);
    }

    #[test]
    fn missing_components_and_packed_formats() {
        // RGB into RGBA gets an opaque alpha
        let converted = convert(&[255, 0, 0], ClientFormat::U8U8U8, gl::RGBA8);
        assert_eq!(converted.data, [255, 0, 0, 255]);
        // 5-6-5 into RGB8
        let red = 0b11111_000000_00000u16.to_ne_bytes();
        let converted = convert(&red, ClientFormat::U5U6U5, gl::RGBA8);
        assert_eq!(converted.data, [255, 0, 0, 255]);
        // bytes into RGB10_A2, packed in the reversed order of OpenGL ES
        let converted = convert(&[255, 0, 0, 255], ClientFormat::U8U8U8U8, gl::RGB10_A2);
        assert_eq!(converted.data, (1023u32 | 3 << 30).to_ne_bytes());
    }

    #[test]
    fn integer_textures() {
        assert!(matches!(
            upload(gl::R8UI, ClientFormat::F32),
            Upload::Refused
        ));
        let converted = convert(&[7, 0, 0, 0], ClientFormat::U32, gl::R8UI);
        assert_eq!(converted.data, [7]);
        assert_eq!(
            (converted.format, converted.ty),
            (gl::RED_INTEGER, gl::UNSIGNED_BYTE)
        );
    }
}
