//! Transfers between boxes of texture levels and boxes of larger client images, through the
//! row length, image height and skips of the pixel store.

#[macro_use]
extern crate safegl;

use safegl::texture::{CubeLayer, Cubemap, Texture2d, Texture2dArray, TextureAnyImage};
use safegl::{Rect, Surface};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Rgba = (u8, u8, u8, u8);

/// A client image of `width × height × depth` texels whose texels are all different.
fn image(width: u32, height: u32, depth: u32) -> Vec<Rgba> {
    (0..width * height * depth)
        .map(|i| (i as u8, (i * 3) as u8, (i * 7) as u8, 255))
        .collect()
}

/// The texel at `(x, y, z)` of a client image of `width × height` texels per slice.
fn texel(image: &[Rgba], width: u32, height: u32, (x, y, z): (u32, u32, u32)) -> Rgba {
    image[(z * width * height + y * width + x) as usize]
}

/// Reads a 2D texture through a framebuffer, which every backend can.
fn read(texture: &Texture2d) -> Vec<Vec<Rgba>> {
    texture.read().unwrap()
}

#[test]
fn write_a_box_of_a_larger_image() {
    let display = support::build_display();
    let texture = Texture2d::empty(&display, 4, 4).unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    let source = image(5, 4, 1);

    // 2 × 2 texels from (1, 2) of the 5 × 4 image to (2, 1) of the texture
    texture
        .main_level()
        .write_box_from((2, 1, 0), (2, 2, 1), &source, (5, 4, 1), (1, 2, 0))
        .unwrap();
    let read = read(&texture);
    for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let expected = texel(&source, 5, 4, (1 + x, 2 + y, 0));
        assert_eq!(read[(1 + y) as usize][(2 + x) as usize], expected, "({x}, {y})");
    }
    assert_eq!(read[0][0], (0, 0, 0, 0));
    assert_eq!(read[3][3], (0, 0, 0, 0));

    // boxes outside their images and sources of the wrong size are refused
    let level = texture.main_level();
    assert!(level.write_box_from((3, 3, 0), (2, 2, 1), &source, (5, 4, 1), (0, 0, 0)).is_err());
    assert!(level.write_box_from((0, 0, 0), (2, 2, 1), &source, (5, 4, 1), (4, 0, 0)).is_err());
    assert!(level.write_box_from((0, 0, 0), (1, 1, 1), &source[1..], (5, 4, 1), (0, 0, 0)).is_err());

    display.assert_no_error(None);
}

#[test]
fn write_slices_and_faces_of_a_larger_image() {
    let display = support::build_display();
    let source = image(2, 2, 3);

    // the slices 1 and 2 of the image to the layers 0 and 1 of an array
    let array = Texture2dArray::empty(&display, 2, 2, 2).unwrap();
    array
        .main_level()
        .write_box_from((0, 0, 0), (2, 2, 2), &source, (2, 2, 3), (0, 0, 1))
        .unwrap();
    for layer in 0..2u32 {
        let image: TextureAnyImage<'_> = array.main_level().layer(layer).unwrap().into();
        let rect = Rect { left: 0, bottom: 0, width: 2, height: 2 };
        let read: Vec<Vec<Rgba>> = image.raw_read(&rect).unwrap();
        assert_eq!(read[1][0], texel(&source, 2, 2, (0, 1, layer + 1)), "layer {layer}");
    }

    // the slices 0 and 1 of the image to the faces 2 and 3 of a cube map
    let cube = Cubemap::empty(&display, 2).unwrap();
    cube.main_level()
        .write_box_from((0, 0, 2), (2, 2, 2), &source, (2, 2, 3), (0, 0, 0))
        .unwrap();
    for (face, slice) in [(CubeLayer::PositiveY, 0), (CubeLayer::NegativeY, 1)] {
        let image: TextureAnyImage<'_> = cube.main_level().image(face).into();
        let rect = Rect { left: 0, bottom: 0, width: 2, height: 2 };
        let read: Vec<Vec<Rgba>> = image.raw_read(&rect).unwrap();
        assert_eq!(read[1][1], texel(&source, 2, 2, (1, 1, slice)), "face {face:?}");
    }

    display.assert_no_error(None);
}

