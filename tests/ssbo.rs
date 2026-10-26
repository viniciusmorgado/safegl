#![cfg(feature = "gl46")]

#[macro_use]
extern crate safegl;

use safegl::uniforms::UniformBuffer;
use safegl::{DrawError, Surface};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Data {
    color: (f32, f32, f32),
}

implement_uniform_block!(Data, color);
implement_any_bit_pattern!(Data, color);

/// A program whose fragment shader declares `blocks` and runs `body`, or `None` without shader
/// storage blocks (OpenGL 4.3 or OpenGL ES 3.1).
fn storage_program(display: &Display, blocks: &str, body: &str) -> Option<safegl::Program> {
    let header = if support::is_gles(display) {
        if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::GlEs, 3, 1)) {
            return None;
        }
        "#version 310 es\nprecision highp float;\n"
    } else {
        if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 4, 3)) {
            return None;
        }
        "#version 430\n"
    };
    let vertex = format!(
        "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
    );
    let fragment = format!(
        "{header}layout(std140) buffer;\nout vec4 f_color;\n{blocks}\n\
         void main() {{ {body} f_color = vec4(0.0, 0.0, 0.0, 1.0); }}"
    );
    Some(safegl::Program::from_source(display, &vertex, &fragment, None).unwrap())
}

fn empty_buffer(display: &Display) -> UniformBuffer<Data> {
    UniformBuffer::new(
        display,
        Data {
            color: (0.0, 0.0, 0.0),
        },
    )
    .unwrap()
}

fn draw<U: safegl::uniforms::Uniforms>(
    display: &Display,
    program: &safegl::Program,
    uniforms: &U,
) -> Result<(), DrawError> {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, program, uniforms, &Default::default())
}

#[test]
fn basic() {
    let display = support::build_display();
    let program = match storage_program(
        &display,
        "buffer MyBlock { vec3 color; };",
        "color = vec3(1.0, 1.0, 0.5);",
    ) {
        Some(program) => program,
        None => return,
    };

    let buffer = empty_buffer(&display);
    draw(&display, &program, &uniform! { MyBlock: &buffer }).unwrap();
    assert_eq!(buffer.read().unwrap().color, (1.0, 1.0, 0.5));

    display.assert_no_error(None);
}

#[test]
fn custom_bind_point() {
    let display = support::build_display();
    let program = match storage_program(
        &display,
        "layout(binding = 2) buffer MyBlock { vec3 color; };",
        "color = vec3(1.0, 1.0, 0.5);",
    ) {
        Some(program) => program,
        None => return,
    };

    let buffer = empty_buffer(&display);
    draw(&display, &program, &uniform! { MyBlock: &buffer }).unwrap();
    assert_eq!(buffer.read().unwrap().color, (1.0, 1.0, 0.5));

    display.assert_no_error(None);
}

#[test]
fn shared_binding_point() {
    let display = support::build_display();
    let program = match storage_program(
        &display,
        "layout(binding = 1) buffer First { vec3 color; } first;\n\
         layout(binding = 1) buffer Second { vec3 color; } second;",
        "first.color = vec3(1.0, 0.0, 0.0); second.color = vec3(0.0, 1.0, 0.0);",
    ) {
        Some(program) => program,
        None => return,
    };

    let first = empty_buffer(&display);
    let second = empty_buffer(&display);
    let result = draw(
        &display,
        &program,
        &uniform! { First: &first, Second: &second },
    );

    if support::is_gles(&display) {
        // OpenGL ES can't move the blocks to other binding points
        assert!(matches!(result, Err(DrawError::SharedBindingPoint { .. })));
    } else {
        // desktop OpenGL gives each block its own binding point
        result.unwrap();
        assert_eq!(first.read().unwrap().color, (1.0, 0.0, 0.0));
        assert_eq!(second.read().unwrap().color, (0.0, 1.0, 0.0));
    }

    display.assert_no_error(None);
}
