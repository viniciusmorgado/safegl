//! What the implementation supports for each internal format.
#![cfg(safegl_gl43)]

#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::texture::{DepthFormat, TextureFormat, UncompressedFloatFormat};
use safegl::{FormatInfoNotSupportedError, FormatSupport, FormatTarget};

mod support;

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
