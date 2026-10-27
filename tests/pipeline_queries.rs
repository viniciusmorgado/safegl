//! Pipeline statistics and transform feedback overflow queries.
#![cfg(safegl_gl46)]

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::{
    ConditionalRendering, PipelineStatistic, PipelineStatisticsQuery, QueryCreationError,
    TransformFeedbackOverflowQuery,
};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::ComputeShader;
use safegl::{DrawError, DrawParameters, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone, Debug, PartialEq)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Captured {
    output_val: [f32; 2],
}
implement_vertex!(Captured, output_val);
implement_any_bit_pattern!(Captured, output_val);

/// Two triangles that cover the target.
fn quad(display: &Display) -> VertexBuffer<Vertex> {
    let positions = [
        [-1.0, -1.0],
        [1.0, -1.0],
        [-1.0, 1.0],
        [1.0, -1.0],
        [1.0, 1.0],
        [-1.0, 1.0],
    ];
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

/// A program that draws white and captures the positions in `output_val`.
fn program(display: &Display) -> Program {
    let source = safegl::program::ProgramCreationInput::SourceCode {
        vertex_shader: "#version 330
            in vec2 position;
            out vec2 output_val;
            void main() { output_val = position; gl_Position = vec4(position, 0.0, 1.0); }",
        fragment_shader: "#version 330
            out vec4 color;
            void main() { color = vec4(1.0); }",
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        geometry_shader: None,
        transform_feedback_varyings: Some((
            vec!["output_val".to_owned()],
            safegl::program::TransformFeedbackMode::Separate,
        )),
        outputs_srgb: false,
        uses_point_size: false,
    };
    Program::new(display, source).unwrap()
}

/// The query of `statistic`, or `None` if the backend doesn't have pipeline statistics, which
/// desktop OpenGL 4.6 has.
fn statistic(display: &Display, statistic: PipelineStatistic) -> Option<PipelineStatisticsQuery> {
    match PipelineStatisticsQuery::new(display, statistic) {
        Ok(query) => Some(query),
        Err(QueryCreationError::NotSupported) => {
            assert!(support::is_gles(display));
            None
        }
    }
}

fn overflow(display: &Display) -> Option<TransformFeedbackOverflowQuery> {
    match TransformFeedbackOverflowQuery::new(display) {
        Ok(query) => Some(query),
        Err(QueryCreationError::NotSupported) => {
            assert!(support::is_gles(display));
            None
        }
    }
}

#[test]
fn statistics_count_the_draws() {
    let display = support::build_display();
    let queries = [
        PipelineStatistic::VerticesSubmitted,
        PipelineStatistic::PrimitivesSubmitted,
        PipelineStatistic::VertexShaderInvocations,
        PipelineStatistic::ClippingInputPrimitives,
        PipelineStatistic::FragmentShaderInvocations,
    ]
    .map(|s| statistic(&display, s));
    let queries: Vec<PipelineStatisticsQuery> = match queries.into_iter().collect() {
        Some(queries) => queries,
        None => return,
    };
    let program = program(&display);
    let quad = quad(&display);
    let texture = safegl::Texture2d::empty(&display, 8, 8).unwrap();

    // two draws that follow each other add up
    let list: Vec<&PipelineStatisticsQuery> = queries.iter().collect();
    let parameters = DrawParameters {
        pipeline_statistics_queries: &list,
        ..Default::default()
    };
    for _ in 0..2 {
        texture
            .as_surface()
            .draw(
                &quad,
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &parameters,
            )
            .unwrap();
    }

    let counts: Vec<u64> = queries.into_iter().map(|q| q.get().unwrap()).collect();
    let [vertices, primitives, vertex_shaders, clipped, fragments]: [u64; 5] =
        counts.try_into().unwrap();
    assert_eq!(vertices, 12);
    assert_eq!(primitives, 4);
    assert!((2..=12).contains(&vertex_shaders), "{vertex_shaders} vertex shaders");
    assert_eq!(clipped, 4);
    // every pixel of both draws, and maybe helper invocations along the diagonal
    assert!(fragments >= 2 * 64, "{fragments} fragment shaders");

    display.assert_no_error(None);
}

#[test]
fn a_query_ends_when_a_command_is_not_given_it() {
    let display = support::build_display();
    let query = match statistic(&display, PipelineStatistic::VerticesSubmitted) {
        Some(query) => query,
        None => return,
    };
    let program = program(&display);
    let quad = quad(&display);
    let texture = safegl::Texture2d::empty(&display, 8, 8).unwrap();
    let draw = |parameters: &DrawParameters<'_>| {
        texture.as_surface().draw(
            &quad,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &uniform! {},
            parameters,
        )
    };

    let counted = DrawParameters {
        pipeline_statistics_queries: &[&query],
        ..Default::default()
    };
    draw(&counted).unwrap();
    draw(&Default::default()).unwrap();
    // OpenGL can't resume a query that ended
    assert!(matches!(draw(&counted), Err(DrawError::WrongQueryOperation)));
    assert_eq!(query.get().unwrap(), 6);

    // two queries can't count the same statistic at once
    let a = statistic(&display, PipelineStatistic::PrimitivesSubmitted).unwrap();
    let b = statistic(&display, PipelineStatistic::PrimitivesSubmitted).unwrap();
    let both = DrawParameters {
        pipeline_statistics_queries: &[&a, &b],
        ..Default::default()
    };
    assert!(matches!(draw(&both), Err(DrawError::WrongQueryOperation)));

    display.assert_no_error(None);
}

