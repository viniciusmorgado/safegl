//! Multidraws that read their number of commands from a buffer (`glMultiDraw*IndirectCount`).
#![cfg(safegl_gl46)]

#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType};
use safegl::index::{
    DrawCommandIndices, DrawCommandNoIndices, DrawCommandsIndicesBuffer,
    DrawCommandsNoIndicesBuffer, IndicesSource, PrimitiveType,
};
use safegl::program::ComputeShader;
use safegl::backend::Facade;
use safegl::{CapabilitiesSource, DrawError, IndexBuffer, Program, Surface, Texture2d, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

/// The number of commands: each draws a vertical third of the target.
const COMMANDS: usize = 3;

/// The colors of the thirds.
const COLORS: [[f32; 3]; COMMANDS] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Two triangles per third of the target, from left to right.
fn vertices(display: &Display) -> VertexBuffer<Vertex> {
    let vertices: Vec<Vertex> = (0..COMMANDS)
        .flat_map(|i| {
            let left = -1.0 + 2.0 * i as f32 / COMMANDS as f32;
            let right = -1.0 + 2.0 * (i + 1) as f32 / COMMANDS as f32;
            [
                [left, -1.0],
                [right, -1.0],
                [left, 1.0],
                [right, -1.0],
                [right, 1.0],
                [left, 1.0],
            ]
            .map(|position| Vertex {
                position,
                color: COLORS[i],
            })
        })
        .collect();
    VertexBuffer::new(display, &vertices).unwrap()
}

fn program(display: &Display) -> Program {
    program!(display,
        330 => {
            vertex: "
                #version 330
                in vec2 position;
                in vec3 color;
                out vec3 v_color;
                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                }
            ",
            fragment: "
                #version 330
                in vec3 v_color;
                out vec4 f_color;
                void main() { f_color = vec4(v_color, 1.0); }
            ",
        },
        300 es => {
            vertex: "
                #version 300 es
                in vec2 position;
                in vec3 color;
                out vec3 v_color;
                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                }
            ",
            fragment: "
                #version 300 es
                precision mediump float;
                in vec3 v_color;
                out vec4 f_color;
                void main() { f_color = vec4(v_color, 1.0); }
            ",
        },
    )
    .unwrap()
}

/// The commands of the thirds, without indices.
fn commands(display: &Display) -> Option<DrawCommandsNoIndicesBuffer> {
    let commands = match DrawCommandsNoIndicesBuffer::empty(display, COMMANDS) {
        Ok(commands) => commands,
        Err(safegl::buffer::BufferCreationError::BufferTypeNotSupported) => return None,
        Err(e) => panic!("{e:?}"),
    };
    let list: Vec<DrawCommandNoIndices> = (0..COMMANDS)
        .map(|i| DrawCommandNoIndices {
            count: 6,
            instance_count: 1,
            first_index: 6 * i as u32,
            base_instance: 0,
        })
        .collect();
    commands.write(&list);
    Some(commands)
}

/// The buffer of the number of commands to draw, which compute shaders can write too.
fn count(display: &Display, count: u32) -> Buffer<u32> {
    Buffer::new(
        display,
        &count,
        BufferType::ShaderStorageBuffer,
        BufferMode::Default,
    )
    .unwrap()
}

/// Draws `indices` on a cleared target, and returns which thirds were drawn, or `None` if the
/// backend can't read the number of commands from a buffer.
fn drawn_thirds(
    display: &Display,
    vertices: &VertexBuffer<Vertex>,
    indices: IndicesSource<'_>,
) -> Option<[bool; COMMANDS]> {
    let program = program(display);
    let texture = Texture2d::empty(display, 96, 8).unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    let result = texture.as_surface().draw(
        vertices,
        indices,
        &program,
        &uniform! {},
        &Default::default(),
    );
    match result {
        Ok(()) => (),
        Err(DrawError::DrawCountNotSupported) => {
            // only desktop OpenGL 4.6 and `GL_ARB_indirect_parameters` have it
            assert!(
                support::is_gles(display)
                    || !display
                        .get_context()
                        .get_extensions()
                        .gl_arb_indirect_parameters
            );
            return None;
        }
        Err(e) => panic!("{e:?}"),
    }

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    let row = &data[4];
    Some(std::array::from_fn(|i| {
        let pixel = row[16 + 32 * i];
        let expected = COLORS[i].map(|c| (c * 255.0) as u8);
        if pixel == (expected[0], expected[1], expected[2], 255) {
            true
        } else {
            assert_eq!(pixel, (0, 0, 0, 0), "third {i}");
            false
        }
    }))
}

