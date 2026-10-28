//! What WebGL refuses where OpenGL ES accepts it: SafeGL returns an error instead of making a
//! WebGL error.
#![cfg(target_arch = "wasm32")]

#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType, CopyError};
use safegl::draw_parameters::{
    Blend, BlendingFunction, Depth, LinearBlendingFactor, PrimitiveRestart, Stencil, StencilTest,
};
use safegl::index::PrimitiveType;
use safegl::program::{LinkLocations, ProgramCreationError, ProgramCreationInput};
use safegl::{DrawError, DrawParameters, IndexBuffer, Surface, VertexBuffer};
use wasm_bindgen_test::wasm_bindgen_test as test;

mod support;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// Draws a line strip with `indices` and `parameters` on the canvas, which has a depth and a
/// stencil buffer.
fn draw(
    display: &support::TestDisplay,
    indices: &IndexBuffer<u16>,
    parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let positions = [[-0.5, -0.5], [0.5, -0.5], [-0.5, 0.5], [0.5, 0.5]];
    let vertices = positions.map(|position| Vertex { position });
    let vertices = VertexBuffer::new(display, &vertices).unwrap();
    let program = support::program_from_legacy(
        display,
        "#version 110\nattribute vec2 position;\n\
         void main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0); }",
    )
    .unwrap();
    let mut frame = display.draw();
    let result = frame.draw(&vertices, indices, &program, &uniform! {}, parameters);
    frame.finish().unwrap();
    result
}

fn strip(display: &support::TestDisplay, indices: &[u16]) -> IndexBuffer<u16> {
    IndexBuffer::new(display, PrimitiveType::LineStrip, indices).unwrap()
}

/// The two faces need the same stencil reference value and masks, but not the same tests and
/// operations; the mask of a test that always passes doesn't matter.
#[test]
fn stencil_faces() {
    let display = support::build_display();
    let indices = strip(&display, &[0, 1, 2, 3]);

    let stencil = Stencil {
        reference_value_clockwise: 1,
        reference_value_counter_clockwise: 2,
        test_clockwise: StencilTest::IfEqual { mask: 0xff },
        test_counter_clockwise: StencilTest::IfEqual { mask: 0xff },
        ..Default::default()
    };
    let parameters = DrawParameters {
        stencil,
        ..Default::default()
    };
    assert!(matches!(
        draw(&display, &indices, &parameters),
        Err(DrawError::StencilFacesMismatch)
    ));

    // the write masks are compared on the 8 bits of the stencil buffer
    let stencil = Stencil {
        test_clockwise: StencilTest::AlwaysPass,
        test_counter_clockwise: StencilTest::IfNotEqual { mask: 0x0f },
        write_mask_clockwise: 0xff,
        write_mask_counter_clockwise: 0xffff_ffff,
        ..Default::default()
    };
    let parameters = DrawParameters {
        stencil,
        ..Default::default()
    };
    draw(&display, &indices, &parameters).unwrap();

    display.assert_no_error(None);
}

