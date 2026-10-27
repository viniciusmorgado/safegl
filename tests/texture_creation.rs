#[macro_use]
extern crate safegl;

#[cfg(safegl_desktop)]
use safegl::Surface;

mod support;

#[cfg(safegl_desktop)]
#[test]
fn empty_texture1d_u8u8u8u8() {
    let display = support::build_display();

    let texture = safegl::texture::Texture1d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
    );

    display.assert_no_error(None);
    drop(texture);
    display.assert_no_error(None);
}

#[test]
fn get_format_u8u8u8u8() {
    let display = support::build_display();

    let texture = safegl::texture::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
        128,
    )
    .unwrap();

    display.assert_no_error(None);

    let format = match texture.get_internal_format() {
        Err(safegl::texture::GetFormatError::NotSupported) => return,
        Ok(f) => f,
    };

    match format {
        safegl::texture::InternalFormat::FourComponents {
            ty1,
            bits1,
            ty2,
            bits2,
            ty3,
            bits3,
            ty4,
            bits4,
        } => {
            assert_eq!(ty1, safegl::texture::InternalFormatType::UnsignedNormalized);
            assert_eq!(ty2, safegl::texture::InternalFormatType::UnsignedNormalized);
            assert_eq!(ty3, safegl::texture::InternalFormatType::UnsignedNormalized);
            assert_eq!(ty4, safegl::texture::InternalFormatType::UnsignedNormalized);

            assert!(bits1 >= 8);
            assert!(bits2 >= 8);
            assert!(bits3 >= 8);
            assert!(bits4 >= 8);
        }
        _ => panic!(),
    }
}

#[cfg(safegl_desktop)]
#[test]
fn depth_texture_1d_creation() {
    let display = support::build_display();

    let texture = match safegl::texture::DepthTexture1d::new(&display, vec![0.0, 0.0, 0.0, 0.0f32]) {
        Err(safegl::texture::TextureCreationError::FormatNotSupported)
        | Err(safegl::texture::TextureCreationError::DimensionsNotSupported)
        | Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(t) => t,
    };

    assert_eq!(texture.get_width(), 4);
    assert_eq!(texture.get_height(), None);
    assert_eq!(texture.get_depth(), None);
    assert_eq!(texture.get_array_size(), None);

    display.assert_no_error(None);
}

#[test]
fn texture_2d_creation() {
    let display = support::build_display();

    let texture = safegl::texture::Texture2d::new(
        &display,
        vec![
            vec![(0, 0, 0, 0), (0, 0, 0, 0)],
            vec![(0, 0, 0, 0), (0, 0, 0, 0)],
            vec![(0, 0, 0, 0), (0, 0, 0, 0u8)],
        ],
    )
    .unwrap();

    assert_eq!(texture.get_width(), 2);
    assert_eq!(texture.get_height(), Some(3));
    assert_eq!(texture.get_depth(), None);
    assert_eq!(texture.get_array_size(), None);

    display.assert_no_error(None);
}

#[test]
fn texture_2d_as_uniform_value_lifetime() {
    let display = support::build_display();

    let texture = safegl::texture::Texture2d::new(
        &display,
        vec![
            vec![(0, 0, 0, 0), (0, 0, 0, 0)],
            vec![(0, 0, 0, 0), (0, 0, 0, 0)],
            vec![(0, 0, 0, 0), (0, 0, 0, 0u8)],
        ],
    )
    .unwrap();

    // A function that takes texture reference should be able to return UniformValue (with lifetime
    // inherited from the reference).
    fn get_uniforms(texture: &safegl::texture::Texture2d) -> safegl::uniforms::UniformValue<'_> {
        use safegl::uniforms::AsUniformValue;
        texture.as_uniform_value()
    }

    let uniforms = get_uniforms(&texture);
    assert!(matches!(
        uniforms,
        safegl::uniforms::UniformValue::Texture(..)
    ));

    display.assert_no_error(None);
}

#[test]
fn empty_texture2d_u8u8u8u8() {
    let display = support::build_display();

    let texture = safegl::texture::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
        128,
    );

    display.assert_no_error(None);
    drop(texture);
    display.assert_no_error(None);
}

