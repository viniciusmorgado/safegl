//! Textures written from the images of browsers: image data and canvases, with the conversions of
//! the pixel store of WebGL.
#![cfg(all(target_arch = "wasm32", feature = "webgl"))]

#[macro_use]
extern crate safegl;

use safegl::texture::{
    MipmapsOption, SrgbTexture2d, Texture2d, UncompressedFloatFormat, UploadError,
    WebImageError, WebImageOptions,
};
use safegl::Surface;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test as test;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData, WebGl2RenderingContext};

mod support;

type Pixel = (u8, u8, u8, u8);

/// A 2 × 2 image: its top row (first in the image) is red and green, its bottom row blue and
/// white, half transparent.
fn image_data() -> ImageData {
    let pixels: [u8; 16] = [
        255, 0, 0, 255, 0, 255, 0, 255, // top
        0, 0, 255, 128, 255, 255, 255, 128, // bottom
    ];
    // a copy in the memory of JavaScript: image data keeps the array that it is given, and a view
    // of the memory of the program would see it reused
    let array = js_sys::Uint8ClampedArray::from(&pixels[..]);
    ImageData::new_with_js_u8_clamped_array_and_sh(&array, 2, 2).unwrap()
}

const RED: Pixel = (255, 0, 0, 255);
const GREEN: Pixel = (0, 255, 0, 255);
const BLUE: Pixel = (0, 0, 255, 128);
const WHITE: Pixel = (255, 255, 255, 128);

/// Reads the rows of a texture, the bottom one first.
fn read(texture: &Texture2d) -> Vec<Vec<Pixel>> {
    texture.read().unwrap()
}

/// Without options, the first row of the image is the first row of the texture: images are
/// written top-down, textures are read bottom-up.
#[test]
fn image_data_default() {
    let display = support::build_display();
    let texture = Texture2d::from_web_image(
        &display,
        &image_data(),
        MipmapsOption::NoMipmap,
        WebImageOptions::default(),
    )
    .unwrap();
    assert_eq!((texture.width(), texture.height()), (2, 2));
    assert_eq!(read(&texture), vec![vec![RED, GREEN], vec![BLUE, WHITE]]);

    display.assert_no_error(None);
}

/// `flip_y` puts the top row of the image at the top of the texture.
#[test]
fn image_data_flipped() {
    let display = support::build_display();
    let options = WebImageOptions {
        flip_y: true,
        ..Default::default()
    };
    let texture =
        Texture2d::from_web_image(&display, &image_data(), MipmapsOption::NoMipmap, options)
            .unwrap();
    assert_eq!(read(&texture), vec![vec![BLUE, WHITE], vec![RED, GREEN]]);

    // the options don't outlive the write
    texture.write_web_image(0, 0, &image_data(), Default::default()).unwrap();
    assert_eq!(read(&texture), vec![vec![RED, GREEN], vec![BLUE, WHITE]]);

    display.assert_no_error(None);
}

/// `premultiply_alpha` multiplies the colors by the alpha.
#[test]
fn image_data_premultiplied() {
    let display = support::build_display();
    let options = WebImageOptions {
        premultiply_alpha: true,
        ..Default::default()
    };
    let texture =
        Texture2d::from_web_image(&display, &image_data(), MipmapsOption::NoMipmap, options)
            .unwrap();
    let rows = read(&texture);
    assert_eq!(rows[0], vec![RED, GREEN]);
    // 255 × 128 / 255, rounded either way
    let (r, g, b, a) = rows[1][1];
    assert!((127..=128).contains(&r) && r == g && g == b, "{:?}", rows[1][1]);
    assert_eq!(a, 128);
    let (r, _, b, _) = rows[1][0];
    assert_eq!(r, 0);
    assert!((127..=128).contains(&b));

    display.assert_no_error(None);
}

/// Without the conversion of colors, the values of the image are kept; image data is in the
/// colors of the browser, so they are kept with it too.
#[test]
fn image_data_unconverted() {
    let display = support::build_display();
    let options = WebImageOptions {
        colorspace_conversion: false,
        ..Default::default()
    };
    let texture =
        Texture2d::from_web_image(&display, &image_data(), MipmapsOption::NoMipmap, options)
            .unwrap();
    assert_eq!(read(&texture), vec![vec![RED, GREEN], vec![BLUE, WHITE]]);

    display.assert_no_error(None);
}

