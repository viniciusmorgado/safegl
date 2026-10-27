//! Vertex buffers bound at different offsets and in different orders, which the VAOs with
//! separate vertex bindings (OpenGL 4.3, OpenGL ES 3.1) bind again when they are reused.

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::{Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

#[derive(Copy, Clone)]
struct Position {
    position: [f32; 2],
}
implement_vertex!(Position, position);

#[derive(Copy, Clone)]
struct Color {
    color: [f32; 3],
}
implement_vertex!(Color, color);

/// A triangle that covers the whole viewport.
const TRIANGLE: [[f32; 2]; 3] = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];

/// The number of triangles of the buffers.
const COUNT: usize = 20;

/// The red of the triangle `i`, which is different for each triangle once read back.
fn red(i: usize) -> f32 {
    (i as f32 * 12.0 + 8.0) / 255.0
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

/// The color of the center of `texture`.
fn center(texture: &safegl::Texture2d) -> (u8, u8, u8, u8) {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[data.len() / 2][data[0].len() / 2]
}

#[test]
fn drawing_from_many_offsets_of_a_buffer() {
    let display = support::build_display();
    let program = program(&display);

    // triangle `i` is at vertices `3 * i` to `3 * i + 2`, with its own color
    let vertices: Vec<Vertex> = (0..COUNT)
        .flat_map(|i| {
            TRIANGLE.map(|position| Vertex {
                position,
                color: [red(i), 0.0, 1.0],
            })
        })
        .collect();
    let buffer = VertexBuffer::new(&display, &vertices).unwrap();
    let texture = support::build_renderable_texture(&display);

    // forward, then backward, so that a VAO is reused at offsets above and below its own
    for i in (0..COUNT).chain((0..COUNT).rev()) {
        let slice = buffer.slice(3 * i..3 * i + 3).unwrap();
        texture
            .as_surface()
            .draw(
                slice,
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &Default::default(),
            )
            .unwrap();
        assert_eq!(center(&texture), ((red(i) * 255.0).round() as u8, 0, 255, 255), "triangle {i}");
    }

    display.assert_no_error(None);
}

#[test]
fn vertex_buffers_in_another_order() {
    let display = support::build_display();
    let program = program(&display);

    let positions =
        VertexBuffer::new(&display, &TRIANGLE.map(|position| Position { position })).unwrap();
    let colors: Vec<Color> = (0..COUNT)
        .flat_map(|i| [Color { color: [red(i), 1.0, 0.0] }; 3])
        .collect();
    let colors = VertexBuffer::new(&display, &colors).unwrap();
    let texture = support::build_renderable_texture(&display);

    for i in 0..COUNT {
        let slice = colors.slice(3 * i..3 * i + 3).unwrap();
        let expected = ((red(i) * 255.0).round() as u8, 255, 0, 255);
        let parameters = Default::default();

        // the binding points follow the order of the buffers
        texture
            .as_surface()
            .draw(
                (&positions, slice),
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &parameters,
            )
            .unwrap();
        assert_eq!(center(&texture), expected, "triangle {i}");

        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        texture
            .as_surface()
            .draw(
                (colors.slice(3 * i..3 * i + 3).unwrap(), &positions),
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &parameters,
            )
            .unwrap();
        assert_eq!(center(&texture), expected, "triangle {i}, in another order");
    }

    display.assert_no_error(None);
}

#[test]
fn per_instance_slices() {
    let display = support::build_display();
    let program = program(&display);

    let positions =
        VertexBuffer::new(&display, &TRIANGLE.map(|position| Position { position })).unwrap();
    let colors: Vec<Color> = (0..COUNT)
        .map(|i| Color {
            color: [red(i), 0.0, 1.0],
        })
        .collect();
    let colors = VertexBuffer::new(&display, &colors).unwrap();
    let texture = support::build_renderable_texture(&display);

    // the instance of each draw takes its color at another offset of the buffer
    for i in (0..COUNT).rev() {
        let slice = colors.slice(i..i + 1).unwrap();
        let instance = slice.per_instance();
        texture
            .as_surface()
            .draw(
                (&positions, instance),
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &Default::default(),
            )
            .unwrap();
        assert_eq!(center(&texture), ((red(i) * 255.0).round() as u8, 0, 255, 255), "instance {i}");
    }

    display.assert_no_error(None);
}
