#![cfg(any(safegl_gl42, safegl_gles31))]

#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

#[test]
fn basic() {
    let display = support::build_display();

    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 4, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 330

            in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
        "
            #version 430

            out vec4 f_color;

            layout(binding = 1) uniform atomic_uint counter;

            void main() {
                f_color = vec4(0.0, 0.0, 0.0, 1.0);
                atomicCounterIncrement(counter);
            }
        ",
        None,
    );

    let program = program.unwrap();

    let buffer = match safegl::buffer::Buffer::new(
        &display,
        &10u32,
        safegl::buffer::BufferType::AtomicCounterBuffer,
        safegl::buffer::BufferMode::Default,
    ) {
        Err(safegl::buffer::BufferCreationError::BufferTypeNotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    let uniforms = uniform! {
        counter: &buffer,
    };

    // Texture size 1024x1024
    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniforms, &Default::default())
        .unwrap();

    let data = buffer.read().unwrap();
    assert_eq!(data, 1024 * 1024 + 10);

    display.assert_no_error(None);
}

/// Atomic counters on a driver below OpenGL 4.2 that has them through the extension, where the
/// buffer of each counter is found without the program interface query of OpenGL 4.3.
#[test]
fn through_the_extension() {
    let display = support::build_display();
    let extensions = support::extensions(&display);
    let header = match support::glsl_header(
        &display,
        420,
        &[(
            "GL_ARB_shader_atomic_counters",
            extensions.gl_arb_shader_atomic_counters,
        )],
    ) {
        Some(header) => header,
        None => return,
    };
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = safegl::Program::from_source(
        &display,
        &format!(
            "{header}
            in vec2 position;

            void main() {{
                gl_Position = vec4(position, 0.0, 1.0);
            }}
        "
        ),
        &format!(
            "{header}
            out vec4 f_color;

            layout(binding = 2, offset = 0) uniform atomic_uint counter;

            void main() {{
                f_color = vec4(0.0, 0.0, 0.0, 1.0);
                atomicCounterIncrement(counter);
            }}
        "
        ),
        None,
    )
    .unwrap();

    let buffer = safegl::buffer::Buffer::new(
        &display,
        &5u32,
        safegl::buffer::BufferType::AtomicCounterBuffer,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap();

    let texture = safegl::Texture2d::empty(&display, 16, 16).unwrap();
    texture
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { counter: &buffer },
            &Default::default(),
        )
        .unwrap();

    assert_eq!(buffer.read().unwrap(), 16 * 16 + 5);
    display.assert_no_error(None);
}