#[test]
fn depth_texture_2d_creation() {
    let display = support::build_display();

    let texture = safegl::texture::DepthTexture2d::new(
        &display,
        vec![
            vec![0.0, 0.0, 0.0, 0.0f32],
            vec![0.0, 0.0, 0.0, 0.0f32],
            vec![0.0, 0.0, 0.0, 0.0f32],
        ],
    );

    let texture = match texture {
        Err(safegl::texture::TextureCreationError::FormatNotSupported)
        | Err(safegl::texture::TextureCreationError::DimensionsNotSupported)
        | Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(t) => t,
    };

    assert_eq!(texture.get_width(), 4);
    assert_eq!(texture.get_height(), Some(3));
    assert_eq!(texture.get_depth(), None);
    assert_eq!(texture.get_array_size(), None);

    display.assert_no_error(None);
}

#[test]
fn empty_depth_texture2d_f32() {
    let display = support::build_display();

    let texture = safegl::texture::DepthTexture2d::empty(&display, 128, 128).unwrap();
    assert_eq!(texture.get_width(), 128);
    assert_eq!(texture.get_height(), Some(128));

    display.assert_no_error(None);
    drop(texture);
    display.assert_no_error(None);
}

#[test]
fn compressed_texture_2d_creation() {
    let display = support::build_display();

    let texture = safegl::texture::CompressedTexture2d::new(
        &display,
        vec![
            vec![(0, 0, 0, 0), (0, 0, 0, 0)],
            vec![(0, 0, 0, 0), (0, 0, 0, 0)],
            vec![(0, 0, 0, 0), (0, 0, 0, 0u8)],
        ],
    )
    .unwrap();

    assert_eq!(texture.get_width(), 2);
    assert_eq!(texture.get_height(), Some(3));
    assert_eq!(texture.get_depth(), None);
    assert_eq!(texture.get_array_size(), None);

    display.assert_no_error(None);
}

macro_rules! empty_texture_test {
    ($test_name:ident, $tex_ty:ident, [$($dims:expr),+],
     $w:expr, $h:expr, $d:expr, $s:expr) =>
    (
        #[test]
        fn $test_name() {
            let display = support::build_display();

            let texture = match safegl::texture::$tex_ty::empty(&display, $($dims),+) {
                Err(safegl::texture::TextureCreationError::FormatNotSupported)
                | Err(safegl::texture::TextureCreationError::DimensionsNotSupported)
                | Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
                Err(e) => panic!("{:?}", e),
                Ok(tex) => tex
            };

            assert_eq!(texture.get_width(), $w);
            assert_eq!(texture.get_height(), $h);
            assert_eq!(texture.get_depth(), $d);
            assert_eq!(texture.get_array_size(), $s);

            assert_eq!(texture.get_mipmap_levels(), 1);

            let _ = texture.get_internal_format();

            display.assert_no_error(None);
            drop(texture);
            display.assert_no_error(None);
        }
    );
}

