#![cfg(any(safegl_gl43, safegl_gles31))]

#[macro_use]
extern crate safegl;

use safegl::uniforms::UniformBuffer;
use safegl::{DrawError, Surface};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

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

/// The elements of an array of a block must be as far apart as in the Rust array: the `float`s
/// of a `std140` block are 16 bytes apart, the ones of a `std430` block 4 bytes apart.
#[test]
fn array_strides() {
    use safegl::uniforms::LayoutMismatchError;

    let display = support::build_display();
    let buffer = UniformBuffer::new(&display, [1.0f32, 2.0, 3.0, 4.0, 5.0]).unwrap();
    let declare = |layout: &str| {
        storage_program(
            &display,
            &format!("layout({layout}) buffer Values {{ float values[5]; }};"),
            "float sum = values[0] + values[4];",
        )
    };
    let Some(std140) = declare("std140") else {
        return;
    };
    match draw(&display, &std140, &uniform! { Values: &buffer }) {
        Err(DrawError::UniformBlockLayoutMismatch { err, .. }) => assert!(
            matches!(
                err,
                LayoutMismatchError::StrideMismatch {
                    expected: 16,
                    obtained: 4,
                }
            ),
            "{err:?}"
        ),
        result => panic!("{:?}", result.err()),
    }

    let std430 = declare("std430").unwrap();
    draw(&display, &std430, &uniform! { Values: &buffer }).unwrap();

    display.assert_no_error(None);
}
