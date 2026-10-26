#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

fn header(display: &Display) -> &'static str {
    if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    }
}

fn build_program(display: &Display, fragment: &str) -> safegl::Program {
    let vertex = format!(
        "{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
        header(display)
    );
    let fragment = format!("{}out vec4 f_color;\n{}", header(display), fragment);
    safegl::Program::from_source(display, &vertex, &fragment, None).unwrap()
}

#[test]
fn alpha_blending_keeps_the_source_alpha() {
    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = build_program(&display, "void main() { f_color = vec4(1.0, 0.0, 0.0, 0.5); }");

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &safegl::DrawParameters {
                blend: safegl::Blend::alpha_blending(),
                ..Default::default()
            },
        )
        .unwrap();

    // half of the red, and an alpha of 0.5 rather than 0.5 × 0.5
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read();
    let (r, _, _, a) = data[0][0];
    assert!((127..=128).contains(&r), "{r}");
    assert!((127..=128).contains(&a), "{a}");
    display.assert_no_error(None);
}