// TODO: compressed textures don't have "empty" yet
/*empty_texture_test!(empty_compressedtexture1d, CompressedTexture1d, [64], 64, None, None, None);
empty_texture_test!(empty_compressedtexture1darray, CompressedTexture1dArray, [64, 32], 64, None, None, Some(32));
empty_texture_test!(empty_compressedtexture2d, CompressedTexture2d, [64, 32], 64, Some(32), None, None);
empty_texture_test!(empty_compressedtexture2darray, CompressedTexture2dArray, [64, 32, 16], 64, Some(32), None, Some(16));
empty_texture_test!(empty_compressedtexture3d, CompressedTexture3d, [64, 32, 16], 64, Some(32), Some(16), None);*/
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_depthstenciltexture1d,
    DepthStencilTexture1d,
    [64],
    64,
    None,
    None,
    None
);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_depthstenciltexture1darray,
    DepthStencilTexture1dArray,
    [64, 32],
    64,
    None,
    None,
    Some(32)
);
empty_texture_test!(
    empty_depthstenciltexture2d,
    DepthStencilTexture2d,
    [64, 32],
    64,
    Some(32),
    None,
    None
);
empty_texture_test!(
    empty_depthstenciltexture2darray,
    DepthStencilTexture2dArray,
    [64, 32, 16],
    64,
    Some(32),
    None,
    Some(16)
);
// TODO: non-working
//empty_texture_test!(empty_depthstenciltexture3d, DepthStencilTexture3d, [64, 32, 16], 64, Some(32), Some(16), None);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_depthtexture1d,
    DepthTexture1d,
    [64],
    64,
    None,
    None,
    None
);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_depthtexture1darray,
    DepthTexture1dArray,
    [64, 32],
    64,
    None,
    None,
    Some(32)
);
empty_texture_test!(
    empty_depthtexture2d,
    DepthTexture2d,
    [64, 32],
    64,
    Some(32),
    None,
    None
);
empty_texture_test!(
    empty_depthtexture2darray,
    DepthTexture2dArray,
    [64, 32, 16],
    64,
    Some(32),
    None,
    Some(16)
);
// TODO: non-working
//empty_texture_test!(empty_depthtexture3d, DepthTexture3d, [64, 32, 16], 64, Some(32), Some(16), None);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_integraltexture1d,
    IntegralTexture1d,
    [64],
    64,
    None,
    None,
    None
);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_integraltexture1darray,
    IntegralTexture1dArray,
    [64, 32],
    64,
    None,
    None,
    Some(32)
);
empty_texture_test!(
    empty_integraltexture2d,
    IntegralTexture2d,
    [64, 32],
    64,
    Some(32),
    None,
    None
);
empty_texture_test!(
    empty_integraltexture2darray,
    IntegralTexture2dArray,
    [64, 32, 16],
    64,
    Some(32),
    None,
    Some(16)
);
empty_texture_test!(
    empty_integraltexture3d,
    IntegralTexture3d,
    [64, 32, 16],
    64,
    Some(32),
    Some(16),
    None
);
#[cfg(safegl_desktop)]
empty_texture_test!(empty_srgbtexture1d, Texture1d, [64], 64, None, None, None);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_srgbtexture1darray,
    Texture1dArray,
    [64, 32],
    64,
    None,
    None,
    Some(32)
);
empty_texture_test!(
    empty_srgbtexture2d,
    Texture2d,
    [64, 32],
    64,
    Some(32),
    None,
    None
);
empty_texture_test!(
    empty_srgbtexture2darray,
    Texture2dArray,
    [64, 32, 16],
    64,
    Some(32),
    None,
    Some(16)
);
empty_texture_test!(
    empty_srgbtexture3d,
    Texture3d,
    [64, 32, 16],
    64,
    Some(32),
    Some(16),
    None
);
#[cfg(all(safegl_desktop, any(safegl_gl44, safegl_gles32)))]
empty_texture_test!(
    empty_stenciltexture1d,
    StencilTexture1d,
    [64],
    64,
    None,
    None,
    None
);
#[cfg(all(safegl_desktop, any(safegl_gl44, safegl_gles32)))]
empty_texture_test!(
    empty_stenciltexture1darray,
    StencilTexture1dArray,
    [64, 32],
    64,
    None,
    None,
    Some(32)
);
#[cfg(any(safegl_gl44, safegl_gles32))]
empty_texture_test!(
    empty_stenciltexture2d,
    StencilTexture2d,
    [64, 32],
    64,
    Some(32),
    None,
    None
);
#[cfg(any(safegl_gl44, safegl_gles32))]
empty_texture_test!(
    empty_stenciltexture2darray,
    StencilTexture2dArray,
    [64, 32, 16],
    64,
    Some(32),
    None,
    Some(16)
);
//empty_texture_test!(empty_stenciltexture3d, StencilTexture3d, [64, 32, 16], 64, Some(32), Some(16), None);
#[cfg(safegl_desktop)]
empty_texture_test!(empty_texture1d, Texture1d, [64], 64, None, None, None);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_texture1darray,
    Texture1dArray,
    [64, 32],
    64,
    None,
    None,
    Some(32)
);
empty_texture_test!(
    empty_texture2d,
    Texture2d,
    [64, 32],
    64,
    Some(32),
    None,
    None
);
empty_texture_test!(
    empty_texture2darray,
    Texture2dArray,
    [64, 32, 16],
    64,
    Some(32),
    None,
    Some(16)
);
empty_texture_test!(
    empty_texture3d,
    Texture3d,
    [64, 32, 16],
    64,
    Some(32),
    Some(16),
    None
);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_unsignedtexture1d,
    UnsignedTexture1d,
    [64],
    64,
    None,
    None,
    None
);
#[cfg(safegl_desktop)]
empty_texture_test!(
    empty_unsignedtexture1darray,
    UnsignedTexture1dArray,
    [64, 32],
    64,
    None,
    None,
    Some(32)
);
empty_texture_test!(
    empty_unsignedtexture2d,
    UnsignedTexture2d,
    [64, 32],
    64,
    Some(32),
    None,
    None
);
empty_texture_test!(
    empty_unsignedtexture2darray,
    UnsignedTexture2dArray,
    [64, 32, 16],
    64,
    Some(32),
    None,
    Some(16)
);
empty_texture_test!(
    empty_unsignedtexture3d,
    UnsignedTexture3d,
    [64, 32, 16],
    64,
    Some(32),
    Some(16),
    None
);

