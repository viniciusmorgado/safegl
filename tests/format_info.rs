//! What the implementation supports for each internal format.
#![cfg(safegl_gl43)]

#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::texture::{DepthFormat, TextureFormat, UncompressedFloatFormat};
use safegl::{FormatInfoNotSupportedError, FormatSupport, FormatTarget};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn format_info() {
    let display = support::build_display();
    let context = display.get_context();
    let rgba8 = TextureFormat::UncompressedFloat(UncompressedFloatFormat::U8U8U8U8);
    let info = match context.format_info(FormatTarget::Texture2d, rgba8) {
        Err(FormatInfoNotSupportedError) => {
            assert!(
                support::is_gles(&display)
                    || !(support::gl_at_least(&display, 4, 3)
                        || support::extensions(&display).gl_arb_internalformat_query2)
            );
            return;
        }
        Ok(info) => info,
    };

    // RGBA8 has everything that OpenGL 4.3 guarantees for it
    assert!(info.supported);
    assert!(info.max_size.0 >= 1024 && info.max_size.1 >= 1024);
    assert!(info.renderable.0 && !info.renderable.1);
    assert_eq!(info.framebuffer_renderable, FormatSupport::Full);
    assert_ne!(info.filter, FormatSupport::None);
    assert!(info.mipmaps);
    assert!(info.texture_views);
    assert!(info.samples.is_empty());
    // the answers are cached
    assert_eq!(
        context.format_info(FormatTarget::Texture2d, rgba8).unwrap(),
        info
    );

    // multisample render buffers list their numbers of samples, from the largest
    let renderbuffer = context
        .format_info(FormatTarget::RenderBuffer, rgba8)
        .unwrap();
    assert!(!renderbuffer.samples.is_empty());
    assert!(renderbuffer
        .samples
        .windows(2)
        .all(|pair| pair[0] > pair[1]));

    // depth formats are depth-renderable, and the shared-exponent format is a texture format
    // that only some implementations render to
    let depth = context
        .format_info(
            FormatTarget::Texture2d,
            TextureFormat::DepthFormat(DepthFormat::I24),
        )
        .unwrap();
    assert!(depth.supported && depth.renderable.1 && !depth.renderable.0);
    let shared_exponent = context
        .format_info(
            FormatTarget::Texture2d,
            TextureFormat::UncompressedFloat(UncompressedFloatFormat::F9F9F9),
        )
        .unwrap();
    assert!(shared_exponent.supported && !shared_exponent.renderable.1);

    display.assert_no_error(None);
}

/// The other answers of `GL_ARB_internalformat_query2` agree with each other and with what the
/// formats are.
#[test]
fn format_details() {
    use safegl::texture::{CompressedFormat, UncompressedIntFormat};
    use safegl::{ColorEncoding, ComponentType, ImageCompatibility, ImageFormatClass, ViewClass};

    let display = support::build_display();
    let context = display.get_context();
    let info = |format| context.format_info(FormatTarget::Texture2d, format);
    let rgba8 = match info(TextureFormat::UncompressedFloat(UncompressedFloatFormat::U8U8U8U8)) {
        Ok(info) => info,
        Err(FormatInfoNotSupportedError) => return,
    };

    // RGBA8: four normalized 8-bit color components, linear, readable, uploadable, 32-bit views
    let c = rgba8.components;
    for (size, ty) in [c.red, c.green, c.blue, c.alpha] {
        assert_eq!((size, ty), (8, ComponentType::UnsignedNormalized));
    }
    assert_eq!(c.depth, (0, ComponentType::None));
    assert_eq!(c.shared_exponent, 0);
    assert_eq!(rgba8.has_components, (true, false, false));
    assert!(rgba8.max_combined_dimensions >= 1024 * 1024);
    assert_eq!(rgba8.color_encoding, Some(ColorEncoding::Linear));
    assert_ne!(rgba8.sampling.fragment, FormatSupport::None);
    assert_eq!(rgba8.compressed, None);
    assert_ne!(rgba8.views.0, FormatSupport::None);
    assert_eq!(rgba8.views.1, Some(ViewClass::Bits32));
    // image units: 32 bits (4 bytes on NVIDIA), and a class of four components (NVIDIA reports
    // 32-bit ones)
    if rgba8.image.0 != FormatSupport::None {
        assert!(matches!(rgba8.image_texel_size, 32 | 4));
        assert!(matches!(
            rgba8.image_class,
            Some(ImageFormatClass::FourX8 | ImageFormatClass::FourX32)
        ));
        assert!(matches!(
            rgba8.image_compatibility,
            Some(ImageCompatibility::BySize | ImageCompatibility::ByClass)
        ));
    }

    // sRGB colors are encoded
    let srgb = info(TextureFormat::Srgb(safegl::texture::SrgbFormat::U8U8U8U8)).unwrap();
    assert_eq!(srgb.color_encoding, Some(ColorEncoding::Srgb));

    // integer components and the shared exponent
    let integer = info(TextureFormat::UncompressedIntegral(UncompressedIntFormat::I32)).unwrap();
    assert_eq!(integer.components.red, (32, ComponentType::Int));
    let shared =
        info(TextureFormat::UncompressedFloat(UncompressedFloatFormat::F9F9F9)).unwrap();
    assert_eq!(shared.components.shared_exponent, 5);

    // depth formats have a depth component and no color
    let depth = info(TextureFormat::DepthFormat(DepthFormat::I24)).unwrap();
    assert_eq!(depth.has_components, (false, true, false));
    assert_eq!(depth.components.depth.0, 24);
    assert_eq!(depth.color_encoding, None);

    // compressed formats give their blocks
    if let Ok(dxt1) = info(TextureFormat::CompressedFormat(CompressedFormat::S3tcDxt1NoAlpha)) {
        if dxt1.supported {
            assert_eq!(dxt1.compressed, Some((4, 4, 8)));
            assert_eq!(dxt1.views.1, Some(ViewClass::S3tcDxt1Rgb));
        }
    }

    display.assert_no_error(None);
}
