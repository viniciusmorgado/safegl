#[macro_use]
extern crate safegl;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
#[should_panic]
fn empty_pixel_buffer() {
    let display = support::build_display();

    let pixel_buffer = safegl::pixel_buffer::PixelBuffer::new_empty(&display, 128 * 128);
    display.assert_no_error(None);

    let _: Vec<Vec<(u8, u8, u8, u8)>> = pixel_buffer.read_as_texture_2d().unwrap();
}

#[test]
fn texture_2d_read_pixelbuffer() {
    let display = support::build_display();

    // we use only powers of two, in order to avoid float rounding errors
    let texture = safegl::texture::Texture2d::new(
        &display,
        vec![
            vec![(0u8, 1u8, 2u8), (4u8, 8u8, 16u8)],
            vec![(32u8, 64u8, 128u8), (32u8, 16u8, 4u8)],
        ],
    )
    .unwrap();

    let read_back: Vec<Vec<(u8, u8, u8, u8)>> =
        match texture.read_to_pixel_buffer().unwrap().read_as_texture_2d() {
            Ok(r) => r,
            Err(safegl::buffer::ReadError::NotSupported) => return,
            e => e.unwrap(),
        };

    assert_eq!(read_back[0][0], (0, 1, 2, 255));
    assert_eq!(read_back[0][1], (4, 8, 16, 255));
    assert_eq!(read_back[1][0], (32, 64, 128, 255));
    assert_eq!(read_back[1][1], (32, 16, 4, 255));

    display.assert_no_error(None);
}

macro_rules! read_texture_test {
    ($test_name:ident, $tex_ty:ident, $data_ty:ty, $data:expr) => {
        #[test]
        fn $test_name() {
            let display = support::build_display();

            let texture = match safegl::texture::$tex_ty::new(&display, $data) {
                Err(safegl::texture::TextureCreationError::FormatNotSupported)
                | Err(safegl::texture::TextureCreationError::DimensionsNotSupported)
                | Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
                Err(e) => panic!("{:?}", e),
                Ok(t) => t,
            };

            let read_back: Vec<Vec<$data_ty>> = texture.read().unwrap();

            assert_eq!(read_back, $data);

            display.assert_no_error(None);
        }
    };
}

read_texture_test!(
    read_texture2d,
    Texture2d,
    (u8, u8, u8, u8),
    vec![
        vec![(0u8, 1u8, 2u8, 3u8), (4u8, 5u8, 6u8, 7u8)],
        vec![(8u8, 9u8, 10u8, 11u8), (12u8, 13u8, 14u8, 15u8)]
    ]
);

#[test]
fn read_float_values_exactly() {
    let display = support::build_display();

    let texture = match safegl::texture::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::F32F32F32F32,
        safegl::texture::MipmapsOption::NoMipmap,
        2,
        2,
    ) {
        Ok(texture) => texture,
        Err(safegl::texture::TextureCreationError::TypeNotSupported)
            | Err(safegl::texture::TextureCreationError::FormatNotSupported) => return,
        Err(e) => panic!("{:?}", e),
    };
    let texels = vec![vec![(1.5f32, -2.0f32, 1.0e6f32, 0.25f32); 2]; 2];
    texture
        .write(
            safegl::Rect {
                left: 0,
                bottom: 0,
                width: 2,
                height: 2,
            },
            texels.clone(),
        )
        .unwrap();

    // read as floats, the values are exact
    let floats: Vec<Vec<(f32, f32, f32, f32)>> = texture.read_as().unwrap();
    assert_eq!(floats, texels);

    // read as bytes, they are clamped and normalized, on OpenGL ES too
    let bytes: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(bytes[0][0], (255, 0, 255, 64));

    display.assert_no_error(None);
}

#[test]
fn read_bytes_as_floats() {
    let display = support::build_display();

    let texture = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    let floats: Vec<Vec<(f32, f32, f32, f32)>> = texture.read_as().unwrap();
    assert_eq!(floats[0][0], (1.0, 0.0, 0.0, 1.0));

    // and into a pixel buffer
    let buffer = texture.read_to_pixel_buffer_as::<(f32, f32, f32, f32)>().unwrap();
    let floats: Vec<Vec<(f32, f32, f32, f32)>> = buffer.read_as_texture_2d().unwrap();
    assert_eq!(floats[0][0], (1.0, 0.0, 0.0, 1.0));

    display.assert_no_error(None);
}
