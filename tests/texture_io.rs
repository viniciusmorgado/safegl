#[macro_use]
extern crate safegl;

use glow::HasContext;
use safegl::framebuffer::{DepthStencilRenderBuffer, SimpleFrameBuffer};
#[cfg(safegl_desktop)]
use safegl::texture::Texture1d;
use safegl::texture::{
    CubeLayer, Cubemap, DepthFormat, DepthStencilFormat, DepthStencilTexture2d, DepthTexture2d,
    MipmapsOption, Texture2dArray, Texture3d, TextureAnyImage, UncompressedFloatFormat,
    UncompressedUintFormat, UnsignedTexture2d,
};
#[cfg(any(safegl_gl44, safegl_gles32))]
use safegl::texture::{InternalFormat, InternalFormatType, StencilFormat, StencilTexture2d};
use safegl::{GlObject, Rect, Surface, Texture2d};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Pixel = (u8, u8, u8, u8);

fn rect(left: u32, bottom: u32, width: u32, height: u32) -> Rect {
    Rect {
        left,
        bottom,
        width,
        height,
    }
}

/// Reads a whole image of a texture.
fn read_image(image: TextureAnyImage<'_>) -> Vec<Vec<Pixel>> {
    let rect = rect(0, 0, image.get_width(), image.get_height().unwrap_or(1));
    image.raw_read(&rect).unwrap()
}

#[cfg(safegl_desktop)]
#[test]
fn write_1d() {
    let display = support::build_display();
    // OpenGL ES has no 1D textures
    let texture = match Texture1d::empty(&display, 4) {
        Ok(texture) => texture,
        Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
        Err(e) => panic!("{:?}", e),
    };

    texture.write(1, vec![(1u8, 2u8, 3u8, 4u8), (5, 6, 7, 8)]).unwrap();

    let data = read_image(texture.main_level().into());
    assert_eq!(data[0][1], (1, 2, 3, 4));
    assert_eq!(data[0][2], (5, 6, 7, 8));
    display.assert_no_error(None);
}

#[test]
fn write_3d() {
    let display = support::build_display();
    let texture = Texture3d::empty(&display, 4, 4, 4).unwrap();
    // the content of an empty texture is undefined
    texture.write((0, 0, 0), vec![vec![vec![(0u8, 0u8, 0u8, 0u8); 4]; 4]; 4]).unwrap();

    // a 2×2×2 block whose texels hold their z coordinate
    let block: Vec<Vec<Vec<Pixel>>> = (0..2).map(|z| vec![vec![(z, 0, 0, 255); 2]; 2]).collect();
    texture.write((1, 1, 2), block).unwrap();

    for z in 2..4 {
        let slice = texture
            .main_level()
            .layer(z)
            .unwrap()
            .into_image(None)
            .unwrap();
        let data = read_image(slice);
        assert_eq!(data[1][1], ((z - 2) as u8, 0, 0, 255));
        assert_eq!(data[2][2], ((z - 2) as u8, 0, 0, 255));
        assert_eq!(data[0][0], (0, 0, 0, 0));
    }
    display.assert_no_error(None);
}

#[test]
fn write_array_layer() {
    let display = support::build_display();
    let texture = Texture2dArray::empty(&display, 2, 2, 3).unwrap();
    for layer in 0..3 {
        let layer = texture.main_level().layer(layer).unwrap();
        layer.write(rect(0, 0, 2, 2), vec![vec![(0u8, 0u8, 0u8, 0u8); 2]; 2]).unwrap();
    }

    let layer = texture.main_level().layer(2).unwrap();
    layer.write(rect(0, 0, 2, 2), vec![vec![(9u8, 8u8, 7u8, 6u8); 2]; 2]).unwrap();

    assert_eq!(read_image(layer.into())[1][1], (9, 8, 7, 6));
    let first = texture.main_level().layer(0).unwrap();
    assert_eq!(read_image(first.into())[1][1], (0, 0, 0, 0));
    display.assert_no_error(None);
}