#[cfg(safegl_desktop)]
#[test]
fn zero_sized_texture_1d_creation() {
    let display = support::build_display();

    let texture = match safegl::texture::Texture1d::new(&display, Vec::<(u8, u8, u8, u8)>::new()) {
        Err(safegl::texture::TextureCreationError::FormatNotSupported)
        | Err(safegl::texture::TextureCreationError::DimensionsNotSupported)
        | Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(t) => t,
    };

    assert_eq!(texture.get_width(), 0);
    assert_eq!(texture.get_height(), None);
    assert_eq!(texture.get_depth(), None);
    assert_eq!(texture.get_array_size(), None);

    display.assert_no_error(None);
}

#[test]
fn zero_sized_texture_2d_creation() {
    let display = support::build_display();

    let texture =
        safegl::texture::Texture2d::new(&display, Vec::<Vec<(u8, u8, u8, u8)>>::new()).unwrap();

    assert_eq!(texture.get_width(), 0);
    assert_eq!(texture.get_height(), Some(0));
    assert_eq!(texture.get_depth(), None);
    assert_eq!(texture.get_array_size(), None);

    display.assert_no_error(None);
}

#[test]
fn zero_sized_texture_3d_creation() {
    let display = support::build_display();

    let texture =
        match safegl::texture::Texture3d::new(&display, Vec::<Vec<Vec<(u8, u8, u8, u8)>>>::new()) {
            Err(safegl::texture::TextureCreationError::FormatNotSupported)
            | Err(safegl::texture::TextureCreationError::DimensionsNotSupported)
            | Err(safegl::texture::TextureCreationError::TypeNotSupported) => return,
            Err(e) => panic!("{:?}", e),
            Ok(t) => t,
        };

    assert_eq!(texture.get_width(), 0);
    assert_eq!(texture.get_height(), Some(0));
    assert_eq!(texture.get_depth(), Some(0));
    assert_eq!(texture.get_array_size(), None);

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn bindless_texture_residency_context_rebuild() {
    let mut display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    let texture = safegl::texture::Texture2d::new(
        &display,
        vec![
            vec![(255, 0, 0, 255), (255, 0, 0, 255)],
            vec![(255, 0, 0, 255), (255, 0, 0, 255u8)],
        ],
    )
    .unwrap();

    let texture = match texture.resident() {
        Ok(t) => t,
        Err(safegl::texture::BindlessTexturesNotSupportedError) => return,
    };

    // here is the trick: we rebuild the display, meaning that texture residency has to be updated
    // by SafeGL
    support::rebuild_display(&mut display);

    // if bindless textures are supported, we can call .unwrap() and expect that everything
    // else is supported here as well

    let program = safegl::Program::from_source(
        &display,
        "
            #version 400

            in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
        "
            #version 400
            #extension GL_ARB_bindless_texture : require

            uniform Samplers {
                sampler2D tex;
            };

            out vec4 f_color;

            void main() {
                f_color = texture(tex, vec2(0.0, 0.0));
            }
        ",
        None,
    )
    .unwrap();

    let buffer = safegl::uniforms::UniformBuffer::new(
        &display,
        safegl::texture::TextureHandle::new(&texture, &Default::default()),
    )
    .unwrap();

    let output = support::build_renderable_texture(&display);
    output.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    output
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { Samplers: &buffer },
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn upload_from_pixel_buffer() {
    let display = support::build_display();

    let texture = safegl::texture::Texture2d::empty(&display, 2, 2).unwrap();

    let buffer = safegl::texture::pixel_buffer::PixelBuffer::new_empty(&display, 4);
    buffer.write(&[
        (0u8, 255u8, 0u8, 255u8),
        (255, 0, 255, 0),
        (255, 255, 0, 255),
        (0, 0, 255, 255),
    ]);

    texture
        .main_level()
        .raw_upload_from_pixel_buffer(buffer.as_slice(), 0..2, 0..2, 0..1).unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));
    assert_eq!(data[0][1], (255, 0, 255, 0));
    assert_eq!(data[1][0], (255, 255, 0, 255));
    assert_eq!(data[1][1], (0, 0, 255, 255));

    display.assert_no_error(None);
}

