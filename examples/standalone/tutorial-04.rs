#![deny(deprecated)]

#[macro_use]
extern crate safegl;
use safegl::{glutin, winit, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

type Display = safegl::Display<glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

struct State {
    vertex_buffer: safegl::VertexBuffer<Vertex>,
    indices: safegl::index::NoIndices,
    program: safegl::Program,
    start: std::time::Instant,
    display: Display,
    window: Window,
}

impl State {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let (window, display) = safegl::backend::glutin::SimpleWindowBuilder::new()
            .with_title("SafeGL tutorial #4")
            .build(event_loop);
        let shape = vec![
            Vertex {
                position: [-0.5, -0.5],
            },
            Vertex {
                position: [0.0, 0.5],
            },
            Vertex {
                position: [0.5, -0.25],
            },
        ];
        let vertex_buffer = safegl::VertexBuffer::new(&display, &shape).unwrap();
        let indices = safegl::index::NoIndices(safegl::index::PrimitiveType::TrianglesList);

        let vertex_shader_src = r#"
                #version 410 core

                in vec2 position;

                uniform mat4 matrix;

                void main() {
                    gl_Position = matrix * vec4(position, 0.0, 1.0);
                }
            "#;
        let fragment_shader_src = r#"
                #version 410 core

                out vec4 color;

                void main() {
                    color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            "#;
        let program =
            safegl::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
                .unwrap();

        let start = std::time::Instant::now();
        Self {
            vertex_buffer,
            indices,
            program,
            start,
            display,
            window,
        }
    }

    fn draw(&self) {
        let t = self.start.elapsed().as_secs_f32();
        let x = t.sin() * 0.5;

        let mut target = self.display.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);

        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [  x, 0.0, 0.0, 1.0f32],
            ]
        };

        target
            .draw(
                &self.vertex_buffer,
                &self.indices,
                &self.program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
        target.finish().unwrap();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let state = State::new(event_loop);
            state.window.request_redraw();
            self.state = Some(state);
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.state = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &self.state else { return };
        if state.window.id() != window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                state.display.resize(size.into());
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let size = state.window.inner_size();
                if size.width > 0 && size.height > 0 {
                    state.draw();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop
        .run_app(&mut App::default())
        .expect("run application");
}