#[test]
fn write_cubemap_face() {
    let display = support::build_display();
    let texture = Cubemap::empty(&display, 2).unwrap();
    for face in [CubeLayer::NegativeY, CubeLayer::PositiveY] {
        let face = texture.main_level().image(face);
        face.write(rect(0, 0, 2, 2), vec![vec![(0u8, 0u8, 0u8, 0u8); 2]; 2]).unwrap();
    }

    let face = texture.main_level().image(CubeLayer::NegativeY);
    face.write(rect(1, 0, 1, 2), vec![vec![(1u8, 2u8, 3u8, 4u8)]; 2]).unwrap();

    let data = read_image(face.into());
    assert_eq!(data[0][1], (1, 2, 3, 4));
    assert_eq!(data[0][0], (0, 0, 0, 0));
    let other = read_image(texture.main_level().image(CubeLayer::PositiveY).into());
    assert_eq!(other[0][1], (0, 0, 0, 0));
    display.assert_no_error(None);
}

#[test]
fn write_and_clear_unsigned_texture() {
    let display = support::build_display();
    let texture = UnsignedTexture2d::empty_with_format(
        &display,
        UncompressedUintFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();

    texture.write(rect(0, 0, 1, 1), vec![vec![(1u8, 2u8, 3u8, 4u8)]]).unwrap();
    let data: Vec<Vec<(u32, u32, u32, u32)>> = texture.read_as().unwrap();
    assert_eq!(data[0][0], (1, 2, 3, 4));

    // `glClearColor` doesn't apply to integer color buffers
    SimpleFrameBuffer::new(&display, &texture)
        .unwrap()
        .clear_color(5.0, 6.0, 7.0, 8.0);
    let data: Vec<Vec<(u32, u32, u32, u32)>> = texture.read_as().unwrap();
    assert_eq!(data[1][1], (5, 6, 7, 8));
    display.assert_no_error(None);
}

#[test]
fn write_and_read_depth() {
    let display = support::build_display();
    let texture = DepthTexture2d::empty_with_format(
        &display,
        DepthFormat::F32,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();

    texture.write(rect(0, 0, 2, 2), vec![vec![0.25f32, 0.75]; 2]).unwrap();

    // OpenGL ES can't read depth values without `GL_NV_read_depth`
    if !support::is_gles(&display) {
        let data: Vec<Vec<f32>> = texture.read().unwrap();
        assert_eq!(data[1], vec![0.25, 0.75]);
    }
    display.assert_no_error(None);
}

#[test]
fn read_depth_stencil() {
    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }

    let color = Texture2d::empty(&display, 2, 2).unwrap();
    let buffer = DepthStencilRenderBuffer::new(&display, DepthStencilFormat::I24I8, 2, 2).unwrap();
    let mut framebuffer =
        SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &buffer).unwrap();
    framebuffer.clear_depth_and_stencil(1.0, 3);

    // the depth in the upper 24 bits and the stencil in the lower 8
    let texture = DepthStencilTexture2d::empty(&display, 2, 2).unwrap();
    let mut target =
        SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &texture).unwrap();
    target.clear_depth_and_stencil(1.0, 3);
    let image: TextureAnyImage<'_> = texture.main_level().into();
    let data: Vec<Vec<u32>> = image.raw_read(&rect(0, 0, 2, 2)).unwrap();
    assert_eq!(data[1][1], (0xff_ffff << 8) | 3);
    display.assert_no_error(None);
}

#[cfg(any(safegl_gl44, safegl_gles32))]
#[test]
fn stencil_texture_format() {
    let display = support::build_display();
    let texture = match StencilTexture2d::empty_with_format(
        &display,
        StencilFormat::I8,
        MipmapsOption::NoMipmap,
        2,
        2,
    ) {
        Ok(texture) => texture,
        Err(safegl::texture::TextureCreationError::TypeNotSupported)
            | Err(safegl::texture::TextureCreationError::FormatNotSupported) => return,
        Err(e) => panic!("{:?}", e),
    };

    match texture.get_internal_format() {
        Ok(InternalFormat::OneComponent { ty1, bits1 }) => {
            assert_eq!(ty1, InternalFormatType::UnsignedInt);
            assert_eq!(bits1, 8);
        }
        Ok(format) => panic!("{format:?}"),
        // `glGetTexLevelParameter` is missing on OpenGL ES 3.0
        Err(_) => assert!(support::is_gles(&display)),
    }
    display.assert_no_error(None);
}

