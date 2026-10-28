//! Draws whose commands are in buffers: one command at a time, several with one call, and
//! commands inside other values.

#![cfg(any(safegl_desktop, safegl_gles31))]

#[macro_use]
extern crate safegl;

use safegl::index::{
    DrawCommandIndices, DrawCommandNoIndices, DrawCommandsIndicesBuffer,
    DrawCommandsNoIndicesBuffer, PrimitiveType,
};
use safegl::{DrawError, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// Three quads that cover the left, middle and right thirds of the target, as triangle strips
/// of 4 vertices.
fn thirds(display: &Display) -> VertexBuffer<Vertex> {
    let mut vertices = Vec::new();
    for third in 0..3 {
        let (left, right) = (
            third as f32 * 2.0 / 3.0 - 1.0,
            (third + 1) as f32 * 2.0 / 3.0 - 1.0,
        );
        for position in [[left, -1.0], [right, -1.0], [left, 1.0], [right, 1.0]] {
            vertices.push(Vertex { position });
        }
    }
    VertexBuffer::new(display, &vertices).unwrap()
}

/// The command that draws the third `third` without indices.
fn third(third: u32) -> DrawCommandNoIndices {
    DrawCommandNoIndices {
        count: 4,
        instance_count: 1,
        first_index: third * 4,
        base_instance: 0,
    }
}

/// Draws `indices` over a black target, and returns which thirds are red.
fn drawn_thirds(
    display: &Display,
    indices: safegl::index::IndicesSource<'_>,
) -> Result<[bool; 3], DrawError> {
    let vertices = thirds(display);
    let program = support::program_from_legacy(
        display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0); }",
    )
    .unwrap();
    let target = safegl::Texture2d::empty(display, 6, 1).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);
    target.as_surface().draw(
        &vertices,
        indices,
        &program,
        &uniform! {},
        &Default::default(),
    )?;
    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    Ok([0, 2, 4].map(|x| data[0][x] == (255, 0, 0, 255)))
}

/// The buffer of commands, or `None` if the driver has no indirect draws.
fn commands_buffer(display: &Display) -> Option<DrawCommandsNoIndicesBuffer> {
    match DrawCommandsNoIndicesBuffer::empty(display, 3) {
        Ok(buffer) => Some(buffer),
        Err(safegl::buffer::BufferCreationError::BufferTypeNotSupported) => {
            assert!(support::is_gles(display));
            None
        }
        Err(error) => panic!("{error:?}"),
    }
}

#[test]
fn one_command_of_a_buffer() {
    let display = support::build_display();
    let commands = match commands_buffer(&display) {
        Some(commands) => commands,
        None => return,
    };
    commands.write(&[third(0), third(1), third(2)]);

    for index in 0..3 {
        let source = commands
            .command(index, PrimitiveType::TriangleStrip)
            .unwrap();
        let mut expected = [false; 3];
        expected[index] = true;
        assert_eq!(drawn_thirds(&display, source).unwrap(), expected);
    }
    assert!(commands.command(3, PrimitiveType::TriangleStrip).is_none());

    display.assert_no_error(None);
}

/// Several commands with one call need multidraw, which the driver may lack.
#[cfg(safegl_gl43)]
#[test]
fn all_the_commands_of_a_buffer() {
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    let commands = match commands_buffer(&display) {
        Some(commands) => commands,
        None => return,
    };
    commands.write(&[third(0), third(1), third(2)]);

    match drawn_thirds(
        &display,
        commands.with_primitive_type(PrimitiveType::TriangleStrip),
    ) {
        Ok(thirds) => assert_eq!(thirds, [true; 3]),
        Err(DrawError::MultidrawNotSupported) => {
            let extensions = display.get_extensions();
            assert!(
                display.get_opengl_version() < &safegl::Version(safegl::Api::Gl, 4, 3)
                    && !extensions.gl_arb_multi_draw_indirect
                    && !extensions.gl_ext_multi_draw_indirect
            );
        }
        Err(error) => panic!("{error}"),
    }

    display.assert_no_error(None);
}

/// An object whose draw command is in the middle of its data.
#[repr(C)]
#[derive(Copy, Clone)]
struct Object {
    bounds: [f32; 3],
    command: DrawCommandIndices,
    visible: u32,
}
implement_any_bit_pattern!(Object, bounds, command, visible);
implement_draw_command!(Object, command: DrawCommandIndices);

#[derive(Copy, Clone)]
struct Mesh {
    bounds: [f32; 4],
    command: DrawCommandNoIndices,
}
implement_any_bit_pattern!(Mesh, bounds, command);
implement_draw_command!(Mesh, command: DrawCommandNoIndices);