#[test]
fn compute_shader_invocations() {
    let display = support::build_display();
    // a driver below OpenGL 4.3 can have compute shaders without GLSL 4.30
    if !ComputeShader::is_supported(&display) || !support::glsl_supported(&display, 430, 310) {
        return;
    }
    let query = match statistic(&display, PipelineStatistic::ComputeShaderInvocations) {
        Some(query) => query,
        None => return,
    };
    let shader = ComputeShader::from_source(
        &display,
        "
            #version 430
            layout(local_size_x = 8) in;
            void main() {}
        ",
    )
    .unwrap();

    shader
        .execute_with_statistics(uniform! {}, 4, 1, 1, &[&query])
        .unwrap();
    shader
        .execute_with_statistics(uniform! {}, 2, 1, 1, &[&query])
        .unwrap();
    assert_eq!(query.get().unwrap(), 6 * 8);

    display.assert_no_error(None);
}

/// Captures the two triangles of the quad into a buffer of `capacity` vertices, with `query`
/// telling whether transform feedback overflowed.
fn captures_overflow(display: &Display, capacity: usize, query: &TransformFeedbackOverflowQuery) {
    let program = program(display);
    let quad = quad(display);
    let mut captured: VertexBuffer<Captured> = VertexBuffer::empty(display, capacity).unwrap();
    let texture = safegl::Texture2d::empty(display, 8, 8).unwrap();
    let session =
        safegl::vertex::TransformFeedbackSession::new(display, &program, &mut captured).unwrap();
    let parameters = DrawParameters {
        transform_feedback: Some(&session),
        transform_feedback_overflow_query: Some(query),
        draw_primitives: false,
        ..Default::default()
    };
    texture
        .as_surface()
        .draw(
            &quad,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &uniform! {},
            &parameters,
        )
        .unwrap();
}

#[test]
fn transform_feedback_overflow() {
    let display = support::build_display();
    let (fits, overflows) = match (overflow(&display), overflow(&display)) {
        (Some(fits), Some(overflows)) => (fits, overflows),
        _ => return,
    };

    captures_overflow(&display, 6, &fits);
    // a triangle doesn't fit in 5 vertices
    captures_overflow(&display, 5, &overflows);
    assert!(!fits.get().unwrap());
    assert!(overflows.get().unwrap());

    // the overflow of a single stream
    let stream = TransformFeedbackOverflowQuery::with_stream(&display, 0).unwrap();
    captures_overflow(&display, 3, &stream);
    assert!(stream.get().unwrap());
    assert!(matches!(
        TransformFeedbackOverflowQuery::with_stream(&display, 1000),
        Err(QueryCreationError::NotSupported)
    ));

    display.assert_no_error(None);
}

#[test]
fn overflow_as_condition() {
    let display = support::build_display();
    let (fits, overflows) = match (overflow(&display), overflow(&display)) {
        (Some(fits), Some(overflows)) => (fits, overflows),
        _ => return,
    };
    captures_overflow(&display, 6, &fits);
    captures_overflow(&display, 3, &overflows);

    let program = program(&display);
    let quad = quad(&display);
    let texture = safegl::Texture2d::empty(&display, 8, 8).unwrap();
    for (query, drawn) in [(&fits, false), (&overflows, true)] {
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        let parameters = DrawParameters {
            condition: Some(ConditionalRendering {
                query: query.into(),
                wait: true,
                per_region: false,
            }),
            ..Default::default()
        };
        texture
            .as_surface()
            .draw(
                &quad,
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &parameters,
            )
            .unwrap();
        let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
        let expected = if drawn { (255, 255, 255, 255) } else { (0, 0, 0, 0) };
        assert_eq!(data[4][4], expected, "overflowed: {drawn}");
    }

    display.assert_no_error(None);
}
