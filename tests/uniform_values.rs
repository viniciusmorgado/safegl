#[macro_use]
extern crate safegl;

use safegl::uniforms::UniformsStorage;
use safegl::Surface;

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

/// The `#version` line for desktop OpenGL, or for OpenGL ES with a default precision.
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

/// Draws a fullscreen rectangle with `uniforms` and returns the color of a pixel.
fn draw<U: safegl::uniforms::Uniforms>(
    display: &Display,
    program: &safegl::Program,
    uniforms: &U,
) -> (u8, u8, u8, u8) {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, program, uniforms, &Default::default())
        .unwrap();

    display.assert_no_error(None);
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read();
    data[0][0]
}

#[test]
fn uniform_array_elements() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "uniform vec4 colors[3];\nvoid main() { f_color = colors[0] + colors[1] + colors[2]; }",
    );

    // the name of the array refers to its first element, as in OpenGL
    let first = program.get_uniform("colors").unwrap();
    assert_eq!(
        first.location,
        program.get_uniform("colors[0]").unwrap().location
    );
    assert!(first.size.is_none());
    assert!(program.get_uniform("colors[2]").is_some());

    let uniforms = UniformsStorage::new("colors", [1.0, 0.0, 0.0, 0.0f32])
        .add("colors[1]", [0.0, 1.0, 0.0, 0.0f32])
        .add("colors[2]", [0.0, 0.0, 1.0, 1.0f32]);
    assert_eq!(draw(&display, &program, &uniforms), (255, 255, 255, 255));

    // each element has its own location
    let uniforms = UniformsStorage::new("colors[2]", [0.0, 0.0, 1.0, 1.0f32])
        .add("colors[0]", [0.0f32; 4])
        .add("colors[1]", [0.0f32; 4]);
    assert_eq!(draw(&display, &program, &uniforms), (0, 0, 255, 255));
}

#[test]
fn non_square_matrices() {
    let display = support::build_display();

    // 4 columns of 3 rows: the product with the first axis is the first column
    let program = build_program(
        &display,
        "uniform mat4x3 m;\nvoid main() { f_color = vec4(m * vec4(1.0, 0.0, 0.0, 0.0), 1.0); }",
    );
    let m: [[f32; 3]; 4] = [
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ];
    assert_eq!(
        draw(&display, &program, &uniform! { m: m }),
        (0, 255, 0, 255)
    );

    // 3 columns of 2 rows: the product with the last axis is the last column
    let program = build_program(
        &display,
        "uniform mat3x2 m;\nvoid main() { f_color = vec4(m * vec3(0.0, 0.0, 1.0), 1.0, 1.0); }",
    );
    let m: [[f32; 2]; 3] = [[0.0, 0.0], [0.0, 0.0], [1.0, 0.0]];
    assert_eq!(
        draw(&display, &program, &uniform! { m: m }),
        (255, 0, 255, 255)
    );

    // a square matrix doesn't match a non-square uniform
    let square = [[0.0f32; 3]; 3];
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let texture = support::build_renderable_texture(&display);
    let result = texture.as_surface().draw(
        &vb,
        &ib,
        &program,
        &uniform! { m: square },
        &Default::default(),
    );
    assert!(matches!(
        result,
        Err(safegl::DrawError::UniformTypeMismatch { .. })
    ));
}
