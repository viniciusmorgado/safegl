//! Transform feedback objects: several buffers, separate and interleaved captures, pauses, vertex
//! streams, and draws of the captured vertices.

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::{ProgramCreationInput, TransformFeedbackMode};
use safegl::vertex::{TransformFeedback, TransformFeedbackCreationError};
use safegl::{DrawError, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone, Debug, PartialEq)]
struct Index {
    index: f32,
}
implement_vertex!(Index, index);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Scalar {
    value: f32,
}
implement_vertex!(Scalar, value);
implement_any_bit_pattern!(Scalar, value);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Pair {
    pair: [f32; 2],
}
implement_vertex!(Pair, pair);
implement_any_bit_pattern!(Pair, pair);

/// The points `0` to `count - 1`.
fn points(display: &Display, count: usize) -> VertexBuffer<Index> {
    let points: Vec<Index> = (0..count)
        .map(|index| Index {
            index: index as f32,
        })
        .collect();
    VertexBuffer::new(display, &points).unwrap()
}

/// A program whose vertex shader writes `value = index * 10` and `pair = (index, -index)`, and
/// captures `varyings` in `mode`.
fn capturing_program(display: &Display, varyings: &[&str], mode: TransformFeedbackMode) -> Program {
    let (vertex, fragment) = if support::is_gles(display) {
        (
            "#version 300 es\nin float index;\nout float value;\nout vec2 pair;\n\
             void main() { value = index * 10.0; pair = vec2(index, -index); gl_PointSize = 1.0;\n\
             gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }",
            "#version 300 es\nprecision mediump float;\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
        )
    } else {
        (
            "#version 330\nin float index;\nout float value;\nout vec2 pair;\n\
             void main() { value = index * 10.0; pair = vec2(index, -index);\n\
             gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }",
            "#version 330\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
        )
    };
    Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: vertex,
            fragment_shader: fragment,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            transform_feedback_varyings: Some((
                varyings.iter().map(|varying| varying.to_string()).collect(),
                mode,
            )),
            outputs_srgb: false,
            uses_point_size: support::is_gles(display),
            locations: Default::default(),
        },
    )
    .unwrap()
}

/// Draws `vertices` as points with `parameters`, without rasterizing them.
fn capture<'a>(
    display: &Display,
    vertices: impl safegl::vertex::MultiVerticesSource<'a>,
    program: &Program,
    parameters: safegl::DrawParameters<'_>,
) -> Result<(), DrawError> {
    let target = support::build_renderable_texture(display);
    target.as_surface().draw(
        vertices,
        NoIndices(PrimitiveType::Points),
        program,
        &uniform! {},
        &safegl::DrawParameters {
            draw_primitives: false,
            ..parameters
        },
    )
}

/// Draws the captured vertices of `stream` with `sources` and `program`, and returns the number
/// of primitives drawn.
#[cfg(safegl_desktop)]
fn replay<'a>(
    display: &Display,
    sources: impl safegl::vertex::MultiVerticesSource<'a>,
    captured: &safegl::vertex::CapturedVertices,
    stream: u32,
    program: &Program,
) -> Result<u64, DrawError> {
    let query = safegl::draw_parameters::PrimitivesGeneratedQuery::new(display).unwrap();
    let target = support::build_renderable_texture(display);
    target.as_surface().draw(
        sources,
        captured.stream_indices(stream, PrimitiveType::Points),
        program,
        &uniform! {},
        &safegl::DrawParameters {
            primitives_generated_queries: &[&query],
            ..Default::default()
        },
    )?;
    Ok(query.get().unwrap())
}

