#[macro_use]
extern crate safegl;

use safegl::Surface;


mod support;

#[test]
fn transform_feedback() {
    let display = support::build_display();

    #[derive(Copy, Clone, PartialEq)]
    struct Vertex {
        output_val: (f32, f32),
    }

    implement_vertex!(Vertex, output_val);
    implement_any_bit_pattern!(Vertex, output_val);

    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    let (vertex_shader, fragment_shader) = if support::is_gles(&display) {
        (
            "#version 300 es
            in vec2 position;
            out vec2 output_val;
            void main() {
                output_val = position;
                gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
            }",
            "#version 300 es
            precision mediump float;
            out vec4 color;
            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }",
        )
    } else {
        (
            "
            #version 110

            attribute vec2 position;

            varying vec2 output_val;

            void main() {
                output_val = position;
                gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
            }
        ",
            "
            #version 130

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
        )
    };

    let source = safegl::program::ProgramCreationInput::SourceCode {
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        geometry_shader: None,
        outputs_srgb: false,
        uses_point_size: false,

        vertex_shader,
        fragment_shader,

        transform_feedback_varyings: Some((
            vec!["output_val".to_string()],
            safegl::program::TransformFeedbackMode::Separate,
        )),
    };

    let program = match safegl::Program::new(&display, source) {
        Ok(p) => p,
        Err(safegl::program::ProgramCreationError::TransformFeedbackNotSupported) => return,
        Err(e) => panic!("{:?}", e),
    };

    let mut out_buffer: safegl::VertexBuffer<Vertex> =
        safegl::VertexBuffer::empty(&display, 6).unwrap();

    // counts the captured primitives
    let query = safegl::draw_parameters::TransformFeedbackPrimitivesWrittenQuery::new(&display)
        .unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    {
        let session =
            safegl::vertex::TransformFeedbackSession::new(&display, &program, &mut out_buffer)
                .unwrap();

        // only capturing the vertices, on OpenGL and OpenGL ES alike
        let params = safegl::DrawParameters {
            transform_feedback: Some(&session),
            transform_feedback_primitives_written_query: Some(&query),
            draw_primitives: false,
            ..Default::default()
        };

        texture
            .as_surface()
            .draw(&vb, &ib, &program, &uniform! {}, &params)
            .unwrap();
    }

    assert_eq!(query.get().unwrap(), 2);

    // nothing was rasterized
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert!(pixels.iter().flatten().all(|&p| p == (0, 0, 0, 0)));

    let result = match out_buffer.read() {
        Ok(r) => r,
        Err(safegl::buffer::ReadError::NotSupported) => return,
        e => e.unwrap(),
    };

    assert_eq!(result[0].output_val, (-1.0, 1.0));
    assert_eq!(result[1].output_val, (1.0, 1.0));
    assert_eq!(result[2].output_val, (-1.0, -1.0));
    assert_eq!(result[3].output_val, (-1.0, -1.0));
    assert_eq!(result[4].output_val, (1.0, 1.0));
    assert_eq!(result[5].output_val, (1.0, -1.0));

    display.assert_no_error(None);
}

/// Builds a program that captures `output_val` from the positions of the vertices.
fn capturing_program(display: &safegl::Display<safegl::glutin::surface::WindowSurface>) -> Option<safegl::Program> {
    let (vertex_shader, fragment_shader) = if support::is_gles(display) {
        (
            "#version 300 es\nin vec2 position;\nout vec2 output_val;\nvoid main() { output_val = position; gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }",
            "#version 300 es\nprecision mediump float;\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
        )
    } else {
        (
            "#version 330\nin vec2 position;\nout vec2 output_val;\nvoid main() { output_val = position; gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }",
            "#version 330\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
        )
    };

    let source = safegl::program::ProgramCreationInput::SourceCode {
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        geometry_shader: None,
        outputs_srgb: false,
        uses_point_size: false,
        vertex_shader,
        fragment_shader,
        transform_feedback_varyings: Some((
            vec!["output_val".to_string()],
            safegl::program::TransformFeedbackMode::Separate,
        )),
    };
    match safegl::Program::new(display, source) {
        Ok(p) => Some(p),
        Err(safegl::program::ProgramCreationError::TransformFeedbackNotSupported) => None,
        Err(e) => panic!("{:?}", e),
    }
}

#[test]
fn transform_feedback_session_continues() {
    let display = support::build_display();

    #[derive(Copy, Clone, PartialEq, Debug)]
    struct Vertex {
        output_val: (f32, f32),
    }
    implement_vertex!(Vertex, output_val);
    implement_any_bit_pattern!(Vertex, output_val);

    let program = match capturing_program(&display) {
        Some(p) => p,
        None => return,
    };
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let mut out_buffer: safegl::VertexBuffer<Vertex> =
        safegl::VertexBuffer::empty(&display, 12).unwrap();
    let texture = support::build_renderable_texture(&display);

    {
        let session =
            safegl::vertex::TransformFeedbackSession::new(&display, &program, &mut out_buffer)
                .unwrap();
        let params = safegl::DrawParameters {
            transform_feedback: Some(&session),
            draw_primitives: false,
            ..Default::default()
        };

        // the second draw appends its vertices after those of the first one
        for _ in 0..2 {
            texture
                .as_surface()
                .draw(&vb, &ib, &program, &uniform! {}, &params)
                .unwrap();
        }
    }

    let result = out_buffer.read().unwrap();
    assert_eq!(result[0].output_val, (-1.0, 1.0));
    assert_eq!(result[6].output_val, (-1.0, 1.0));
    assert_eq!(result[11].output_val, (1.0, -1.0));

    display.assert_no_error(None);
}

#[test]
fn transform_feedback_session_of_another_program() {
    let display = support::build_display();

    #[derive(Copy, Clone)]
    struct Vertex {
        output_val: (f32, f32),
    }
    implement_vertex!(Vertex, output_val);
    implement_any_bit_pattern!(Vertex, output_val);

    let program = match capturing_program(&display) {
        Some(p) => p,
        None => return,
    };
    let (vb, ib, other_program) = support::build_fullscreen_red_pipeline(&display);
    let mut out_buffer: safegl::VertexBuffer<Vertex> =
        safegl::VertexBuffer::empty(&display, 6).unwrap();
    let texture = support::build_renderable_texture(&display);

    let session =
        safegl::vertex::TransformFeedbackSession::new(&display, &program, &mut out_buffer).unwrap();
    let params = safegl::DrawParameters {
        transform_feedback: Some(&session),
        ..Default::default()
    };
    let result = texture
        .as_surface()
        .draw(&vb, &ib, &other_program, &uniform! {}, &params);
    assert!(matches!(
        result,
        Err(safegl::DrawError::TransformFeedbackProgramMismatch)
    ));

    display.assert_no_error(None);
}