/// Blocks of a larger compressed image are written without copying them out of it.
#[cfg(safegl_gl42)]
#[test]
fn write_blocks_of_a_larger_compressed_image() {
    use safegl::texture::{
        CompressedFormat, CompressedMipmapsOption, CompressedTexture2d, UploadError,
    };

    let display = support::build_display();
    // an 8 × 4 image of two DXT1 blocks: red, then green (5:6:5 colors, every index 0)
    let blocks = [0x00, 0xf8, 0x00, 0xf8, 0, 0, 0, 0, 0xe0, 0x07, 0xe0, 0x07, 0, 0, 0, 0];
    let texture = match CompressedTexture2d::empty_with_format(
        &display,
        CompressedFormat::S3tcDxt1NoAlpha,
        CompressedMipmapsOption::NoMipmap,
        4,
        4,
    ) {
        Ok(texture) => texture,
        // without S3TC
        Err(_) => return,
    };
    let level = texture.main_level();
    match level.write_compressed_box_from((0, 0, 0), (4, 4, 1), &blocks, (8, 4, 1), (4, 0, 0)) {
        Ok(()) => (),
        Err(UploadError::NotSupported) => {
            assert!(
                !support::gl_at_least(&display, 4, 2)
                    && !support::extensions(&display).gl_arb_compressed_texture_pixel_storage
            );
            return;
        }
        Err(error) => panic!("{error:?}"),
    }
    // compressed textures are read on desktop OpenGL only
    if !support::is_gles(&display) {
        let read: Vec<Vec<Rgba>> = texture.read().unwrap();
        assert_eq!(read[0][0], (0, 255, 0, 255));
    }
    // boxes that don't start on the blocks are refused
    assert!(matches!(
        level.write_compressed_box_from((0, 0, 0), (4, 4, 1), &blocks, (8, 4, 1), (2, 0, 0)),
        Err(UploadError::MisalignedBlocks)
    ));

    display.assert_no_error(None);
}

/// A box of a texture level is read into a box of a larger client image, whose other texels
/// stay as they are.
#[cfg(safegl_gl45)]
#[test]
fn read_a_box_into_a_larger_image() {
    use safegl::ReadError;

    let display = support::build_display();
    let source = image(2, 2, 1);
    let texture = Texture2d::empty(&display, 2, 2).unwrap();
    texture
        .main_level()
        .write_box_from((0, 0, 0), (2, 2, 1), &source, (2, 2, 1), (0, 0, 0))
        .unwrap();

    let mut dest = vec![(9u8, 9u8, 9u8, 9u8); 4 * 3];
    match texture
        .main_level()
        .read_box_into((0, 0, 0), (2, 2, 1), &mut dest, (4, 3, 1), (1, 1, 0))
    {
        Ok(()) => (),
        Err(ReadError::NotSupported) => return,
        Err(error) => panic!("{error:?}"),
    }
    for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(texel(&dest, 4, 3, (1 + x, 1 + y, 0)), texel(&source, 2, 2, (x, y, 0)));
    }
    assert_eq!(dest[0], (9, 9, 9, 9));
    assert_eq!(texel(&dest, 4, 3, (3, 2, 0)), (9, 9, 9, 9));

    display.assert_no_error(None);
}

/// Other code that changes the layout of client images doesn't break the uploads of SafeGL once
/// the state is restored.
#[test]
fn uploads_after_foreign_pixel_store_changes() {
    use safegl::glow::HasContext;

    let display = support::build_display();
    unsafe {
        display.exec_in_context(|| {
            let gl = display.glow_context();
            gl.pixel_store_i32(safegl::glow::UNPACK_ROW_LENGTH, 7);
            gl.pixel_store_i32(safegl::glow::UNPACK_SKIP_PIXELS, 3);
            gl.pixel_store_i32(safegl::glow::UNPACK_SKIP_ROWS, 2);
        });
    }
    display.restore_state();

    let source = image(2, 2, 1);
    let texture = Texture2d::new(
        &display,
        vec![vec![source[0], source[1]], vec![source[2], source[3]]],
    )
    .unwrap();
    let read = read(&texture);
    assert_eq!(read[1][1], source[3]);
    display.assert_no_error(None);
}