#[test]
fn separate_buffers() {
    let display = support::build_display();
    let program = capturing_program(
        &display,
        &["value", "pair"],
        TransformFeedbackMode::Separate,
    );
    let mut values: VertexBuffer<Scalar> = VertexBuffer::empty(&display, 3).unwrap();
    let mut pairs: VertexBuffer<Pair> = VertexBuffer::empty(&display, 3).unwrap();

    {
        let transform_feedback = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut values)
            .unwrap()
            .with_buffer(&mut pairs)
            .unwrap();
        capture(
            &display,
            &points(&display, 3),
            &program,
            safegl::DrawParameters {
                transform_feedback: Some(&transform_feedback),
                ..Default::default()
            },
        )
        .unwrap();
    }

    let values: Vec<f32> = values.read().unwrap().iter().map(|v| v.value).collect();
    assert_eq!(values, [0.0, 10.0, 20.0]);
    let pairs: Vec<[f32; 2]> = pairs.read().unwrap().iter().map(|p| p.pair).collect();
    assert_eq!(pairs, [[0.0, -0.0], [1.0, -1.0], [2.0, -2.0]]);

    display.assert_no_error(None);
}

#[test]
fn buffers_of_the_program() {
    let display = support::build_display();
    let program = capturing_program(
        &display,
        &["value", "pair"],
        TransformFeedbackMode::Separate,
    );
    let mut values: VertexBuffer<Scalar> = VertexBuffer::empty(&display, 3).unwrap();
    let mut pairs: VertexBuffer<Pair> = VertexBuffer::empty(&display, 3).unwrap();
    let mut extra: VertexBuffer<Pair> = VertexBuffer::empty(&display, 3).unwrap();

    // the buffers must follow the order and the formats of the program
    let wrong = TransformFeedback::new(&display, &program)
        .unwrap()
        .with_buffer(&mut pairs);
    assert_eq!(
        wrong.err(),
        Some(TransformFeedbackCreationError::WrongVertexFormat)
    );
    let too_many = TransformFeedback::new(&display, &program)
        .unwrap()
        .with_buffer(&mut values)
        .unwrap()
        .with_buffer(&mut pairs)
        .unwrap()
        .with_buffer(&mut extra);
    assert_eq!(
        too_many.err(),
        Some(TransformFeedbackCreationError::TooManyBuffers)
    );

    // all of them before drawing
    let missing = TransformFeedback::new(&display, &program)
        .unwrap()
        .with_buffer(&mut values)
        .unwrap();
    let result = capture(
        &display,
        &points(&display, 3),
        &program,
        safegl::DrawParameters {
            transform_feedback: Some(&missing),
            ..Default::default()
        },
    );
    assert!(matches!(
        result,
        Err(DrawError::TransformFeedbackBuffersMissing)
    ));
    drop(missing);

    let (_, _, nothing) = support::build_fullscreen_red_pipeline(&display);
    assert_eq!(
        TransformFeedback::new(&display, &nothing).err(),
        Some(TransformFeedbackCreationError::NothingCaptured)
    );

    display.assert_no_error(None);
}

/// A draw without the object pauses its capture, which the next draw with it resumes after the
/// vertices it captured.
#[test]
fn captures_pause_and_resume() {
    let display = support::build_display();
    let program = capturing_program(&display, &["value"], TransformFeedbackMode::Interleaved);
    let mut first: VertexBuffer<Scalar> = VertexBuffer::empty(&display, 6).unwrap();
    let mut second: VertexBuffer<Scalar> = VertexBuffer::empty(&display, 6).unwrap();
    let (vb, ib, other_program) = support::build_fullscreen_red_pipeline(&display);
    let target = support::build_renderable_texture(&display);

    {
        let a = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut first)
            .unwrap();
        let b = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut second)
            .unwrap();
        let with = |transform_feedback| safegl::DrawParameters {
            transform_feedback: Some(transform_feedback),
            ..Default::default()
        };
        let three = points(&display, 3);

        capture(&display, &three, &program, with(&a)).unwrap();
        // another program draws in between, then another capture
        target
            .as_surface()
            .draw(&vb, &ib, &other_program, &uniform! {}, &Default::default())
            .unwrap();
        capture(&display, &three, &program, with(&b)).unwrap();
        capture(&display, &three, &program, with(&a)).unwrap();
        capture(&display, &three, &program, with(&b)).unwrap();

        // the type of primitives of a capture can't change
        let result = target.as_surface().draw(
            &three,
            NoIndices(PrimitiveType::LinesList),
            &program,
            &uniform! {},
            &with(&a),
        );
        assert!(matches!(
            result,
            Err(DrawError::TransformFeedbackPrimitivesMismatch)
        ));
    }

    for buffer in [first, second] {
        let values: Vec<f32> = buffer.read().unwrap().iter().map(|v| v.value).collect();
        assert_eq!(values, [0.0, 10.0, 20.0, 0.0, 10.0, 20.0]);
    }

    display.assert_no_error(None);
}

