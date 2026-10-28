//! sRGB textures and the `outputs_srgb` flag of programs (glium#1115).

#[macro_use]
extern crate safegl;

use safegl::framebuffer::SimpleFrameBuffer;
use safegl::program::ProgramCreationInput;
use safegl::texture::SrgbTexture2d;
use safegl::{CapabilitiesSource, Surface, Texture2d};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

/// 0.5 in linear RGB is 188 in sRGB.
const SRGB_HALF: u8 = 188;

/// Whether the context can write to sRGB targets without encoding to sRGB.
fn can_disable_encoding(display: &Display) -> bool {
    !support::is_gles(display) || display.get_extensions().gl_ext_srgb_write_control
}

fn program(display: &Display, fragment_body: &str, outputs_srgb: bool) -> safegl::Program {
    let header = if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    };
    let vertex = format!(
        "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
    );
    let fragment = format!(
        "{header}uniform sampler2D tex;\nout vec4 color;\nvoid main() {{ {fragment_body} }}"
    );
    safegl::Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: &vertex,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: &fragment,
            transform_feedback_varyings: None,
            outputs_srgb,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap()
}

fn draw<S: Surface, U: safegl::uniforms::Uniforms>(
    display: &Display,
    mut surface: S,
    program: &safegl::Program,
    uniforms: &U,
) {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    surface
        .draw(&vb, &ib, program, uniforms, &Default::default())
        .unwrap();
}

fn first_pixel(pixels: Vec<Vec<(u8, u8, u8, u8)>>) -> (u8, u8, u8, u8) {
    pixels[0][0]
}

/// Asserts that `actual` is `expected` with a rounding error of 1.
fn assert_near(actual: (u8, u8, u8, u8), expected: (u8, u8, u8, u8)) {
    let near = |a: u8, b: u8| a.abs_diff(b) <= 1;
    assert!(
        near(actual.0, expected.0)
            && near(actual.1, expected.1)
            && near(actual.2, expected.2)
            && near(actual.3, expected.3),
        "{actual:?} instead of {expected:?}"
    );
}

#[test]
fn round_trip() {
    let display = support::build_display();
    let texel = (SRGB_HALF, 10, 250, 255);
    let texture = SrgbTexture2d::new(&display, vec![vec![texel; 2]; 2]).unwrap();

    // the texels are stored and read as they are
    assert_eq!(first_pixel(texture.read().unwrap()), texel);

    display.assert_no_error(None);
}

#[test]
fn sampling_decodes() {
    let display = support::build_display();
    let program = program(&display, "color = texture(tex, vec2(0.5));", true);
    let texel = (SRGB_HALF, SRGB_HALF, SRGB_HALF, 255);
    let srgb = SrgbTexture2d::new(&display, vec![vec![texel]]).unwrap();
    let linear = Texture2d::new(&display, vec![vec![texel]]).unwrap();
    let target = Texture2d::empty(&display, 1, 1).unwrap();

    // a sRGB texture is converted to linear RGB when sampled
    draw(
        &display,
        SimpleFrameBuffer::new(&display, &target).unwrap(),
        &program,
        &uniform! { tex: &srgb },
    );
    assert_near(first_pixel(target.read().unwrap()), (128, 128, 128, 255));

    // a linear one isn't
    draw(
        &display,
        SimpleFrameBuffer::new(&display, &target).unwrap(),
        &program,
        &uniform! { tex: &linear },
    );
    assert_eq!(first_pixel(target.read().unwrap()), texel);

    display.assert_no_error(None);
}

#[test]
fn drawing_encodes_unless_the_program_outputs_srgb() {
    let display = support::build_display();
    let texture = Texture2d::empty(&display, 1, 1).unwrap();
    let target = SrgbTexture2d::empty(&display, 1, 1).unwrap();
    let body = "color = vec4(0.5, 0.5, 0.5, 1.0);";

    // linear output is encoded to sRGB
    let linear_output = program(&display, body, false);
    draw(
        &display,
        SimpleFrameBuffer::new(&display, &target).unwrap(),
        &linear_output,
        &uniform! { tex: &texture },
    );
    assert_near(
        first_pixel(target.read().unwrap()),
        (SRGB_HALF, SRGB_HALF, SRGB_HALF, 255),
    );

    // sRGB output is written as it is
    if can_disable_encoding(&display) {
        let srgb_output = program(&display, body, true);
        draw(
            &display,
            SimpleFrameBuffer::new(&display, &target).unwrap(),
            &srgb_output,
            &uniform! { tex: &texture },
        );
        assert_near(first_pixel(target.read().unwrap()), (128, 128, 128, 255));
    }

    // a linear target is never encoded
    let linear_target = Texture2d::empty(&display, 1, 1).unwrap();
    draw(
        &display,
        linear_target.as_surface(),
        &linear_output,
        &uniform! { tex: &texture },
    );
    assert_near(
        first_pixel(linear_target.read().unwrap()),
        (128, 128, 128, 255),
    );

    display.assert_no_error(None);
}

#[test]
fn clearing() {
    let display = support::build_display();
    let target = SrgbTexture2d::empty(&display, 1, 1).unwrap();

    // `clear_color` takes linear values
    SimpleFrameBuffer::new(&display, &target)
        .unwrap()
        .clear_color(0.5, 0.5, 0.5, 1.0);
    assert_near(
        first_pixel(target.read().unwrap()),
        (SRGB_HALF, SRGB_HALF, SRGB_HALF, 255),
    );

    // and `clear_color_srgb` sRGB values
    if can_disable_encoding(&display) {
        SimpleFrameBuffer::new(&display, &target)
            .unwrap()
            .clear_color_srgb(0.5, 0.5, 0.5, 1.0);
        assert_near(first_pixel(target.read().unwrap()), (128, 128, 128, 255));
    }

    display.assert_no_error(None);
}
