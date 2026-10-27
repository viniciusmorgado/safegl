#[macro_use]
extern crate safegl;
mod support;

use safegl::index::PrimitiveType;
use safegl::{Display, Surface};
use glutin::surface::WindowSurface;
use support::{ApplicationContext, State};

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

struct Application {
    pub vertex_buffer: safegl::VertexBuffer<Vertex>,
    pub index_buffer: safegl::IndexBuffer<u16>,
    pub program: safegl::Program,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "SafeGL screenshot example";

    fn new(display: &Display<WindowSurface>) -> Self {
        let vertex_buffer = {
            safegl::VertexBuffer::new(
                display,
                &[
                    Vertex {
                        position: [-0.5, -0.5],
                        color: [0.0, 1.0, 0.0],
                    },
                    Vertex {
                        position: [0.0, 0.5],
                        color: [0.0, 0.0, 1.0],
                    },
                    Vertex {
                        position: [0.5, -0.5],
                        color: [1.0, 0.0, 0.0],
                    },
                ],
            )
            .unwrap()
        };

        // building the index buffer
        let index_buffer =
            safegl::IndexBuffer::new(display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();

        // compiling shaders and linking them together
        let program = program!(display,
            330 => {
                vertex: "
                    #version 330

                    uniform lowp mat4 matrix;

                    in lowp vec2 position;
                    in lowp vec3 color;

                    out lowp vec3 vColor;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0) * matrix;
                        vColor = color;
                    }
                ",

                fragment: "
                    #version 330

                    out vec4 f_color;
                    in lowp vec3 vColor;

                    void main() {
                        f_color = vec4(vColor, 1.0);
                    }
                "
            },
            100 => {
                vertex: "
                    #version 100

                    uniform lowp mat4 matrix;

                    attribute lowp vec2 position;
                    attribute lowp vec3 color;

                    varying lowp vec3 vColor;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0) * matrix;
                        vColor = color;
                    }
                ",

                fragment: "
                    #version 100
                    varying lowp vec3 vColor;

                    void main() {
                        gl_FragColor = vec4(vColor, 1.0);
                    }
                "
            },
        )
        .unwrap();

        Self {
            vertex_buffer,
            index_buffer,
            program,
        }
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let mut frame = display.draw();
        // For this example a simple identity matrix suffices
        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32]
            ]
        };

        // Now we can draw the triangle
        frame.clear_color(0.0, 0.0, 0.0, 0.0);
        frame
            .draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
        frame.finish().unwrap();

        // Now we can read the front buffer and save it
        let image: safegl::texture::RawImage2d<'_, u8> = display.read_front_buffer().unwrap();
        let image =
            image::ImageBuffer::from_raw(image.width, image.height, image.data.into_owned())
                .unwrap();
        let image = image::DynamicImage::ImageRgba8(image).flipv();
        image.save("safegl-example-screenshot.png").unwrap();
    }
}

fn main() {
    State::<Application>::run_once(true);
}
