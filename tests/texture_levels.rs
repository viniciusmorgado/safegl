//! Whole levels of every kind of texture read with `read_volume` (`glGetTexImage`), and the
//! whole-texture reads of 2D textures, which use it on desktop OpenGL.
#![cfg(safegl_desktop)]

#[macro_use]
extern crate safegl;

use safegl::framebuffer::SimpleFrameBuffer;
use safegl::texture::{
    CompressedFormat, CompressedMipmapsOption, CompressedTexture2d, CubeLayer, Cubemap,
    DepthStencilTexture2d, DepthTexture2d, IntegralTexture2d, MipmapsOption, RawImage2d,
    Texture1d, Texture1dArray, Texture2d, Texture2dArray, Texture3d,
    TextureRectangle, UncompressedFloatFormat,
};
use safegl::{ReadError, Surface};

mod support;

type Volume<P> = Vec<Vec<Vec<P>>>;

/// Returns true on desktop OpenGL, which reads levels with `glGetTexImage`.
fn desktop(display: &support::WindowDisplay) -> bool {
    !support::is_gles(display)
}

#[test]
fn levels_of_every_dimension() {
    let display = support::build_display();
    if !desktop(&display) {
        return;
    }
    let texel = |i: u8| (i, i + 1, i + 2, 255u8);

    let one = Texture1d::new(&display, vec![texel(0), texel(10)]).unwrap();
    let read: Volume<(u8, u8, u8, u8)> = one.main_level().read_volume().unwrap();
    assert_eq!(read, vec![vec![vec![texel(0), texel(10)]]]);

    // the layers of a 1D array are its rows
    let layers = Texture1dArray::new(&display, vec![vec![texel(0)], vec![texel(20)]]).unwrap();
    let read: Volume<(u8, u8, u8, u8)> = layers.main_level().read_volume().unwrap();
    assert_eq!(read, vec![vec![vec![texel(0)], vec![texel(20)]]]);

    let array = Texture2dArray::new(&display, vec![vec![vec![texel(0)]], vec![vec![texel(30)]]])
        .unwrap();
    let read: Volume<(u8, u8, u8, u8)> = array.main_level().read_volume().unwrap();
    assert_eq!(read, vec![vec![vec![texel(0)]], vec![vec![texel(30)]]]);

    let volume = Texture3d::new(&display, vec![vec![vec![texel(0)]], vec![vec![texel(40)]]])
        .unwrap();
    let read: Volume<(u8, u8, u8, u8)> = volume.main_level().read_volume().unwrap();
    assert_eq!(read, vec![vec![vec![texel(0)]], vec![vec![texel(40)]]]);

    let rectangle = TextureRectangle::new(&display, vec![vec![texel(50), texel(60)]]).unwrap();
    let read: Volume<(u8, u8, u8, u8)> = rectangle.main_level().read_volume().unwrap();
    assert_eq!(read, vec![vec![vec![texel(50), texel(60)]]]);

    // the faces of a cube map, read one by one, in the order of `CubeLayer`
    let cube = Cubemap::empty(&display, 1).unwrap();
    let faces = [
        CubeLayer::PositiveX,
        CubeLayer::NegativeX,
        CubeLayer::PositiveY,
        CubeLayer::NegativeY,
        CubeLayer::PositiveZ,
        CubeLayer::NegativeZ,
    ];
    for (i, face) in faces.into_iter().enumerate() {
        let red = i as f32 / 255.0;
        SimpleFrameBuffer::new(&display, cube.main_level().image(face))
            .unwrap()
            .clear_color(red, 0.0, 0.0, 1.0);
    }
    let read: Volume<(u8, u8, u8, u8)> = cube.main_level().read_volume().unwrap();
    let reds: Vec<u8> = read.iter().map(|face| face[0][0].0).collect();
    assert_eq!(reds, vec![0, 1, 2, 3, 4, 5]);

    display.assert_no_error(None);
}

