#[macro_use]
extern crate safegl;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn test_read() {
    let display = support::build_display();

    let texture =
        safegl::texture::Texture2d::empty(&display, 512, 512).expect("fail when creating texture");

    let image = texture.main_level().first_layer().into_image(None).unwrap();

    let rect = safegl::Rect {
        left: 0,
        bottom: 4_286_579_200,
        width: 512,
        height: 8_388_608,
    };

    // the rectangle is out of the image, even if its corners overflow
    let pixels: Result<Vec<Vec<(u8, u8, u8, u8)>>, _> = image.raw_read(&rect);
    assert!(matches!(pixels, Err(safegl::ReadError::OutOfBounds)));
}

#[test]
fn read_front_buffer_without_error() {
    let display = support::build_display();

    // EGL window surfaces usually have no front buffer, which used to raise an OpenGL error
    match display.read_front_buffer::<Vec<Vec<(u8, u8, u8, u8)>>>() {
        Ok(_) | Err(safegl::ReadError::AttachmentMissing) => (),
        Err(e) => panic!("{e:?}"),
    }
    display.assert_no_error(None);
}