#[test]
fn rows_without_padding() {
    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }

    // rows of three texels of three bytes are 9 bytes long, which `GL_PACK_ALIGNMENT` 4 would
    // pad to 12 bytes
    let data: Vec<Vec<Pixel>> = (0..3u8)
        .map(|y| (0..3u8).map(|x| (x, y, 0, 255)).collect())
        .collect();
    let texture = Texture2d::with_format(
        &display,
        data,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();

    let image: TextureAnyImage<'_> = texture.main_level().into();
    let read: Vec<Vec<(u8, u8, u8)>> = image.raw_read(&rect(0, 0, 3, 3)).unwrap();
    assert_eq!(read[1][0], (0, 1, 0));
    assert_eq!(read[2][2], (2, 2, 0));
    display.assert_no_error(None);
}

#[test]
fn texture_from_id_with_mipmaps() {
    let display = support::build_display();

    // a red texture created outside SafeGL, with storage for its mipmaps
    let id = unsafe {
        display.exec_in_context(|| {
            let gl = display.glow_context();
            let id = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(id));
            gl.tex_storage_2d(glow::TEXTURE_2D, 3, glow::RGBA8, 4, 4);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                4,
                4,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&[255, 0, 0, 255].repeat(16))),
            );
            id
        })
    };
    display.restore_state();

    // another texture is bound to the active unit when the mipmaps are generated
    let other = Texture2d::empty(&display, 4, 4).unwrap();
    let _ = read_image(other.main_level().into());

    let texture = unsafe {
        Texture2d::from_id(
            &display,
            UncompressedFloatFormat::U8U8U8U8,
            id,
            true,
            MipmapsOption::AutoGeneratedMipmaps,
            safegl::texture::Dimensions::Texture2d {
                width: 4,
                height: 4,
            },
        )
    };
    assert_eq!(texture.get_id(), id);

    let level = texture.mipmap(1).unwrap();
    assert_eq!(read_image(level.into())[0][0], (255, 0, 0, 255));
    display.assert_no_error(None);
}

/// Depth-stencil textures are written with `u32`s that pack a 24-bit depth above an 8-bit
/// stencil index; other client formats are refused instead of panicking.
#[test]
fn write_depth_stencil() {
    let display = support::build_display();
    let texture = DepthStencilTexture2d::empty_with_format(
        &display,
        DepthStencilFormat::I24I8,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();
    let value = (0x80_0000u32 << 8) | 9;
    texture
        .main_level()
        .write(rect(0, 0, 2, 2), vec![vec![value; 2]; 2])
        .unwrap();
    assert!(texture
        .main_level()
        .write(rect(0, 0, 2, 2), vec![vec![0.5f32; 2]; 2])
        .is_err());
    // depth textures are given floats
    let depth = safegl::texture::DepthTexture2d::empty(&display, 2, 2).unwrap();
    assert!(depth
        .main_level()
        .write(rect(0, 0, 2, 2), vec![vec![1u8; 2]; 2])
        .is_err());

    // the stencil index written is the one sampled, where it can be
    #[cfg(any(safegl_gl43, safegl_gles31, safegl_webgl))]
    {
        use safegl::texture::DepthStencilTextureMode;
        if texture
            .set_depth_stencil_mode(DepthStencilTextureMode::Stencil)
            .is_ok()
        {
            let (vb, ib) = support::build_rectangle_vb_ib(&display);
            let header = if support::is_gles(&display) {
                "#version 310 es\nprecision highp float;\nprecision highp usampler2D;\n"
            } else {
                "#version 330 core\n"
            };
            let program = safegl::Program::from_source(
                &display,
                &format!("{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"),
                &format!(
                    "{header}uniform usampler2D tex;\nout vec4 color;\n\
                     void main() {{ color = vec4(float(texelFetch(tex, ivec2(0), 0).r) / 255.0); }}"
                ),
                None,
            )
            .unwrap();
            let target = Texture2d::empty(&display, 2, 2).unwrap();
            target
                .as_surface()
                .draw(
                    &vb,
                    &ib,
                    &program,
                    &safegl::uniform! { tex: &texture },
                    &Default::default(),
                )
                .unwrap();
            let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
            assert_eq!(data[0][0].0, 9);
        }
    }

    display.assert_no_error(None);
}

/// Stencil textures are written with stencil indices.
#[cfg(any(safegl_gl44, safegl_gles32))]
#[test]
fn write_stencil() {
    let display = support::build_display();
    let texture = match StencilTexture2d::empty_with_format(
        &display,
        StencilFormat::I8,
        MipmapsOption::NoMipmap,
        2,
        2,
    ) {
        Ok(texture) => texture,
        Err(safegl::texture::TextureCreationError::TypeNotSupported)
        | Err(safegl::texture::TextureCreationError::FormatNotSupported) => return,
        Err(e) => panic!("{:?}", e),
    };
    texture
        .main_level()
        .write(rect(0, 0, 2, 2), vec![vec![5u8; 2]; 2])
        .unwrap();

    // read back through a stencil test: only the texels whose stencil is 5 are drawn
    let color = Texture2d::empty(&display, 2, 2).unwrap();
    let mut framebuffer =
        SimpleFrameBuffer::with_stencil_buffer(&display, &color, &texture).unwrap();
    framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let parameters = safegl::DrawParameters {
        stencil: safegl::draw_parameters::Stencil {
            test_clockwise: safegl::StencilTest::IfEqual { mask: 0xff },
            test_counter_clockwise: safegl::StencilTest::IfEqual { mask: 0xff },
            reference_value_clockwise: 5,
            reference_value_counter_clockwise: 5,
            ..Default::default()
        },
        ..Default::default()
    };
    framebuffer
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &parameters,
        )
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = color.read().unwrap();
    assert_eq!(data[1][1], (255, 0, 0, 255));

    display.assert_no_error(None);
}

