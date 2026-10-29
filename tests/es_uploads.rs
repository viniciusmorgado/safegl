//! Uploads whose data doesn't have the format of the texture: desktop OpenGL converts them, and
//! SafeGL converts them the same way on OpenGL ES and WebGL, which only take matching data.

#[macro_use]
extern crate safegl;

use safegl::texture::{
    MipmapsOption, RawImage2d, SrgbFormat, SrgbTexture2d, Texture2d, UncompressedFloatFormat,
    UncompressedUintFormat, UnsignedTexture2d,
};
use safegl::{Rect, Surface};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Pixel = (u8, u8, u8, u8);

/// The whole of the textures of the tests, of 2x1 texels.
const WHOLE: Rect = Rect {
    left: 0,
    bottom: 0,
    width: 2,
    height: 1,
};

/// A texture of `format`, of 2x1 texels created from `data`, read back as bytes.
fn round_trip<T>(display: &support::Backend, format: UncompressedFloatFormat, data: Vec<Vec<T>>)
    -> Vec<Vec<Pixel>>
where
    T: safegl::texture::PixelValue,
{
    let texture =
        Texture2d::with_format(display, data, format, MipmapsOption::NoMipmap).unwrap();
    texture.read().unwrap()
}

/// RGBA bytes into textures of fewer components keep the first ones.
#[test]
fn bytes_into_fewer_components() {
    let display = support::build_display();
    let data = vec![vec![(10u8, 20u8, 30u8, 40u8), (200, 150, 100, 50)]];
    assert_eq!(
        round_trip(&display, UncompressedFloatFormat::U8, data.clone()),
        [[(10, 0, 0, 255), (200, 0, 0, 255)]]
    );
    assert_eq!(
        round_trip(&display, UncompressedFloatFormat::U8U8, data.clone()),
        [[(10, 20, 0, 255), (200, 150, 0, 255)]]
    );
    assert_eq!(
        round_trip(&display, UncompressedFloatFormat::U8U8U8, data),
        [[(10, 20, 30, 255), (200, 150, 100, 255)]]
    );
    display.assert_no_error(None);
}

/// Floats into bytes are clamped and rounded, and RGB gets an opaque alpha.
#[test]
fn floats_into_bytes() {
    let display = support::build_display();
    let data = vec![vec![(0.0f32, 0.5f32, 1.0f32, 2.0f32), (1.0, 0.25, -1.0, 0.0)]];
    assert_eq!(
        round_trip(&display, UncompressedFloatFormat::U8U8U8U8, data),
        [[(0, 128, 255, 255), (255, 64, 0, 0)]]
    );
    let data = vec![vec![(1.0f32, 0.0f32, 0.0f32), (0.0, 0.0, 1.0)]];
    assert_eq!(
        round_trip(&display, UncompressedFloatFormat::U8U8U8U8, data),
        [[(255, 0, 0, 255), (0, 0, 255, 255)]]
    );
    display.assert_no_error(None);
}

/// Bytes into floating-point textures are normalized.
#[test]
fn bytes_into_floats() {
    let display = support::build_display();
    // values that half floats hold exactly
    let data = vec![vec![(255u8, 0u8, 255u8, 255u8), (0, 255, 0, 0)]];
    let texture = Texture2d::with_format(
        &display,
        data,
        UncompressedFloatFormat::F16F16F16F16,
        MipmapsOption::NoMipmap,
    );
    // reading floating-point textures needs color-renderable floats on OpenGL ES
    let Ok(texture) = texture else {
        return;
    };
    let image: safegl::texture::TextureAnyImage<'_> = texture.main_level().into();
    let pixels: Result<Vec<Vec<(f32, f32, f32, f32)>>, _> = image.raw_read(&WHOLE);
    if let Ok(pixels) = pixels {
        assert_eq!(pixels, [[(1.0, 0.0, 1.0, 1.0), (0.0, 1.0, 0.0, 0.0)]]);
    }
    display.assert_no_error(None);
}

/// The sRGB formats of one component take RGBA bytes too.
#[test]
fn bytes_into_srgb_red() {
    let display = support::build_display();
    let data = vec![vec![(255u8, 1u8, 2u8, 3u8), (0, 4, 5, 6)]];
    let texture =
        SrgbTexture2d::with_format(&display, data, SrgbFormat::U8, MipmapsOption::NoMipmap);
    // the format needs `GL_EXT_texture_sRGB_R8` on OpenGL ES
    if let Ok(texture) = texture {
        // OpenGL ES reads through framebuffers, to which the format can't be attached
        match texture.read() {
            Ok(pixels) => {
                let pixels: Vec<Vec<Pixel>> = pixels;
                assert_eq!((pixels[0][0].0, pixels[0][1].0), (255, 0));
            }
            Err(safegl::ReadError::NotRenderable) => assert!(support::is_gles(&display)),
            Err(error) => panic!("{error}"),
        }
    }
    display.assert_no_error(None);
}

/// Unsigned integers of another size into an integer texture keep their values; floats are
/// refused, since no exact conversion exists.
#[test]
fn integer_textures() {
    let display = support::build_display();
    let data = vec![vec![7u32, 200u32]];
    let texture = UnsignedTexture2d::with_format(
        &display,
        data,
        UncompressedUintFormat::U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();
    let image: safegl::texture::TextureAnyImage<'_> = texture.main_level().into();
    let read: Result<Vec<Vec<(u32, u32, u32, u32)>>, _> = image.raw_read(&WHOLE);
    if let Ok(read) = read {
        assert_eq!((read[0][0].0, read[0][1].0), (7, 200));
    }
    display.assert_no_error(None);
}

/// Writes of rectangles convert their data too.
#[test]
fn writes_are_converted() {
    let display = support::build_display();
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8,
        MipmapsOption::NoMipmap,
        2,
        1,
    )
    .unwrap();
    let rect = Rect {
        left: 0,
        bottom: 0,
        width: 2,
        height: 1,
    };
    texture
        .write(rect, vec![vec![(30u8, 1u8, 2u8, 3u8), (60, 4, 5, 6)]])
        .unwrap();
    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    assert_eq!(pixels, [[(30, 0, 0, 255), (60, 0, 0, 255)]]);
    let image = RawImage2d::from_raw_rgba(vec![0.5f32, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0], (2, 1));
    texture.write(rect, image).unwrap();
    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    assert_eq!(pixels, [[(128, 0, 0, 255), (255, 0, 0, 255)]]);
    // the clears go through framebuffers, which every format has
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);
    display.assert_no_error(None);
}
