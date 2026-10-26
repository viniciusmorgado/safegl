#[macro_use]
extern crate safegl;

use safegl::framebuffer::{DepthRenderBuffer, RenderBuffer};
use safegl::texture::{
    DepthFormat, DepthStencilFormat, DepthStencilTexture2d, DepthTexture2d, IntegralTexture2d,
    MipmapsOption, Texture1d, Texture2d, TextureCreationError, UncompressedFloatFormat,
    UncompressedIntFormat, UncompressedUintFormat, UnsignedTexture2d,
};
use safegl::{CapabilitiesSource, Surface};

mod support;

const SIZE: u32 = 4;

#[test]
fn empty_textures_of_every_kind() {
    let display = support::build_display();

    for format in [
        UncompressedFloatFormat::U8,
        UncompressedFloatFormat::U8U8,
        UncompressedFloatFormat::U8U8U8,
        UncompressedFloatFormat::U8U8U8U8,
        UncompressedFloatFormat::F16F16F16F16,
        UncompressedFloatFormat::F32,
        UncompressedFloatFormat::F11F11F10,
    ] {
        Texture2d::empty_with_format(&display, format, MipmapsOption::NoMipmap, SIZE, SIZE)
            .unwrap_or_else(|e| panic!("{format:?}: {e}"));
    }

    for format in [
        UncompressedIntFormat::I8,
        UncompressedIntFormat::I32I32I32I32,
    ] {
        IntegralTexture2d::empty_with_format(&display, format, MipmapsOption::NoMipmap, SIZE, SIZE)
            .unwrap_or_else(|e| panic!("{format:?}: {e}"));
    }

    for format in [
        UncompressedUintFormat::U8,
        UncompressedUintFormat::U16U16,
        UncompressedUintFormat::U32U32U32U32,
        UncompressedUintFormat::U10U10U10U2,
    ] {
        UnsignedTexture2d::empty_with_format(&display, format, MipmapsOption::NoMipmap, SIZE, SIZE)
            .unwrap_or_else(|e| panic!("{format:?}: {e}"));
    }

    for format in [DepthFormat::I16, DepthFormat::I24, DepthFormat::F32] {
        DepthTexture2d::empty_with_format(&display, format, MipmapsOption::NoMipmap, SIZE, SIZE)
            .unwrap_or_else(|e| panic!("{format:?}: {e}"));
    }

    for format in [DepthStencilFormat::I24I8, DepthStencilFormat::F32I8] {
        DepthStencilTexture2d::empty_with_format(
            &display,
            format,
            MipmapsOption::NoMipmap,
            SIZE,
            SIZE,
        )
        .unwrap_or_else(|e| panic!("{format:?}: {e}"));
    }

    display.assert_no_error(None);
}

#[test]
fn textures_from_data_with_fewer_than_four_components() {
    let display = support::build_display();

    let red = Texture2d::new(&display, vec![vec![255u8; 2]; 2]).unwrap();
    let red_green = Texture2d::new(&display, vec![vec![(255u8, 128u8); 2]; 2]).unwrap();
    let rgb = Texture2d::new(&display, vec![vec![(255u8, 128u8, 64u8); 2]; 2]).unwrap();

    let read = |texture: &Texture2d| -> (u8, u8, u8, u8) {
        let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
        data[1][1]
    };
    assert_eq!(read(&red), (255, 0, 0, 255));
    assert_eq!(read(&red_green), (255, 128, 0, 255));
    assert_eq!(read(&rgb), (255, 128, 64, 255));

    display.assert_no_error(None);
}

#[test]
fn unsigned_texture_from_bytes() {
    let display = support::build_display();

    // OpenGL ES only accepts bytes for an 8-bit integer format
    let texture = UnsignedTexture2d::new(&display, vec![vec![(1u8, 2u8, 3u8, 4u8); 2]; 2]).unwrap();
    let data: Vec<Vec<(u32, u32, u32, u32)>> = unsafe { texture.unchecked_read().unwrap() };
    assert_eq!(data[0][0], (1, 2, 3, 4));

    display.assert_no_error(None);
}

#[test]
fn depth_texture_from_floats_with_mipmaps() {
    let display = support::build_display();

    // the mipmaps of a depth texture can't be generated on OpenGL ES, where it gets one level
    let texture = DepthTexture2d::new(&display, vec![vec![0.5f32; 8]; 8]).unwrap();
    if support::is_gles(&display) {
        assert_eq!(texture.get_mipmap_levels(), 1);
    }

    display.assert_no_error(None);
}

#[test]
fn oversized_texture() {
    let display = support::build_display();

    let max = display.get_capabilities().max_texture_size as u32;
    assert!(matches!(
        Texture2d::empty(&display, max + 1, 1),
        Err(TextureCreationError::DimensionsNotSupported)
    ));

    display.assert_no_error(None);
}

#[test]
fn one_dimensional_textures_on_opengl_es() {
    let display = support::build_display();

    let texture = Texture1d::empty(&display, SIZE);
    if support::is_gles(&display) {
        assert!(matches!(
            texture,
            Err(TextureCreationError::TypeNotSupported)
        ));
    } else {
        texture.unwrap();
    }

    display.assert_no_error(None);
}

#[test]
fn render_buffers() {
    let display = support::build_display();

    // depth render buffers exist on OpenGL ES too
    DepthRenderBuffer::new(&display, DepthFormat::I24, SIZE, SIZE).unwrap();
    RenderBuffer::new(&display, UncompressedFloatFormat::U8U8U8U8, SIZE, SIZE).unwrap();

    // a shared-exponent format can't be rendered to
    assert!(RenderBuffer::new(&display, UncompressedFloatFormat::F9F9F9, SIZE, SIZE).is_err());

    display.assert_no_error(None);
}

#[test]
fn rgb565() {
    let display = support::build_display();
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U5U6U5,
        MipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap();

    // RGB565 is color-renderable, and its extreme values are exact
    texture.as_surface().clear_color(1.0, 0.0, 1.0, 1.0);
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (255, 0, 255, 255));
    display.assert_no_error(None);
}