/// A constant color factor and a constant alpha factor can't blend the colors together.
#[test]
fn blend_constants() {
    let display = support::build_display();
    let indices = strip(&display, &[0, 1, 2, 3]);
    let blend = |source, destination| DrawParameters {
        blend: Blend {
            color: BlendingFunction::Addition {
                source,
                destination,
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let mixed = blend(
        LinearBlendingFactor::ConstantColor,
        LinearBlendingFactor::OneMinusConstantAlpha,
    );
    assert!(matches!(
        draw(&display, &indices, &mixed),
        Err(DrawError::BlendConstantsMismatch)
    ));
    let mixed = blend(
        LinearBlendingFactor::ConstantAlpha,
        LinearBlendingFactor::OneMinusConstantColor,
    );
    assert!(matches!(
        draw(&display, &indices, &mixed),
        Err(DrawError::BlendConstantsMismatch)
    ));

    let colors = blend(
        LinearBlendingFactor::ConstantColor,
        LinearBlendingFactor::OneMinusConstantColor,
    );
    draw(&display, &indices, &colors).unwrap();
    // the alpha function isn't checked
    let parameters = DrawParameters {
        blend: Blend {
            alpha: BlendingFunction::Addition {
                source: LinearBlendingFactor::ConstantColor,
                destination: LinearBlendingFactor::ConstantAlpha,
            },
            ..Default::default()
        },
        ..Default::default()
    };
    draw(&display, &indices, &parameters).unwrap();

    display.assert_no_error(None);
}

/// The near value of the depth range can't be above its far value.
#[test]
fn reversed_depth_range() {
    let display = support::build_display();
    let indices = strip(&display, &[0, 1, 2, 3]);
    let range = |range| DrawParameters {
        depth: Depth {
            range,
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(matches!(
        draw(&display, &indices, &range((0.75, 0.25))),
        Err(DrawError::InvalidDepthRange)
    ));
    draw(&display, &indices, &range((0.25, 0.25))).unwrap();

    display.assert_no_error(None);
}

/// Index buffers only copy to and from index buffers.
#[test]
fn index_buffer_copies() {
    let display = support::build_display();
    let indices = strip(&display, &[0, 1, 2, 3]);
    let other = strip(&display, &[3, 2, 1, 0]);
    let data =
        Buffer::<[u16]>::new(&display, &[0, 0, 0, 0], BufferType::ArrayBuffer, BufferMode::Default)
            .unwrap();

    assert!(matches!(
        indices.copy_to(&data),
        Err(CopyError::IndexBufferMismatch)
    ));
    assert!(matches!(
        data.copy_to(&*other),
        Err(CopyError::IndexBufferMismatch)
    ));
    indices.copy_to(&*other).unwrap();
    assert_eq!(other.read().unwrap(), vec![0, 1, 2, 3]);

    display.assert_no_error(None);
}

/// WebGL reserves the names that start with `webgl_` and `_webgl_`, and the names longer than
/// 1024 bytes.
#[test]
fn reserved_link_location_names() {
    let display = support::build_display();
    let long = "a".repeat(1025);
    for name in ["webgl_position", "_webgl_position", &long] {
        let attributes = [(name, 0)];
        let program = safegl::Program::new(
            &display,
            ProgramCreationInput::SourceCode {
                vertex_shader: "#version 300 es\nin vec2 position;\n\
                                void main() { gl_Position = vec4(position, 0.0, 1.0); }",
                fragment_shader: "#version 300 es\nprecision mediump float;\nout vec4 color;\n\
                                  void main() { color = vec4(1.0); }",
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                geometry_shader: None,
                transform_feedback_varyings: None,
                outputs_srgb: false,
                uses_point_size: false,
                locations: LinkLocations {
                    attributes: &attributes,
                    ..Default::default()
                },
            },
        );
        assert!(
            matches!(program, Err(ProgramCreationError::InvalidLinkLocation { .. })),
            "{}",
            &name[..name.len().min(20)]
        );
    }

    display.assert_no_error(None);
}

/// WebGL always restarts primitives at the largest index value: disabling primitive restart is
/// an error when the index buffer may hold it.
#[test]
fn primitive_restart_always_on() {
    let display = support::build_display();
    let disabled = DrawParameters {
        primitive_restart: PrimitiveRestart::Disabled,
        ..Default::default()
    };
    let fixed = DrawParameters {
        primitive_restart: PrimitiveRestart::FixedIndex,
        ..Default::default()
    };
    let always_on = |result| matches!(result, Err(DrawError::PrimitiveRestartAlwaysOn));

    // in the data the buffer is created with
    let restart = strip(&display, &[0, 1, u16::MAX, 2, 3]);
    assert!(always_on(draw(&display, &restart, &disabled)));
    draw(&display, &restart, &fixed).unwrap();

    // written over, or written later
    let indices = strip(&display, &[0, 1, 2, 3, 0]);
    draw(&display, &indices, &disabled).unwrap();
    indices.write(&[0, 1, u16::MAX, 2, 3]);
    assert!(always_on(draw(&display, &indices, &disabled)));
    indices.write(&[0, 1, 2, 3, 0]);
    draw(&display, &indices, &disabled).unwrap();
    let buffer: &Buffer<[u16]> = &indices;
    buffer.slice(2..3).unwrap().write(&[u16::MAX]);
    assert!(always_on(draw(&display, &indices, &disabled)));

    // through a mapping
    let mut indices = strip(&display, &[0, 1, 2, 3, 0]);
    indices.map_write().set(4, u16::MAX);
    assert!(always_on(draw(&display, &indices, &disabled)));

    // copied from another buffer
    let indices = strip(&display, &[0, 1, 2, 3, 0]);
    restart.copy_to(&*indices).unwrap();
    assert!(always_on(draw(&display, &indices, &disabled)));

    // created empty: WebGL fills buffers with zeros
    let empty = IndexBuffer::<u16>::empty(&display, PrimitiveType::LineStrip, 4).unwrap();
    draw(&display, &empty, &disabled).unwrap();

    display.assert_no_error(None);
}

/// The result of a query is only available in a later task of the event loop: `get` returns an
/// error until then, and the query stays usable.
#[wasm_bindgen_test::wasm_bindgen_test]
async fn query_results_in_a_later_task() {
    use safegl::draw_parameters::{AnySamplesPassedQuery, QueryResultError};

    let display = support::build_display();
    let indices = strip(&display, &[0, 1, 2, 3]);
    let query = AnySamplesPassedQuery::new(&display, false).unwrap();
    let parameters = DrawParameters {
        samples_passed_query: Some((&query).into()),
        ..Default::default()
    };
    draw(&display, &indices, &parameters).unwrap();

    assert!(!query.is_ready());
    assert_eq!(query.get(), Err(QueryResultError::NotAvailable));
    support::until(|| query.is_ready()).await;
    assert_eq!(query.get(), Ok(true));

    display.assert_no_error(None);
}