/// A canvas drawn by the 2D context of the browser, written to a part of a texture, and to an sRGB
/// texture with mipmaps.
#[test]
fn canvas() {
    let display = support::build_display();
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = document.create_element("canvas").unwrap().unchecked_into();
    canvas.set_width(4);
    canvas.set_height(2);
    let context: CanvasRenderingContext2d =
        canvas.get_context("2d").unwrap().unwrap().unchecked_into();
    // the left half blue, the right half yellow
    context.set_fill_style_str("#0000ff");
    context.fill_rect(0.0, 0.0, 2.0, 2.0);
    context.set_fill_style_str("#ffff00");
    context.fill_rect(2.0, 0.0, 2.0, 2.0);

    let texture = Texture2d::empty(&display, 8, 4).unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture.write_web_image(3, 1, &canvas, Default::default()).unwrap();
    let rows = read(&texture);
    let (blue, yellow, empty) = ((0, 0, 255, 255), (255, 255, 0, 255), (0, 0, 0, 0));
    assert_eq!(rows[0][3], empty);
    for row in &rows[1..3] {
        assert_eq!(row[2], empty);
        assert_eq!(&row[3..7], &[blue, blue, yellow, yellow]);
        assert_eq!(row[7], empty);
    }
    assert_eq!(rows[3][3], empty);

    let srgb = SrgbTexture2d::from_web_image(
        &display,
        &canvas,
        MipmapsOption::AutoGeneratedMipmaps,
        Default::default(),
    )
    .unwrap();
    assert_eq!(srgb.get_mipmap_levels(), 3);
    let texels: Vec<Vec<Pixel>> = srgb.read().unwrap();
    assert_eq!(texels[0], vec![blue, blue, yellow, yellow]);

    display.assert_no_error(None);
}

/// Images that don't fit, and textures that aren't of `U8U8U8U8` texels, are errors.
#[test]
fn refused_writes() {
    let display = support::build_display();
    let small = Texture2d::empty(&display, 2, 2).unwrap();
    assert_eq!(
        small.write_web_image(1, 0, &image_data(), Default::default()),
        Err(UploadError::OutOfBounds)
    );

    let float = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::F16F16F16F16,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();
    assert_eq!(
        float.write_web_image(0, 0, &image_data(), Default::default()),
        Err(UploadError::FormatNotSupported)
    );

    // a canvas without pixels can't make a texture
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = document.create_element("canvas").unwrap().unchecked_into();
    canvas.set_width(0);
    canvas.set_height(0);
    let result = Texture2d::from_web_image(
        &display,
        &canvas,
        MipmapsOption::NoMipmap,
        Default::default(),
    );
    assert!(matches!(result, Err(WebImageError::Creation(_))), "{result:?}");

    display.assert_no_error(None);
}

/// `restore_state` resets the pixel store of WebGL that other code changed.
#[test]
fn restored_pixel_store() {
    let display = support::build_display();
    let gl: WebGl2RenderingContext = display
        .canvas()
        .get_context("webgl2")
        .unwrap()
        .unwrap()
        .unchecked_into();
    gl.pixel_storei(WebGl2RenderingContext::UNPACK_FLIP_Y_WEBGL, 1);
    gl.pixel_storei(WebGl2RenderingContext::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);
    display.restore_state();
    let flip = gl.get_parameter(WebGl2RenderingContext::UNPACK_FLIP_Y_WEBGL).unwrap();
    let premultiply = gl
        .get_parameter(WebGl2RenderingContext::UNPACK_PREMULTIPLY_ALPHA_WEBGL)
        .unwrap();
    assert_eq!((flip.as_bool(), premultiply.as_bool()), (Some(false), Some(false)));
    let conversion = gl
        .get_parameter(WebGl2RenderingContext::UNPACK_COLORSPACE_CONVERSION_WEBGL)
        .unwrap();
    assert_eq!(
        conversion.as_f64(),
        Some(WebGl2RenderingContext::BROWSER_DEFAULT_WEBGL as f64)
    );

    display.assert_no_error(None);
}