#[test]
fn commands_inside_other_values() {
    let display = support::build_display();
    if commands_buffer(&display).is_none() {
        return;
    }

    // with indices: the quads as two triangles each
    let indices: Vec<u16> = (0..3u16)
        .flat_map(|third| [0, 1, 2, 1, 3, 2].map(|index| third * 4 + index))
        .collect();
    let indices =
        safegl::IndexBuffer::new(&display, PrimitiveType::TrianglesList, &indices).unwrap();
    let objects: Vec<Object> = (0..3)
        .map(|third| Object {
            bounds: [third as f32; 3],
            command: DrawCommandIndices {
                count: 6,
                instance_count: 1,
                first_index: third * 6,
                base_vertex: 0,
                base_instance: 0,
            },
            visible: 1,
        })
        .collect();
    let objects = safegl::buffer::Buffer::new(
        &display,
        &objects[..],
        safegl::buffer::BufferType::DrawIndirectBuffer,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap();
    let objects = DrawCommandsIndicesBuffer::from_buffer(objects);
    let source = objects.command_with_index_buffer(2, &indices).unwrap();
    assert_eq!(
        drawn_thirds(&display, source).unwrap(),
        [false, false, true]
    );

    // without indices, where the struct has no `repr(C)`: the macro takes the offset of the field
    let meshes: Vec<Mesh> = (0..3)
        .map(|index| Mesh {
            bounds: [0.0; 4],
            command: third(index),
        })
        .collect();
    let meshes = safegl::buffer::Buffer::new(
        &display,
        &meshes[..],
        safegl::buffer::BufferType::DrawIndirectBuffer,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap();
    let meshes = DrawCommandsNoIndicesBuffer::from_buffer(meshes);
    let source = meshes.command(1, PrimitiveType::TriangleStrip).unwrap();
    assert_eq!(
        drawn_thirds(&display, source).unwrap(),
        [false, true, false]
    );

    #[cfg(safegl_gl43)]
    {
        for (source, expected) in [
            (objects.with_index_buffer(&indices), [true; 3]),
            (
                meshes.with_primitive_type(PrimitiveType::TriangleStrip),
                [true; 3],
            ),
        ] {
            match drawn_thirds(&display, source) {
                Ok(thirds) => assert_eq!(thirds, expected),
                Err(DrawError::MultidrawNotSupported) => (),
                Err(error) => panic!("{error}"),
            }
        }
    }

    display.assert_no_error(None);
}

/// OpenGL ES doesn't draw indirectly while transform feedback records.
#[test]
fn indirect_draws_with_transform_feedback_on_opengl_es() {
    let display = support::build_display();
    if !support::is_gles(&display) {
        return;
    }
    let commands = match commands_buffer(&display) {
        Some(commands) => commands,
        None => return,
    };
    commands.write(&[third(0), third(1), third(2)]);

    let program = safegl::Program::new(
        &display,
        safegl::program::ProgramCreationInput::SourceCode {
            vertex_shader: "#version 300 es\nin vec2 position;\nout vec2 captured;\n\
                            void main() {\n\
                                captured = position;\n\
                                gl_Position = vec4(position, 0.0, 1.0);\n\
                            }",
            fragment_shader: "#version 300 es\nprecision mediump float;\nout vec4 color;\n\
                              void main() { color = vec4(1.0); }",
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            transform_feedback_varyings: Some((
                vec!["captured".to_owned()],
                safegl::program::TransformFeedbackMode::Separate,
            )),
            outputs_srgb: false,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap();
    #[derive(Copy, Clone)]
    struct Captured {
        captured: [f32; 2],
    }
    implement_vertex!(Captured, captured);
    implement_any_bit_pattern!(Captured, captured);
    let mut output: VertexBuffer<Captured> = VertexBuffer::empty(&display, 12).unwrap();
    let session =
        safegl::vertex::TransformFeedback::new(&display, &program)
            .and_then(|transform_feedback| transform_feedback.with_buffer(&mut output)).unwrap();

    let vertices = thirds(&display);
    let target = support::build_renderable_texture(&display);
    let result = target.as_surface().draw(
        &vertices,
        commands.command(0, PrimitiveType::TriangleStrip).unwrap(),
        &program,
        &uniform! {},
        &safegl::DrawParameters {
            transform_feedback: Some(&session),
            ..Default::default()
        },
    );
    assert!(matches!(
        result,
        Err(DrawError::IndirectDrawWithTransformFeedback)
    ));

    display.assert_no_error(None);
}

/// A buffer without commands draws nothing, without needing multidraw.
// the source of a whole buffer of commands is the one of multidraw, from the gl43 level
#[cfg(safegl_gl43)]
#[test]
fn an_empty_buffer_draws_nothing() {
    let display = support::build_display();
    let commands = match DrawCommandsNoIndicesBuffer::empty(&display, 0) {
        Ok(commands) => commands,
        Err(safegl::buffer::BufferCreationError::BufferTypeNotSupported) => return,
        Err(error) => panic!("{error:?}"),
    };
    let source = commands.with_primitive_type(PrimitiveType::TriangleStrip);
    assert_eq!(drawn_thirds(&display, source).unwrap(), [false; 3]);

    display.assert_no_error(None);
}