#[test]
fn the_number_of_commands_comes_from_the_buffer() {
    let display = support::build_display();
    let vertices = vertices(&display);
    let commands = match commands(&display) {
        Some(commands) => commands,
        None => return,
    };

    for drawn in 0..=COMMANDS as u32 {
        let count = count(&display, drawn);
        let indices = commands.with_draw_count(
            PrimitiveType::TrianglesList,
            count.as_slice(),
            COMMANDS,
        );
        let thirds = match drawn_thirds(&display, &vertices, indices) {
            Some(thirds) => thirds,
            None => return,
        };
        assert_eq!(
            thirds,
            std::array::from_fn(|i| (i as u32) < drawn),
            "{drawn} commands"
        );
    }

    display.assert_no_error(None);
}

#[test]
fn the_maximum_limits_the_number_of_commands() {
    let display = support::build_display();
    let vertices = vertices(&display);
    let commands = match commands(&display) {
        Some(commands) => commands,
        None => return,
    };
    let count = count(&display, COMMANDS as u32);

    // the maximum is at most the number of commands of the buffer
    for (max, expected) in [(1, [true, false, false]), (10, [true, true, true])] {
        let indices = commands.with_draw_count(PrimitiveType::TrianglesList, count.as_slice(), max);
        match drawn_thirds(&display, &vertices, indices) {
            Some(thirds) => assert_eq!(thirds, expected, "maximum of {max}"),
            None => return,
        }
    }

    display.assert_no_error(None);
}

#[test]
fn indexed_commands() {
    let display = support::build_display();
    let vertices = vertices(&display);
    let indices: Vec<u16> = (0..6 * COMMANDS as u16).collect();
    let indices = IndexBuffer::new(&display, PrimitiveType::TrianglesList, &indices).unwrap();
    let commands = match DrawCommandsIndicesBuffer::empty(&display, COMMANDS) {
        Ok(commands) => commands,
        Err(safegl::buffer::BufferCreationError::BufferTypeNotSupported) => return,
        Err(e) => panic!("{e:?}"),
    };
    let list: Vec<DrawCommandIndices> = (0..COMMANDS)
        .map(|i| DrawCommandIndices {
            count: 6,
            instance_count: 1,
            first_index: 6 * i as u32,
            base_vertex: 0,
            base_instance: 0,
        })
        .collect();
    commands.write(&list);
    let count = count(&display, 2);

    let source = commands.with_index_buffer_and_draw_count(&indices, count.as_slice(), COMMANDS);
    if let Some(thirds) = drawn_thirds(&display, &vertices, source) {
        assert_eq!(thirds, [true, true, false]);
    }

    display.assert_no_error(None);
}

#[test]
fn a_compute_shader_writes_the_number_of_commands() {
    let display = support::build_display();
    // a driver below OpenGL 4.3 can have compute shaders without GLSL 4.30
    if support::is_gles(&display)
        || !ComputeShader::is_supported(&display)
        || !support::glsl_supported(&display, 430, 310)
    {
        return;
    }
    let vertices = vertices(&display);
    let commands = match commands(&display) {
        Some(commands) => commands,
        None => return,
    };
    let count = count(&display, COMMANDS as u32);

    // the draw reads the number after the compute shader writes it
    let shader = ComputeShader::from_source(
        &display,
        "
            #version 430
            layout(local_size_x = 1) in;
            layout(std430) buffer Count { uint count; };
            void main() { count = 1u; }
        ",
    )
    .unwrap();
    shader.execute(uniform! { Count: &count }, 1, 1, 1).unwrap();

    let indices =
        commands.with_draw_count(PrimitiveType::TrianglesList, count.as_slice(), COMMANDS);
    if let Some(thirds) = drawn_thirds(&display, &vertices, indices) {
        assert_eq!(thirds, [true, false, false]);
    }

    display.assert_no_error(None);
}