/// Interleaved buffers separated by `gl_NextBuffer`, with components left unwritten by
/// `gl_SkipComponents`, which only desktop OpenGL has.
#[test]
fn skipped_components_and_buffers() {
    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let program = capturing_program(
        &display,
        &[
            "value",
            "gl_SkipComponents2",
            "gl_NextBuffer",
            "gl_SkipComponents1",
            "pair",
        ],
        TransformFeedbackMode::Interleaved,
    );

    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    struct First {
        value: f32,
        skipped: [f32; 2],
    }
    implement_vertex!(First, value, skipped);
    implement_any_bit_pattern!(First, value, skipped);
    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    struct Second {
        skipped: f32,
        pair: [f32; 2],
    }
    implement_vertex!(Second, skipped, pair);
    implement_any_bit_pattern!(Second, skipped, pair);

    // the skipped components keep their values
    let mut first = VertexBuffer::new(
        &display,
        &[First {
            value: 7.0,
            skipped: [7.0; 2],
        }; 2],
    )
    .unwrap();
    let mut second = VertexBuffer::new(
        &display,
        &[Second {
            skipped: 7.0,
            pair: [7.0; 2],
        }; 2],
    )
    .unwrap();
    {
        let transform_feedback = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut first)
            .unwrap()
            .with_buffer(&mut second)
            .unwrap();
        capture(
            &display,
            &points(&display, 2),
            &program,
            safegl::DrawParameters {
                transform_feedback: Some(&transform_feedback),
                ..Default::default()
            },
        )
        .unwrap();
    }

    assert_eq!(
        first.read().unwrap(),
        [
            First {
                value: 0.0,
                skipped: [7.0; 2]
            },
            First {
                value: 10.0,
                skipped: [7.0; 2]
            }
        ]
    );
    assert_eq!(
        second.read().unwrap(),
        [
            Second {
                skipped: 7.0,
                pair: [0.0, -0.0]
            },
            Second {
                skipped: 7.0,
                pair: [1.0, -1.0]
            }
        ]
    );

    display.assert_no_error(None);
}

/// The captured vertices drawn without reading their number back, on desktop OpenGL.
#[cfg(safegl_desktop)]
#[test]
fn draws_of_captured_vertices() {
    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let program = capturing_program(&display, &["value"], TransformFeedbackMode::Interleaved);
    let mut captured: VertexBuffer<Scalar> = VertexBuffer::empty(&display, 8).unwrap();
    let captured_vertices = {
        let transform_feedback = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut captured)
            .unwrap();
        capture(
            &display,
            &points(&display, 5),
            &program,
            safegl::DrawParameters {
                transform_feedback: Some(&transform_feedback),
                ..Default::default()
            },
        )
        .unwrap();
        transform_feedback.finish()
    };

    // a program that reads the captured values
    let program_replay = support::program_from_legacy(
        &display,
        "#version 110\nattribute float value;\nvoid main() { gl_Position = vec4(value / 100.0, 0.0, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0); }",
    )
    .unwrap();
    // the 5 captured points, not the 8 that the buffer holds
    assert_eq!(
        replay(&display, &captured, &captured_vertices, 0, &program_replay).unwrap(),
        5
    );
    // several instances, as many times
    if display.get_opengl_version() >= &safegl::Version(safegl::Api::Gl, 4, 2) {
        let instances = safegl::vertex::EmptyInstanceAttributes { len: 3 };
        let count = replay(
            &display,
            (&captured, instances),
            &captured_vertices,
            0,
            &program_replay,
        );
        assert_eq!(count.unwrap(), 15);
    }
    // the vertex sources must hold the whole capture
    let small: VertexBuffer<Scalar> = VertexBuffer::empty(&display, 4).unwrap();
    assert!(matches!(
        replay(&display, &small, &captured_vertices, 0, &program_replay),
        Err(DrawError::CapturedVerticesOutOfBounds)
    ));

    display.assert_no_error(None);
}

