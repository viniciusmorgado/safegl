use crate::support::{self, ApplicationContext, State};
use glutin::surface::WindowSurface;
use nalgebra::{Matrix4, Point3, Vector3};
use safegl::draw_parameters::PolygonOffset;
use safegl::framebuffer::SimpleFrameBuffer;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::ProgramCreationInput;
use safegl::texture::{DepthFormat, DepthTexture2d, MipmapsOption};
use safegl::uniforms::{
    DepthTextureComparison, MagnifySamplerFilter, MinifySamplerFilter, Sampler,
};
use safegl::winit::event::{ElementState, KeyEvent, WindowEvent};
use safegl::winit::keyboard::{KeyCode, PhysicalKey};
use safegl::{Display, Program, Surface};
use std::time::Instant;

const SHADOW_MAP_SIZE: u32 = 1024;

/// The teapot model is about 1.8 units wide and 0.9 high once scaled.
const TEAPOT_SCALE: f32 = 0.01;
/// Lifts the teapot so that its base touches the ground.
const TEAPOT_LIFT: f32 = 0.399;

#[derive(Copy, Clone)]
struct GroundVertex {
    position: [f32; 3],
    normal: [f32; 3],
}
implement_vertex!(GroundVertex, position, normal);

#[derive(Copy, Clone, PartialEq)]
enum Mode {
    NoOffset,
    Constant,
    SlopeScaled,
    Clamped,
}

impl Mode {
    fn from_number(number: u32) -> Option<Mode> {
        Some(match number {
            1 => Mode::NoOffset,
            2 => Mode::Constant,
            3 => Mode::SlopeScaled,
            4 => Mode::Clamped,
            _ => return None,
        })
    }

    fn polygon_offset(self) -> PolygonOffset {
        // With a 24-bit depth buffer, one unit is 2^-24 of the depth range. The factor
        // multiplies the largest change of depth across one pixel of the shadow map.
        let (factor, units) = match self {
            Mode::NoOffset => (0.0, 0.0),
            Mode::Constant => (0.0, 2000.0),
            Mode::SlopeScaled | Mode::Clamped => (40.0, 2000.0),
        };
        PolygonOffset {
            factor,
            units,
            fill: self != Mode::NoOffset,
            ..Default::default()
        }
    }

    fn describe(self, clamp: f32) -> String {
        match self {
            Mode::NoOffset => "1: no offset".to_owned(),
            Mode::Constant => "2: constant offset".to_owned(),
            Mode::SlopeScaled => "3: slope-scaled offset".to_owned(),
            Mode::Clamped => format!("4: slope-scaled offset clamped to {clamp:.5}"),
        }
    }
}

fn program(display: &Display<WindowSurface>, vertex: &str, fragment: &str) -> Program {
    Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: vertex,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: fragment,
            transform_feedback_varyings: None,
            outputs_srgb: false,
            uses_point_size: false,
        },
    )
    .unwrap()
}

struct Application {
    teapot: safegl::vertex::VertexBufferAny,
    ground: safegl::VertexBuffer<GroundVertex>,
    shadow_map: DepthTexture2d,
    shadow_program: Program,
    scene_program: Program,
    mode: Mode,
    clamp: f32,
    sun_angle: f32,
    sun_paused: bool,
    camera_angle: f32,
    camera_paused: bool,
    last_update: Instant,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "SafeGL shadow_bias example";