/// Boxes of every kind of texture are read without framebuffers.
#[cfg(safegl_gl45)]
#[test]
fn read_boxes() {
    use safegl::ReadError;

    let display = support::build_display();
    // texel (x, y, z) is (x, y, z, 255)
    let data: Vec<Vec<Vec<(u8, u8, u8, u8)>>> = (0..4u8)
        .map(|z| (0..4u8).map(|y| (0..4u8).map(|x| (x, y, z, 255)).collect()).collect())
        .collect();
    let texture = Texture3d::with_format(
        &display,
        data.clone(),
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();
    let level = texture.main_level();
    match level.read_box::<(u8, u8, u8, u8)>((1, 2, 3), (2, 1, 1)) {
        Err(ReadError::NotSupported) => return,
        result => assert_eq!(result.unwrap(), vec![(1, 2, 3, 255), (2, 2, 3, 255)]),
    }
    let volume: Vec<Vec<Vec<(u8, u8, u8, u8)>>> = level.read_volume().unwrap();
    assert_eq!(volume, data);
    assert!(matches!(
        level.read_box::<(u8, u8, u8, u8)>((3, 0, 0), (2, 1, 1)),
        Err(ReadError::OutOfBounds)
    ));

    // depth and integer textures, which framebuffers read only partly on OpenGL ES
    let depth = DepthTexture2d::empty(&display, 2, 2).unwrap();
    depth
        .main_level()
        .write(rect(0, 0, 2, 2), vec![vec![0.25f32, 0.75]; 2])
        .unwrap();
    let depths = depth.main_level().read_box::<f32>((0, 0, 0), (2, 1, 1)).unwrap();
    // stored as 24-bit normalized integers
    assert!((depths[0] - 0.25).abs() < 1e-6 && (depths[1] - 0.75).abs() < 1e-6);
    let texels = vec![vec![(1u8, 2u8, 3u8, 4u8); 2]; 2];
    let unsigned = UnsignedTexture2d::new(&display, texels).unwrap();
    let texels = unsigned
        .main_level()
        .read_box::<(u8, u8, u8, u8)>((1, 1, 0), (1, 1, 1))
        .unwrap();
    assert_eq!(texels, vec![(1, 2, 3, 4)]);

    display.assert_no_error(None);
}

/// The blocks of compressed textures are read as they are stored.
#[cfg(safegl_gl45)]
#[test]
fn read_compressed_boxes() {
    use safegl::texture::{CompressedFormat, CompressedMipmapsOption, CompressedTexture2d};
    use safegl::ReadError;

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    // a 6 × 6 texture in RGTC1 has 2 × 2 blocks of 8 bytes, each filled with its index
    let blocks: Vec<u8> = (0..4u8).flat_map(|block| [block; 8]).collect();
    let texture = CompressedTexture2d::with_compressed_data(
        &display,
        &blocks,
        6,
        6,
        CompressedFormat::RgtcFormatU,
        CompressedMipmapsOption::NoMipmap,
    )
    .unwrap();
    let level = texture.main_level();
    match level.read_compressed_box((4, 0, 0), (2, 4, 1)) {
        Err(ReadError::NotSupported) => return,
        result => assert_eq!(result.unwrap(), vec![1; 8]),
    }
    assert_eq!(
        level.read_compressed_box((0, 4, 0), (6, 2, 1)).unwrap(),
        [[2; 8], [3; 8]].concat()
    );
    assert!(matches!(
        level.read_compressed_box((2, 0, 0), (4, 4, 1)),
        Err(ReadError::MisalignedBlocks)
    ));
    assert!(matches!(
        level.read_compressed_box((0, 0, 0), (2, 4, 1)),
        Err(ReadError::MisalignedBlocks)
    ));

    display.assert_no_error(None);
}

/// The regions written to compressed textures are made of whole blocks.
#[cfg(safegl_desktop)]
#[test]
fn write_compressed_blocks() {
    use safegl::texture::{CompressedFormat, CompressedMipmapsOption, CompressedTexture2d};
    use safegl::texture::UploadError;

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let texture = CompressedTexture2d::with_compressed_data(
        &display,
        &[0u8; 4 * 8],
        6,
        6,
        CompressedFormat::RgtcFormatU,
        CompressedMipmapsOption::NoMipmap,
    )
    .unwrap();
    let format = CompressedFormat::RgtcFormatU;
    // a whole block, and a partial one at the edge
    texture
        .write_compressed_data(rect(0, 0, 4, 4), &[1u8; 8], 4, 4, format)
        .unwrap();
    texture
        .write_compressed_data(rect(4, 4, 2, 2), &[3u8; 8], 2, 2, format)
        .unwrap();
    assert_eq!(
        texture.write_compressed_data(rect(2, 0, 4, 4), &[1u8; 8], 4, 4, format),
        Err(UploadError::MisalignedBlocks)
    );
    assert_eq!(
        texture.write_compressed_data(rect(0, 0, 2, 4), &[1u8; 8], 2, 4, format),
        Err(UploadError::MisalignedBlocks)
    );

    #[cfg(safegl_gl45)]
    if let Ok(block) = texture.main_level().read_compressed_box((4, 4, 0), (2, 2, 1)) {
        assert_eq!(block, vec![3; 8]);
    }

    display.assert_no_error(None);
}

/// The compressed data of a cube map holds its six faces, which OpenGL reads one by one.
#[test]
fn read_compressed_cubemap() {
    use safegl::texture::{CompressedCubemap, CompressedFormat, CompressedMipmapsOption};

    let display = support::build_display();
    // compressed data can only be read on desktop OpenGL
    if support::is_gles(&display) {
        return;
    }
    let cubemap = match CompressedCubemap::empty_with_format(
        &display,
        CompressedFormat::S3tcDxt1NoAlpha,
        CompressedMipmapsOption::NoMipmap,
        8,
    ) {
        Ok(cubemap) => cubemap,
        // without S3TC
        Err(safegl::texture::TextureCreationError::FormatNotSupported) => return,
        Err(e) => panic!("{e:?}"),
    };

    // 2x2 blocks of 8 bytes per face
    let (format, data) = cubemap.read_compressed_data().unwrap();
    assert_eq!(format, CompressedFormat::S3tcDxt1NoAlpha);
    assert_eq!(data.len(), 6 * 2 * 2 * 8);

    display.assert_no_error(None);
}