/// Two vertex streams of a geometry shader, captured into their own buffers and counted by
/// their own queries, on desktop OpenGL.
#[cfg(safegl_desktop)]
#[test]
fn vertex_streams() {
    use safegl::draw_parameters::{
        PrimitivesGeneratedQuery, TransformFeedbackPrimitivesWrittenQuery,
    };

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let program = Program::new(
        &display,
        ProgramCreationInput::SourceCode {
            vertex_shader: "#version 400\nin float index;\nout float v_index;\n\
                            void main() { v_index = index; }",
            geometry_shader: Some(
                "#version 400\nlayout(points) in;\nlayout(points, max_vertices = 3) out;\n\
                 in float v_index[];\n\
                 layout(stream = 0) out float first;\nlayout(stream = 1) out float second;\n\
                 void main() {\n\
                     first = v_index[0];\nEmitStreamVertex(0);\nEndStreamPrimitive(0);\n\
                     for (int i = 0; i < 2; i++) {\n\
                         second = v_index[0] * 10.0 + float(i);\nEmitStreamVertex(1);\n\
                         EndStreamPrimitive(1);\n\
                     }\n\
                 }",
            ),
            fragment_shader: "#version 400\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            transform_feedback_varyings: Some((
                vec![
                    "first".to_owned(),
                    "gl_NextBuffer".to_owned(),
                    "second".to_owned(),
                ],
                TransformFeedbackMode::Interleaved,
            )),
            outputs_srgb: false,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap();

    #[derive(Copy, Clone, Debug, PartialEq)]
    struct First {
        first: f32,
    }
    implement_vertex!(First, first);
    implement_any_bit_pattern!(First, first);
    #[derive(Copy, Clone, Debug, PartialEq)]
    struct Second {
        second: f32,
    }
    implement_vertex!(Second, second);
    implement_any_bit_pattern!(Second, second);

    let mut firsts: VertexBuffer<First> = VertexBuffer::empty(&display, 2).unwrap();
    let mut seconds: VertexBuffer<Second> = VertexBuffer::empty(&display, 4).unwrap();
    let generated =
        [0, 1].map(|stream| PrimitivesGeneratedQuery::with_stream(&display, stream).unwrap());
    let written = [0, 1].map(|stream| {
        TransformFeedbackPrimitivesWrittenQuery::with_stream(&display, stream).unwrap()
    });
    let captured = {
        let transform_feedback = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut firsts)
            .unwrap()
            .with_buffer(&mut seconds)
            .unwrap();
        capture(
            &display,
            &points(&display, 2),
            &program,
            safegl::DrawParameters {
                transform_feedback: Some(&transform_feedback),
                primitives_generated_queries: &[&generated[0], &generated[1]],
                transform_feedback_primitives_written_queries: &[&written[0], &written[1]],
                ..Default::default()
            },
        )
        .unwrap();
        transform_feedback.finish()
    };

    assert_eq!(generated.map(|query| query.get().unwrap()), [2, 4]);
    assert_eq!(written.map(|query| query.get().unwrap()), [2, 4]);
    let firsts_read: Vec<f32> = firsts.read().unwrap().iter().map(|v| v.first).collect();
    assert_eq!(firsts_read, [0.0, 1.0]);
    let seconds_read: Vec<f32> = seconds.read().unwrap().iter().map(|v| v.second).collect();
    assert_eq!(seconds_read, [0.0, 1.0, 10.0, 11.0]);

    // the vertices of stream 1 drawn again
    let program_replay = support::program_from_legacy(
        &display,
        "#version 110\nattribute float second;\nvoid main() { gl_Position = vec4(second / 100.0, 0.0, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0); }",
    )
    .unwrap();
    assert_eq!(
        replay(&display, &seconds, &captured, 1, &program_replay).unwrap(),
        4
    );

    display.assert_no_error(None);
}