#[test]
fn upload_from_pixel_buffer_inverted() {
    let display = support::build_display();
    // OpenGL ES can't flip the components while uploading
    if support::is_gles(&display) {
        return;
    }

    let texture = safegl::texture::Texture2d::empty(&display, 2, 2).unwrap();

    let buffer = safegl::texture::pixel_buffer::PixelBuffer::new_empty(&display, 4);
    buffer.write(&[
        (0u8, 255u8, 0u8, 255u8),
        (255, 0, 255, 0),
        (255, 255, 0, 255),
        (0, 0, 255, 255),
    ]);

    texture
        .main_level()
        .raw_upload_from_pixel_buffer_inverted(buffer.as_slice(), 0..2, 0..2, 0..1).unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));
    assert_eq!(data[0][1], (255, 0, 255, 0));
    assert_eq!(data[1][0], (0, 255, 255, 255));
    assert_eq!(data[1][1], (255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn compressed_texture_with_partial_blocks() {
    use safegl::texture::{CompressedFormat, CompressedMipmapsOption, CompressedTexture2d};

    let display = support::build_display();

    // a 6x6 image takes 2x2 blocks of 8 bytes in DXT1, the blocks at the edges being partial
    let data = vec![0u8; 2 * 2 * 8];
    let texture = CompressedTexture2d::with_compressed_data(
        &display,
        &data,
        6,
        6,
        CompressedFormat::S3tcDxt1NoAlpha,
        CompressedMipmapsOption::NoMipmap,
    );
    match texture {
        Ok(texture) => assert_eq!(texture.get_width(), 6),
        // without S3TC
        Err(safegl::texture::TextureCreationError::FormatNotSupported) => (),
        Err(e) => panic!("{:?}", e),
    }

    // data whose size doesn't fit the blocks is an error, not a panic
    let result = CompressedTexture2d::with_compressed_data(
        &display,
        &data[..10],
        6,
        6,
        CompressedFormat::S3tcDxt1NoAlpha,
        CompressedMipmapsOption::NoMipmap,
    );
    assert!(result.is_err());

    display.assert_no_error(None);
}

#[test]
fn data_of_the_wrong_size() {
    let display = support::build_display();

    // RGB bytes given as RGBA
    let image = safegl::texture::RawImage2d::from_raw_rgba(vec![0u8; 3 * 4 * 4], (4, 4));
    assert!(matches!(
        safegl::Texture2d::new(&display, image),
        Err(safegl::texture::TextureCreationError::DataSizeMismatch)
    ));

    let texture = safegl::Texture2d::empty(&display, 4, 4).unwrap();
    let image = safegl::texture::RawImage2d::from_raw_rgba(vec![0u8; 3 * 4 * 4], (4, 4));
    let result = texture.write(
        safegl::Rect {
            left: 0,
            bottom: 0,
            width: 4,
            height: 4,
        },
        image,
    );
    assert!(matches!(
        result,
        Err(safegl::texture::UploadError::DataSizeMismatch { expected: 64, obtained: 48 })
    ));

    display.assert_no_error(None);
}
