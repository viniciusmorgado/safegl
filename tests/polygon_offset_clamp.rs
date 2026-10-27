#![cfg(safegl_gl46)]

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::PolygonOffset;
use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use safegl::index::PrimitiveType;
use safegl::texture::DepthFormat;
use safegl::{DrawError, Surface, Texture2d};

mod support;

const SIZE: u32 = 16;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

/// Draws a quad covering the target at window depth `0.5 + depth_ndc / 2`, in `color`.
struct Pipeline {
    vertices: safegl::VertexBuffer<Vertex>,
    indices: safegl::IndexBuffer<u8>,
    program: safegl::Program,
}

impl Pipeline {
    fn new(display: &safegl::Display<safegl::glutin::surface::WindowSurface>) -> Pipeline {
        let vertices = safegl::VertexBuffer::new(
            display,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap();
        let indices =
            safegl::IndexBuffer::new(display, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3])
                .unwrap();
        let program = program!(display,
            330 => {
                vertex: "
                    #version 330 core
                    uniform float depth_ndc;
                    in vec2 position;
                    void main() { gl_Position = vec4(position, depth_ndc, 1.0); }
                ",
                fragment: "
                    #version 330 core
                    uniform vec4 color;
                    out vec4 f_color;
                    void main() { f_color = color; }
                ",
            },
            300 es => {
                vertex: "
                    #version 300 es
                    uniform float depth_ndc;
                    in vec2 position;
                    void main() { gl_Position = vec4(position, depth_ndc, 1.0); }
                ",
                fragment: "
                    #version 300 es
                    precision mediump float;
                    uniform vec4 color;
                    out vec4 f_color;
                    void main() { f_color = color; }
                ",
            },
        )
        .unwrap();

        Pipeline {
            vertices,
            indices,
            program,
        }
    }
}

/// Draws a green quad at depth 0.5 with a depth offset of about 0.006, clamped to `clamp`,
/// then a red quad at depth 0.503 that only passes the depth test where the green quad is
/// further. Returns the color in the middle of the target.
fn draw_offset_quad(clamp: f32) -> Result<(u8, u8, u8, u8), DrawError> {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);

    let color = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    // A fixed-point depth buffer, whose smallest step is 2^-24: an offset of `units` moves the
    // depth by `units / 2^24`.
    let depth = DepthRenderBuffer::new(&display, DepthFormat::I24, SIZE, SIZE).unwrap();
    let mut target = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);

    target.draw(
        &pipeline.vertices,
        &pipeline.indices,
        &pipeline.program,
        &uniform! { depth_ndc: 0.0f32, color: (0.0f32, 1.0f32, 0.0f32, 1.0f32) },
        &safegl::DrawParameters {
            depth: safegl::Depth {
                test: safegl::DepthTest::Overwrite,
                write: true,
                ..Default::default()
            },
            polygon_offset: PolygonOffset {
                units: 100_000.0,
                fill: true,
                ..Default::default()
            },
            polygon_offset_clamp: clamp,
            ..Default::default()
        },
    )?;

    target
        .draw(
            &pipeline.vertices,
            &pipeline.indices,
            &pipeline.program,
            &uniform! { depth_ndc: 0.006f32, color: (1.0f32, 0.0f32, 0.0f32, 1.0f32) },
            &safegl::DrawParameters {
                depth: safegl::Depth {
                    test: safegl::DepthTest::IfLess,
                    write: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    drop(target);

    display.assert_no_error(None);
    let data: Vec<Vec<(u8, u8, u8, u8)>> = color.read().unwrap();
    Ok(data[SIZE as usize / 2][SIZE as usize / 2])
}

#[test]
fn without_clamp_the_offset_pushes_the_quad_behind() {
    // 0.5 + 0.006 is behind 0.503
    assert_eq!(draw_offset_quad(0.0).unwrap(), (255, 0, 0, 255));
}

#[test]
fn clamped_offset_stays_in_front() {
    // 0.5 + 0.001 is in front of 0.503
    match draw_offset_quad(0.001) {
        Err(DrawError::PolygonOffsetClampNotSupported) => (),
        result => assert_eq!(result.unwrap(), (0, 255, 0, 255)),
    }
}

#[test]
fn a_clamp_larger_than_the_offset_changes_nothing() {
    match draw_offset_quad(0.5) {
        Err(DrawError::PolygonOffsetClampNotSupported) => (),
        result => assert_eq!(result.unwrap(), (255, 0, 0, 255)),
    }
}
