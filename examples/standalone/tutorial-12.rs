#![deny(deprecated)]

#[macro_use]
extern crate safegl;
use safegl::{glutin, winit, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

type Display = safegl::Display<glutin::surface::WindowSurface>;

#[path = "../../book/tuto-07-teapot.rs"]
mod teapot;

struct State {
    positions: safegl::VertexBuffer<teapot::Vertex>,
    normals: safegl::VertexBuffer<teapot::Normal>,
    indices: safegl::IndexBuffer<u16>,
    program: safegl::Program,
    display: Display,
    window: Window,
}

impl State {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let (window, display) = safegl::backend::glutin::SimpleWindowBuilder::new()
            .with_title("SafeGL tutorial #12")
            .with_config_template_builder(
                glutin::config::ConfigTemplateBuilder::new().with_depth_size(24),
            )
            .build(event_loop);
        let positions = safegl::VertexBuffer::new(&display, &teapot::VERTICES).unwrap();
        let normals = safegl::VertexBuffer::new(&display, &teapot::NORMALS).unwrap();
        let indices = safegl::IndexBuffer::new(
            &display,
            safegl::index::PrimitiveType::TrianglesList,
            &teapot::INDICES,
        )
        .unwrap();

        let vertex_shader_src = r#"
                    #version 410 core

                    in vec3 position;
                    in vec3 normal;

                    out vec3 v_normal;

                    uniform mat4 perspective;
                    uniform mat4 view;
                    uniform mat4 model;

                    void main() {
                        mat4 modelview = view * model;
                        v_normal = transpose(inverse(mat3(modelview))) * normal;
                        gl_Position = perspective * modelview * vec4(position, 1.0);
                    }
                "#;

        let fragment_shader_src = r#"
                    #version 410 core

                    in vec3 v_normal;
                    out vec4 color;
                    uniform vec3 u_light;

                    void main() {
                        float brightness = max(dot(normalize(v_normal), normalize(u_light)), 0.0);
                        vec3 dark_color = vec3(0.6, 0.0, 0.0);
                        vec3 regular_color = vec3(1.0, 0.0, 0.0);
                        color = vec4(mix(dark_color, regular_color, brightness), 1.0);
                    }
                "#;

        let program =
            safegl::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
                .unwrap();
        Self {
            positions,
            normals,
            indices,
            program,
            display,
            window,
        }
    }

    fn draw(&self) {
        let mut target = self.display.draw();
        target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);

        let model = [
            [0.01, 0.0, 0.0, 0.0],
            [0.0, 0.01, 0.0, 0.0],
            [0.0, 0.0, 0.01, 0.0],
            [0.0, 0.0, 2.0, 1.0f32],
        ];

        let view = view_matrix(&[2.0, -1.0, 1.0], &[-2.0, 1.0, 1.0], &[0.0, 1.0, 0.0]);

        let perspective = {
            let (width, height) = target.get_dimensions();
            let aspect_ratio = height as f32 / width as f32;

            let fov: f32 = std::f32::consts::PI / 3.0;
            let zfar = 1024.0;
            let znear = 0.1;

            let f = 1.0 / (fov / 2.0).tan();

            [
                [f * aspect_ratio, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
            ]
        };

        let light = [-1.0, 0.4, 0.9f32];

        let params = safegl::DrawParameters {
            depth: safegl::Depth {
                test: safegl::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            //backface_culling: safegl::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };

        target
            .draw(
                (&self.positions, &self.normals),
                &self.indices,
                &self.program,
                &uniform! { model: model, view: view, perspective: perspective, u_light: light },
                &params,
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
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop
        .run_app(&mut App::default())
        .expect("run application");
}

fn view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
    let f = {
        let f = direction;
        let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
        let len = len.sqrt();
        [f[0] / len, f[1] / len, f[2] / len]
    };

    let s = [
        up[1] * f[2] - up[2] * f[1],
        up[2] * f[0] - up[0] * f[2],
        up[0] * f[1] - up[1] * f[0],
    ];

    let s_norm = {
        let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
        let len = len.sqrt();
        [s[0] / len, s[1] / len, s[2] / len]
    };

    let u = [
        f[1] * s_norm[2] - f[2] * s_norm[1],
        f[2] * s_norm[0] - f[0] * s_norm[2],
        f[0] * s_norm[1] - f[1] * s_norm[0],
    ];

    let p = [
        -position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
        -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
        -position[0] * f[0] - position[1] * f[1] - position[2] * f[2],
    ];

    [
        [s_norm[0], u[0], f[0], 0.0],
        [s_norm[1], u[1], f[1], 0.0],
        [s_norm[2], u[2], f[2], 0.0],
        [p[0], p[1], p[2], 1.0],
    ]
}
