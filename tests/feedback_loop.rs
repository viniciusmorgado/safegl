//! Draws that sample a level of a texture that they render to.

#[macro_use]
extern crate safegl;

use safegl::framebuffer::SimpleFrameBuffer;
use safegl::texture::{MipmapsOption, UncompressedFloatFormat};
use safegl::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
use safegl::{DrawError, Surface, Texture2d};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Sampling a level that the draw renders to would read what it writes: it is an error, which
/// WebGL raises and OpenGL leaves undefined. Limiting the sampled levels to the others renders
/// a level from another one.
#[test]
fn sampled_level_rendered_to() {
    let display = support::build_display();
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::EmptyMipmapsMax(1),
        4,
        4,
    )
    .unwrap();
    texture.as_surface().clear_color(0.0, 1.0, 0.0, 1.0);

    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = support::program_from_legacy(
        &display,
        "#version 110\nattribute vec2 position;\n\
         void main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nuniform sampler2D tex;\n\
         void main() { gl_FragColor = texture2D(tex, vec2(0.5)); }",
    )
    .unwrap();
    let sampler = texture
        .sampled()
        .minify_filter(MinifySamplerFilter::NearestMipmapNearest)
        .magnify_filter(MagnifySamplerFilter::Nearest);
    let level = texture.mipmap(1).unwrap();
    let mut target = SimpleFrameBuffer::new(&display, level).unwrap();
    target.clear_color(0.0, 0.0, 0.0, 0.0);
    let mut draw = || {
        target.draw(
            &vb,
            &ib,
            &program,
            &uniform! { tex: sampler },
            &Default::default(),
        )
    };

    // the levels 0 and 1 are sampled
    assert!(matches!(draw(), Err(DrawError::FeedbackLoop)));

    // only the level 0 is
    texture.set_mipmap_range(0, 0).unwrap();
    draw().unwrap();
    let texels: Vec<Vec<(u8, u8, u8, u8)>> = level
        .first_layer()
        .into_image(None)
        .unwrap()
        .raw_read(&safegl::Rect {
            left: 0,
            bottom: 0,
            width: 2,
            height: 2,
        })
        .unwrap();
    assert_eq!(texels[1][1], (0, 255, 0, 255));

    display.assert_no_error(None);
}

/// A texture that the draw samples can't be the depth attachment either.
#[test]
fn sampled_depth_attachment() {
    let display = support::build_display();
    // WebGL needs attachments of the same size
    let depth = support::build_constant_depth_texture(&display, 0.5);
    let color = Texture2d::empty(&display, 2, 2).unwrap();
    let mut target = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();

    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = support::program_from_legacy(
        &display,
        "#version 110\nattribute vec2 position;\n\
         void main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nuniform sampler2D tex;\n\
         void main() { gl_FragColor = texture2D(tex, vec2(0.5)); }",
    )
    .unwrap();
    let result = target.draw(
        &vb,
        &ib,
        &program,
        &uniform! { tex: &depth },
        &Default::default(),
    );
    assert!(matches!(result, Err(DrawError::FeedbackLoop)));

    display.assert_no_error(None);
}
