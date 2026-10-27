//! What `outputs_srgb` changes.
//!
//! Both bands are the same gradient of linear values from black to white. The top band is drawn
//! by a program with `outputs_srgb: false`: SafeGL enables `GL_FRAMEBUFFER_SRGB`, so OpenGL
//! encodes the linear values to sRGB when writing them to the sRGB window, and the gradient is
//! linear in light, which looks bright, with the dark tones squeezed on the left. The bottom band
//! is drawn by a program with `outputs_srgb: true`: the values are written as they are, so they
//! are taken as sRGB values, and the gradient is linear in perceived brightness.
//!
//! Shaders that compute lighting in linear space use `outputs_srgb: false`; shaders that output
//! colors that are already sRGB, like the colors of an image or of a UI, use `true`. Programs
//! built with `Program::from_source` and the `program!` macro use `true`.

#[macro_use]
extern crate safegl;
mod support;

use glutin::surface::WindowSurface;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::{Display, Surface};
use support::{ApplicationContext, State};

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

fn create_program(display: &Display<WindowSurface>, outputs_srgb: bool) -> safegl::Program {
    let source = safegl::program::ProgramCreationInput::SourceCode {
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        geometry_shader: None,
        outputs_srgb,
        uses_point_size: false,
        locations: Default::default(),

        vertex_shader: "
            #version 330

            uniform vec2 offset;
            in vec2 position;
            in vec3 color;
            out vec3 v_color;

            void main() {
                gl_Position = vec4(position * vec2(1.9, 0.85) + offset, 0.0, 1.0);
                v_color = color;
            }
        ",
        fragment_shader: "
            #version 330

            in vec3 v_color;
            out vec4 f_color;

            void main() {
                f_color = vec4(v_color, 1.0);
            }
        ",
        transform_feedback_varyings: None,
    };

    safegl::Program::new(display, source).unwrap()
}

struct Application {
    vertex_buffer: safegl::VertexBuffer<Vertex>,
    linear_program: safegl::Program,
    srgb_program: safegl::Program,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "SafeGL outputs_srgb example";

    fn new(display: &Display<WindowSurface>) -> Self {
        // a strip from x = 0 to 1, whose color goes from black to white
        const RESOLUTION: usize = 100;
        let vertices: Vec<Vertex> = (0..RESOLUTION)
            .flat_map(|i| {
                let x = i as f32 / (RESOLUTION - 1) as f32;
                [0.0, 1.0].map(|y| Vertex {
                    position: [x, y],
                    color: [x, x, x],
                })
            })
            .collect();

        Self {
            vertex_buffer: safegl::VertexBuffer::new(display, &vertices).unwrap(),
            linear_program: create_program(display, false),
            srgb_program: create_program(display, true),
        }
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let mut frame = display.draw();
        // a green background, to make the gradients stand out
        frame.clear_color(0.1, 0.3, 0.1, 1.0);

        for (program, offset) in [
            (&self.linear_program, [-0.95, 0.05f32]),
            (&self.srgb_program, [-0.95, -0.9f32]),
        ] {
            frame
                .draw(
                    &self.vertex_buffer,
                    NoIndices(PrimitiveType::TriangleStrip),
                    program,
                    &uniform! { offset: offset },
                    &Default::default(),
                )
                .unwrap();
        }

        frame.finish().unwrap();
    }
}

fn main() {
    State::<Application>::run_loop();
}