#[test]
fn levels_of_every_kind_of_format() {
    let display = support::build_display();
    if !desktop(&display) {
        return;
    }
    let color = Texture2d::empty(&display, 2, 2).unwrap();

    let integral = IntegralTexture2d::new(&display, vec![vec![(-3i8, 4i8, -5i8, 6i8)]]).unwrap();
    let read: Volume<(i32, i32, i32, i32)> = integral.main_level().read_volume().unwrap();
    assert_eq!(read[0][0][0], (-3, 4, -5, 6));
    // the 2D reads of signed integers
    let read: Vec<Vec<(i32, i32, i32, i32)>> = integral.read_as().unwrap();
    assert_eq!(read[0][0], (-3, 4, -5, 6));

    let depth = DepthTexture2d::empty(&display, 2, 2).unwrap();
    SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth)
        .unwrap()
        .clear_depth(0.5);
    let read: Volume<f32> = depth.main_level().read_volume().unwrap();
    assert!((read[0][1][1] - 0.5).abs() < 1e-3);

    let depth_stencil = DepthStencilTexture2d::empty(&display, 2, 2).unwrap();
    SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &depth_stencil)
        .unwrap()
        .clear_depth_and_stencil(1.0, 7);
    let read: Volume<u32> = depth_stencil.main_level().read_volume().unwrap();
    assert_eq!(read[0][0][0], (0xff_ffff << 8) | 7);

    // stencil textures are core in OpenGL 4.4, which their type needs
    #[cfg(safegl_gl44)]
    if let Ok(stencil) = safegl::texture::StencilTexture2d::empty(&display, 2, 2) {
        SimpleFrameBuffer::with_stencil_buffer(&display, &color, &stencil)
            .unwrap()
            .clear_stencil(9);
        let read: Volume<u8> = stencil.main_level().read_volume().unwrap();
        assert_eq!(read[0][1][0], 9);
    }

    display.assert_no_error(None);
}

/// Formats that can't be rendered to are read on desktop OpenGL, where reading them through a
/// framebuffer returned zeros, and refused on OpenGL ES.
#[test]
fn formats_that_cant_be_rendered_to() {
    let display = support::build_display();

    // red in DXT1: two equal 5:6:5 colors and every index 0
    let block = [0x00, 0xf8, 0x00, 0xf8, 0, 0, 0, 0];
    if let Ok(compressed) = CompressedTexture2d::with_compressed_data(
        &display,
        &block,
        4,
        4,
        CompressedFormat::S3tcDxt1NoAlpha,
        CompressedMipmapsOption::NoMipmap,
    ) {
        match compressed.read::<Vec<Vec<(u8, u8, u8, u8)>>>() {
            Ok(read) => {
                assert!(desktop(&display));
                assert_eq!(read[3][3], (255, 0, 0, 255));
                let buffer = compressed.read_to_pixel_buffer().unwrap();
                let read: Vec<Vec<(u8, u8, u8, u8)>> = buffer.read_as_texture_2d().unwrap();
                assert_eq!(read[0][0], (255, 0, 0, 255));
            }
            Err(ReadError::NotRenderable) => assert!(!desktop(&display)),
            Err(error) => panic!("{error:?}"),
        }
    }

    let shared_exponent = Texture2d::with_format(
        &display,
        RawImage2d::from_raw_rgb(vec![1.0f32, 0.5, 0.25], (1, 1)),
        UncompressedFloatFormat::F9F9F9,
        MipmapsOption::NoMipmap,
    )
    .unwrap();
    match shared_exponent.read_as::<Vec<Vec<(f32, f32, f32)>>, _>() {
        Ok(read) => {
            assert!(desktop(&display));
            assert_eq!(read[0][0], (1.0, 0.5, 0.25));
        }
        Err(ReadError::NotRenderable) => assert!(!desktop(&display)),
        Err(error) => panic!("{error:?}"),
    }

    display.assert_no_error(None);
}

/// Multisample textures have no level to read.
#[test]
fn multisample_levels_are_not_read() {
    let display = support::build_display();
    if !desktop(&display) {
        return;
    }
    let texture = safegl::texture::Texture2dMultisample::empty(&display, 2, 2, 2).unwrap();
    assert!(matches!(
        texture.main_level().read_volume::<(u8, u8, u8, u8), Volume<_>>(),
        Err(ReadError::NotSupported)
    ));
}