/// A rebuilt context loses the captures of the previous one.
#[cfg(safegl_desktop)]
#[test]
fn captures_lost_with_the_context() {
    let mut display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let program = capturing_program(&display, &["value"], TransformFeedbackMode::Interleaved);
    let mut captured: VertexBuffer<Scalar> = VertexBuffer::empty(&display, 2).unwrap();
    let captured_vertices = {
        let transform_feedback = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut captured)
            .unwrap();
        capture(
            &display,
            &points(&display, 2),
            &program,
            safegl::DrawParameters {
                transform_feedback: Some(&transform_feedback),
                ..Default::default()
            },
        )
        .unwrap();
        transform_feedback.finish()
    };

    support::rebuild_display(&mut display);

    let replay = support::program_from_legacy(
        &display,
        "#version 110\nattribute float value;\nvoid main() { gl_Position = vec4(value, 0.0, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0); }",
    )
    .unwrap();
    let target = support::build_renderable_texture(&display);
    let result = target.as_surface().draw(
        &captured,
        captured_vertices.indices(PrimitiveType::Points),
        &replay,
        &uniform! {},
        &Default::default(),
    );
    assert!(matches!(result, Err(DrawError::CapturedVerticesLost)));

    display.assert_no_error(None);
}

/// Varyings placed by `xfb_buffer`, `xfb_offset` and `xfb_stride` in the shader (OpenGL 4.4).
#[test]
fn layouts_of_the_shader() {
    let display = support::build_display();
    if support::is_gles(&display) || !support::glsl_supported(&display, 440, 0) {
        return;
    }
    let program = Program::new(
        &display,
        ProgramCreationInput::SourceCode {
            vertex_shader: "#version 440\nin float index;\n\
                            layout(xfb_buffer = 0, xfb_offset = 8) out float value;\n\
                            layout(xfb_buffer = 1, xfb_stride = 16, xfb_offset = 4) out vec2 pair;\n\
                            void main() { value = index * 10.0; pair = vec2(index, -index);\n\
                            gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }",
            fragment_shader: "#version 440\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb: false,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap();

    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    struct First {
        skipped: [f32; 2],
        value: f32,
    }
    implement_vertex!(First, skipped, value);
    implement_any_bit_pattern!(First, skipped, value);
    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    struct Second {
        before: f32,
        pair: [f32; 2],
        after: f32,
    }
    implement_vertex!(Second, before, pair, after);
    implement_any_bit_pattern!(Second, before, pair, after);

    let buffers = program.get_transform_feedback_buffers();
    assert_eq!(buffers.len(), 2);
    assert_eq!((buffers[0].stride, buffers[1].stride), (12, 16));

    let mut first = VertexBuffer::new(
        &display,
        &[First {
            skipped: [7.0; 2],
            value: 7.0,
        }; 2],
    )
    .unwrap();
    let mut second = VertexBuffer::new(
        &display,
        &[Second {
            before: 7.0,
            pair: [7.0; 2],
            after: 7.0,
        }; 2],
    )
    .unwrap();
    {
        let transform_feedback = TransformFeedback::new(&display, &program)
            .unwrap()
            .with_buffer(&mut first)
            .unwrap()
            .with_buffer(&mut second)
            .unwrap();
        capture(
            &display,
            &points(&display, 2),
            &program,
            safegl::DrawParameters {
                transform_feedback: Some(&transform_feedback),
                ..Default::default()
            },
        )
        .unwrap();
    }

    assert_eq!(
        first.read().unwrap(),
        [
            First {
                skipped: [7.0; 2],
                value: 0.0
            },
            First {
                skipped: [7.0; 2],
                value: 10.0
            }
        ]
    );
    assert_eq!(
        second.read().unwrap(),
        [
            Second {
                before: 7.0,
                pair: [0.0, -0.0],
                after: 7.0
            },
            Second {
                before: 7.0,
                pair: [1.0, -1.0],
                after: 7.0
            }
        ]
    );

    display.assert_no_error(None);
}
