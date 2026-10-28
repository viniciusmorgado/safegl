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

/// Returns true if the backend has texture barriers, which make feedback loops defined.
fn texture_barriers(display: &support::Backend) -> bool {
    let extensions = support::extensions(display);
    support::gl_at_least(display, 4, 5)
        || extensions.gl_arb_texture_barrier
        || extensions.gl_nv_texture_barrier
}

/// Sampling a level that the draw renders to would read what it writes: it is an error, which
/// WebGL raises and OpenGL leaves undefined, except with texture barriers. Limiting the sampled
/// levels to the others renders a level from another one.
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
    if !texture_barriers(&display) {
        assert!(matches!(draw(), Err(DrawError::FeedbackLoop)));
    }

    // only the level 0 is; a level attached outside of the range of a texture created without
    // texture storage makes the framebuffer incomplete
    let storage = support::gl_at_least(&display, 4, 2)
        || support::is_gles(&display)
        || support::extensions(&display).gl_arb_texture_storage;
    if !storage {
        display.assert_no_error(None);
        return;
    }
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
    if texture_barriers(&display) {
        result.unwrap();
    } else {
        assert!(matches!(result, Err(DrawError::FeedbackLoop)));
    }

    display.assert_no_error(None);
}