    fn new(display: &Display<WindowSurface>) -> Self {
        let teapot = support::load_wavefront(display, include_bytes!("../support/teapot.obj"));

        let ground = safegl::VertexBuffer::new(
            display,
            &[[-4.0, -4.0], [4.0, -4.0], [-4.0, 4.0], [4.0, 4.0]].map(|[x, z]| GroundVertex {
                position: [x, 0.0, z],
                normal: [0.0, 1.0, 0.0],
            }),
        )
        .unwrap();

        // A fixed-point format, so that `PolygonOffset::units` has the same size everywhere.
        let shadow_map = DepthTexture2d::empty_with_format(
            display,
            DepthFormat::I24,
            MipmapsOption::NoMipmap,
            SHADOW_MAP_SIZE,
            SHADOW_MAP_SIZE,
        )
        .unwrap();

        // Both passes transform positions the same way: `model` places the teapot or the
        // ground in the world.
        let shadow_program = program(
            display,
            "
                #version 330 core
                uniform mat4 model;
                uniform mat4 light_view_projection;
                in vec3 position;
                void main() {
                    gl_Position = light_view_projection * model * vec4(position, 1.0);
                }
            ",
            "
                #version 330 core
                void main() {}
            ",
        );

        let scene_program = program(
            display,
            "
                #version 330 core
                uniform mat4 model;
                uniform mat4 view_projection;
                uniform mat4 light_view_projection;
                in vec3 position;
                in vec3 normal;
                out vec3 v_normal;
                out vec4 v_light_position;
                void main() {
                    vec4 world = model * vec4(position, 1.0);
                    v_normal = mat3(model) * normal;
                    v_light_position = light_view_projection * world;
                    gl_Position = view_projection * world;
                }
            ",
            "
                #version 330 core
                uniform sampler2DShadow shadow_map;
                uniform vec3 sun_direction;
                uniform vec3 albedo;
                in vec3 v_normal;
                in vec4 v_light_position;
                out vec4 color;
                void main() {
                    // from clip space of the sun to texture coordinates and depth, 0 to 1
                    vec3 shadow = v_light_position.xyz / v_light_position.w * 0.5 + 0.5;
                    // 1 where the pixel is no further from the sun than what the shadow map
                    // saw, filtered over the 4 nearest texels
                    float lit = texture(shadow_map, shadow);

                    vec3 n = normalize(v_normal);
                    float sun = max(dot(n, sun_direction), 0.0) * lit;
                    color = vec4(albedo * (0.15 + 0.85 * sun), 1.0);
                }
            ",
        );

        let mode = std::env::args()
            .nth(1)
            .and_then(|arg| arg.parse().ok())
            .and_then(Mode::from_number)
            .unwrap_or(Mode::NoOffset);
        let clamp = 0.004;
        println!("{}", mode.describe(clamp));

        Application {
            teapot,
            ground,
            shadow_map,
            shadow_program,
            scene_program,
            mode,
            clamp,
            sun_angle: 0.4,
            sun_paused: true,
            camera_angle: 0.4 + std::f32::consts::PI - 1.2,
            camera_paused: true,
            last_update: Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let elapsed = (now - self.last_update).as_secs_f32();
        if !self.sun_paused {
            self.sun_angle += elapsed * 0.15;
        }
        if !self.camera_paused {
            self.camera_angle += elapsed * 0.1;
        }
        self.last_update = now;
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let (width, height) = display.get_framebuffer_dimensions();
        if width == 0 || height == 0 {
            return;
        }

        let teapot_model: [[f32; 4]; 4] =
            (Matrix4::new_translation(&Vector3::new(0.0, TEAPOT_LIFT, 0.0))
                * Matrix4::new_scaling(TEAPOT_SCALE))
            .into();
        let ground_model: [[f32; 4]; 4] = Matrix4::identity().into();

        // A low sun, 12 degrees above the horizon: its light grazes the ground.
        let elevation = 12f32.to_radians();
        let sun_direction = Vector3::new(
            self.sun_angle.cos() * elevation.cos(),
            elevation.sin(),
            self.sun_angle.sin() * elevation.cos(),
        );
        let sun_view = Matrix4::look_at_rh(
            &Point3::from(sun_direction * 10.0),
            &Point3::origin(),
            &Vector3::y(),
        );
        let sun_projection = Matrix4::new_orthographic(-3.0, 3.0, -3.0, 3.0, 5.0, 15.0);
        let light_view_projection: [[f32; 4]; 4] = (sun_projection * sun_view).into();

        // The shadow pass: only depth, with the offset of the current mode.
        let mut shadow_target = SimpleFrameBuffer::depth_only(display, &self.shadow_map).unwrap();
        shadow_target.clear_depth(1.0);
        let shadow_params = safegl::DrawParameters {
            depth: safegl::Depth {
                test: safegl::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            polygon_offset: self.mode.polygon_offset(),
            polygon_offset_clamp: if self.mode == Mode::Clamped {
                self.clamp
            } else {
                0.0
            },
            ..Default::default()
        };
        shadow_target
            .draw(
                &self.teapot,
                NoIndices(PrimitiveType::TrianglesList),
                &self.shadow_program,
                &uniform! { model: teapot_model, light_view_projection: light_view_projection },
                &shadow_params,
            )
            .unwrap();
        shadow_target
            .draw(
                &self.ground,
                NoIndices(PrimitiveType::TriangleStrip),
                &self.shadow_program,
                &uniform! { model: ground_model, light_view_projection: light_view_projection },
                &shadow_params,
            )
            .unwrap();

        // The scene, seen from the camera.
        let eye = Point3::new(
            self.camera_angle.cos() * 2.6,
            0.9,
            self.camera_angle.sin() * 2.6,
        );
        let view = Matrix4::look_at_rh(&eye, &Point3::new(0.0, 0.25, 0.0), &Vector3::y());
        let projection = Matrix4::new_perspective(width as f32 / height as f32, 0.9, 0.05, 50.0);
        let view_projection: [[f32; 4]; 4] = (projection * view).into();

        let shadow_map = Sampler::new(&self.shadow_map)
            .magnify_filter(MagnifySamplerFilter::Linear)
            .minify_filter(MinifySamplerFilter::Linear)
            .depth_texture_comparison(Some(DepthTextureComparison::LessOrEqual));
        let sun_direction: [f32; 3] = sun_direction.into();
        let params = safegl::DrawParameters {
            depth: safegl::Depth {
                test: safegl::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut frame = display.draw();
        frame.clear_color_and_depth((0.55, 0.62, 0.72, 1.0), 1.0);
        frame
            .draw(
                &self.teapot,
                NoIndices(PrimitiveType::TrianglesList),
                &self.scene_program,
                &uniform! {
                    model: teapot_model,
                    view_projection: view_projection,
                    light_view_projection: light_view_projection,
                    shadow_map: shadow_map,
                    sun_direction: sun_direction,
                    albedo: [0.8f32, 0.75, 0.7],
                },
                &params,
            )
            .unwrap();
        frame
            .draw(
                &self.ground,
                NoIndices(PrimitiveType::TriangleStrip),
                &self.scene_program,
                &uniform! {
                    model: ground_model,
                    view_projection: view_projection,
                    light_view_projection: light_view_projection,
                    shadow_map: shadow_map,
                    sun_direction: sun_direction,
                    albedo: [0.45f32, 0.5, 0.4],
                },
                &params,
            )
            .unwrap();
        frame.finish().unwrap();

        display.assert_no_error(None);
    }

    fn handle_window_event(&mut self, event: &WindowEvent, window: &safegl::winit::window::Window) {
        let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(key),
                    ..
                },
            ..
        } = event
        else {
            return;
        };

        match key {
            KeyCode::Digit1 => self.mode = Mode::NoOffset,
            KeyCode::Digit2 => self.mode = Mode::Constant,
            KeyCode::Digit3 => self.mode = Mode::SlopeScaled,
            KeyCode::Digit4 => self.mode = Mode::Clamped,
            KeyCode::Equal | KeyCode::NumpadAdd => self.clamp *= 1.5,
            KeyCode::Minus | KeyCode::NumpadSubtract => self.clamp /= 1.5,
            KeyCode::KeyL => self.sun_paused = !self.sun_paused,
            KeyCode::Space => self.camera_paused = !self.camera_paused,
            _ => return,
        }
        window.set_title(&format!(
            "{} - {}",
            Self::WINDOW_TITLE,
            self.mode.describe(self.clamp)
        ));
    }
}

pub fn main() {
    State::<Application>::run_loop();
}