/// Reads the level of a texture of each type built from data, and compares it with the data as
/// `read_volume` returns it.
macro_rules! level_test {
    ($name:ident, $texture:ident, $pixel:ty, $data:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let display = support::build_display();
            if !desktop(&display) {
                return;
            }
            let texture = match safegl::texture::$texture::new(&display, $data) {
                Err(safegl::texture::TextureCreationError::FormatNotSupported)
                | Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
                result => result.unwrap(),
            };
            let read: Volume<$pixel> = texture.main_level().read_volume().unwrap();
            assert_eq!(read, $expected);
            display.assert_no_error(None);
        }
    };
}

level_test!(level_texture1d, Texture1d, (u8, u8, u8, u8),
    vec![(0u8, 1, 2, 3), (4, 5, 6, 7)], vec![vec![vec![(0, 1, 2, 3), (4, 5, 6, 7)]]]);
level_test!(level_texture1darray, Texture1dArray, (u8, u8, u8, u8),
    vec![vec![(0u8, 1, 2, 3)], vec![(4, 5, 6, 7)]],
    vec![vec![vec![(0, 1, 2, 3)], vec![(4, 5, 6, 7)]]]);
level_test!(level_texture2d, Texture2d, (u8, u8, u8, u8),
    vec![vec![(0u8, 1, 2, 3), (4, 5, 6, 7)], vec![(8, 9, 10, 11), (12, 13, 14, 15)]],
    vec![vec![vec![(0, 1, 2, 3), (4, 5, 6, 7)], vec![(8, 9, 10, 11), (12, 13, 14, 15)]]]);
level_test!(level_texture3d, Texture3d, (u8, u8, u8, u8),
    vec![vec![vec![(0u8, 1, 2, 3)]], vec![vec![(4, 5, 6, 7)]]],
    vec![vec![vec![(0, 1, 2, 3)]], vec![vec![(4, 5, 6, 7)]]]);
// sRGB texels are read as they are stored, without conversion
level_test!(level_srgbtexture2d, SrgbTexture2d, (u8, u8, u8, u8),
    vec![vec![(10u8, 100, 200, 255)]], vec![vec![vec![(10, 100, 200, 255)]]]);
level_test!(level_integraltexture1d, IntegralTexture1d, (i16, i16, i16, i16),
    vec![(-1i16, 2, -3, 4)], vec![vec![vec![(-1, 2, -3, 4)]]]);
level_test!(level_integraltexture2darray, IntegralTexture2dArray, (i16, i16, i16, i16),
    vec![vec![vec![(-1i16, 2, -3, 4)]], vec![vec![(5, -6, 7, -8)]]],
    vec![vec![vec![(-1, 2, -3, 4)]], vec![vec![(5, -6, 7, -8)]]]);
level_test!(level_integraltexture3d, IntegralTexture3d, (i16, i16, i16, i16),
    vec![vec![vec![(-1i16, 2, -3, 4)]], vec![vec![(5, -6, 7, -8)]]],
    vec![vec![vec![(-1, 2, -3, 4)]], vec![vec![(5, -6, 7, -8)]]]);
level_test!(level_unsignedtexture1darray, UnsignedTexture1dArray, (u16, u16, u16, u16),
    vec![vec![(1u16, 2, 3, 4)], vec![(500, 600, 700, 800)]],
    vec![vec![vec![(1, 2, 3, 4)], vec![(500, 600, 700, 800)]]]);
level_test!(level_unsignedtexture3d, UnsignedTexture3d, (u16, u16, u16, u16),
    vec![vec![vec![(1u16, 2, 3, 4)]], vec![vec![(500, 600, 700, 800)]]],
    vec![vec![vec![(1, 2, 3, 4)]], vec![vec![(500, 600, 700, 800)]]]);
// depths are stored as normalized integers, where 0 and 1 are exact
level_test!(level_depthtexture1d, DepthTexture1d, f32,
    vec![0.0f32, 1.0], vec![vec![vec![0.0, 1.0]]]);
level_test!(level_depthtexture2darray, DepthTexture2dArray, f32,
    vec![vec![vec![0.0f32]], vec![vec![1.0]]], vec![vec![vec![0.0]], vec![vec![1.0]]]);
