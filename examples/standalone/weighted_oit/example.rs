use crate::support::{self, ApplicationContext, State};
use glutin::surface::WindowSurface;
use nalgebra::{Matrix4, Point3, Vector3};
use safegl::framebuffer::{
    DepthRenderBuffer, InvalidateAttachments, MultiOutputFrameBuffer, SimpleFrameBuffer,
};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::ProgramCreationInput;
use safegl::texture::{DepthFormat, MipmapsOption, Texture2d, UncompressedFloatFormat};
use safegl::vertex::EmptyVertexAttributes;
use safegl::winit::event::{ElementState, KeyEvent, WindowEvent};
use safegl::winit::keyboard::{KeyCode, PhysicalKey};
use safegl::{
    Blend, BlendingFunction, Display, LinearBlendingFactor, OutputState, Program, Surface,
};
use std::time::Instant;

#[derive(Copy, Clone)]
struct PaneVertex {
    position: [f32; 3],
    color: [f32; 4],
}
implement_vertex!(PaneVertex, position, color);

#[derive(Copy, Clone)]
struct GroundVertex {
    position: [f32; 3],
    normal: [f32; 3],
}
implement_vertex!(GroundVertex, position, normal);

/// The render targets, recreated when the window is resized.
struct Targets {
    opaque: Texture2d,
    depth: DepthRenderBuffer,
    accumulation: Texture2d,
    revealage: Texture2d,
}

impl Targets {
    fn new(display: &Display<WindowSurface>, (width, height): (u32, u32)) -> Targets {
        let texture = |format| {
            Texture2d::empty_with_format(display, format, MipmapsOption::NoMipmap, width, height)
                .unwrap()
        };
        Targets {
            opaque: texture(UncompressedFloatFormat::F16F16F16F16),
            depth: DepthRenderBuffer::new(display, DepthFormat::F32, width, height).unwrap(),
            accumulation: texture(UncompressedFloatFormat::F16F16F16F16),
            revealage: texture(UncompressedFloatFormat::F16),
        }
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.opaque.width(), self.opaque.height())
    }
}

/// Six vertical panes through the vertical axis, 30 degrees apart, each with its own tint.
fn panes() -> Vec<PaneVertex> {
    const TINTS: [[f32; 4]; 6] = [
        [0.95, 0.25, 0.20, 0.45],
        [0.95, 0.75, 0.15, 0.40],
        [0.25, 0.85, 0.30, 0.40],
        [0.15, 0.65, 0.95, 0.45],
        [0.45, 0.25, 0.95, 0.40],
        [0.95, 0.30, 0.75, 0.35],
    ];
    let mut vertices = Vec::new();
    for (index, color) in TINTS.into_iter().enumerate() {
        let angle = index as f32 * std::f32::consts::PI / 6.0;
        let (x, z) = (angle.cos() * 1.3, angle.sin() * 1.3);
        let corner = |side: f32, y: f32| PaneVertex {
            position: [x * side, y, z * side],
            color,
        };
        let quad = [
            corner(-1.0, 0.05),
            corner(1.0, 0.05),
            corner(1.0, 1.15),
            corner(-1.0, 0.05),
            corner(1.0, 1.15),
            corner(-1.0, 1.15),
        ];
        vertices.extend(quad);
    }
    vertices
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
            locations: Default::default(),
        },
    )
    .unwrap()
}

/// What the last pass shows.
#[derive(Copy, Clone, PartialEq)]
enum View {
    Composite,
    Revealage,
}

struct Application {
    teapot: safegl::vertex::VertexBufferAny,
    ground: safegl::VertexBuffer<GroundVertex>,
    panes: safegl::VertexBuffer<PaneVertex>,
    opaque_program: Program,
    oit_program: Program,
    alpha_program: Program,
    composite_program: Program,
    targets: Option<Targets>,
    order_independent: bool,
    view: View,
    mask_revealage: bool,
    camera_angle: f32,
    camera_paused: bool,
    last_update: Instant,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "SafeGL weighted_oit example";

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
        let panes = safegl::VertexBuffer::new(display, &panes()).unwrap();

        let opaque_program = program(
            display,
            "
                #version 330 core
                uniform mat4 model;
                uniform mat4 view_projection;
                in vec3 position;
                in vec3 normal;
                out vec3 v_normal;
                void main() {
                    v_normal = mat3(model) * normal;
                    gl_Position = view_projection * model * vec4(position, 1.0);
                }
            ",
            "
                #version 330 core
                uniform vec3 albedo;
                in vec3 v_normal;
                out vec4 color;
                const vec3 SUN = normalize(vec3(0.3, 1.0, 0.5));
                void main() {
                    float light = 0.25 + 0.75 * max(dot(normalize(v_normal), SUN), 0.0);
                    color = vec4(albedo * light, 1.0);
                }
            ",
        );

        let panes_vertex_shader = "
            #version 330 core
            uniform mat4 view_projection;
            in vec3 position;
            in vec4 color;
            out vec4 v_color;
            void main() {
                v_color = color;
                gl_Position = view_projection * vec4(position, 1.0);
            }
        ";

        // Writes each pane to both outputs, which the draw parameters blend differently.
        let oit_program = program(
            display,
            panes_vertex_shader,
            "
                #version 330 core
                in vec4 v_color;
                layout(location = 0) out vec4 accumulation;
                layout(location = 1) out float revealage;
                void main() {
                    vec4 premultiplied = vec4(v_color.rgb * v_color.a, v_color.a);
                    // Equation (10) of the paper: surfaces close to the camera, and opaque
                    // ones, weigh more in the average.
                    float z = gl_FragCoord.z;
                    float weight = clamp(
                        pow(min(1.0, premultiplied.a * 10.0) + 0.01, 3.0) * 1e8
                            * pow(1.0 - z * 0.9, 3.0),
                        1e-2, 3e3);
                    accumulation = premultiplied * weight;
                    revealage = premultiplied.a;
                }
            ",
        );

        let alpha_program = program(
            display,
            panes_vertex_shader,
            "
                #version 330 core
                in vec4 v_color;
                out vec4 color;
                void main() {
                    color = v_color;
                }
            ",
        );

        let composite_program = program(
            display,
            "
                #version 330 core
                void main() {
                    vec2 corner = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
                    gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
                }
            ",
            "
                #version 330 core
                uniform sampler2D opaque;
                uniform sampler2D accumulation;
                uniform sampler2D revealage;
                // 0: composite, 1: transparent surfaces already in `opaque`, 2: revealage
                uniform int view;
                out vec4 color;
                void main() {
                    ivec2 pixel = ivec2(gl_FragCoord.xy);
                    vec3 background = texelFetch(opaque, pixel, 0).rgb;
                    float reveal = texelFetch(revealage, pixel, 0).r;
                    if (view == 1) {
                        color = vec4(background, 1.0);
                    } else if (view == 2) {
                        color = vec4(vec3(reveal), 1.0);
                    } else {
                        vec4 sum = texelFetch(accumulation, pixel, 0);
                        // the weighted average of the colors of the transparent surfaces
                        vec3 average = sum.rgb / max(sum.a, 1e-5);
                        color = vec4(mix(average, background, reveal), 1.0);
                    }
                }
            ",
        );

        let flag = |name: &str| std::env::args().skip(1).any(|arg| arg == name);

        Application {
            teapot,
            ground,
            panes,
            opaque_program,
            oit_program,
            alpha_program,
            composite_program,
            targets: None,
            order_independent: !flag("unsorted"),
            view: if flag("revealage") {
                View::Revealage
            } else {
                View::Composite
            },
            mask_revealage: flag("masked"),
            camera_angle: 0.3,
            camera_paused: false,
            last_update: Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        if !self.camera_paused {
            self.camera_angle += (now - self.last_update).as_secs_f32() * 0.2;
        }
        self.last_update = now;
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let dimensions = display.get_framebuffer_dimensions();
        if dimensions.0 == 0 || dimensions.1 == 0 {
            return;
        }
        if self.targets.as_ref().map(Targets::dimensions) != Some(dimensions) {
            self.targets = Some(Targets::new(display, dimensions));
        }
        let targets = self.targets.as_ref().unwrap();

        let eye = Point3::new(
            self.camera_angle.cos() * 3.4,
            1.6,
            self.camera_angle.sin() * 3.4,
        );
        let view = Matrix4::look_at_rh(&eye, &Point3::new(0.0, 0.45, 0.0), &Vector3::y());
        let projection =
            Matrix4::new_perspective(dimensions.0 as f32 / dimensions.1 as f32, 0.9, 0.1, 30.0);
        let view_projection: [[f32; 4]; 4] = (projection * view).into();

        // 1. The opaque scene, with depth.
        let mut opaque =
            SimpleFrameBuffer::with_depth_buffer(display, &targets.opaque, &targets.depth).unwrap();
        opaque.clear_color_and_depth((0.45, 0.55, 0.7, 1.0), 1.0);
        let depth_write = safegl::DrawParameters {
            depth: safegl::Depth {
                test: safegl::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let teapot_model: [[f32; 4]; 4] =
            (Matrix4::new_translation(&Vector3::new(0.0, 0.399, 0.0)) * Matrix4::new_scaling(0.01))
                .into();
        let identity: [[f32; 4]; 4] = Matrix4::identity().into();
        opaque
            .draw(
                &self.teapot,
                NoIndices(PrimitiveType::TrianglesList),
                &self.opaque_program,
                &uniform! {
                    model: teapot_model,
                    view_projection: view_projection,
                    albedo: [0.85f32, 0.82, 0.78],
                },
                &depth_write,
            )
            .unwrap();
        opaque
            .draw(
                &self.ground,
                NoIndices(PrimitiveType::TriangleStrip),
                &self.opaque_program,
                &uniform! {
                    model: identity,
                    view_projection: view_projection,
                    albedo: [0.35f32, 0.4, 0.33],
                },
                &depth_write,
            )
            .unwrap();

        // Transparent surfaces are tested against the opaque depth, but don't write depth, so
        // that they don't hide each other.
        let depth_test = safegl::Depth {
            test: safegl::DepthTest::IfLess,
            write: false,
            ..Default::default()
        };

        if self.order_independent {
            // 2. The transparent surfaces, in any order, into the two OIT targets.
            targets
                .accumulation
                .main_level()
                .clear((0.0f32, 0.0f32, 0.0f32, 0.0f32))
                .unwrap();
            targets.revealage.main_level().clear(1.0f32).unwrap();

            let add = BlendingFunction::Addition {
                source: LinearBlendingFactor::One,
                destination: LinearBlendingFactor::One,
            };
            let multiply = BlendingFunction::Addition {
                source: LinearBlendingFactor::Zero,
                destination: LinearBlendingFactor::OneMinusSourceColor,
            };
            let outputs = [
                OutputState {
                    blend: Blend {
                        color: add,
                        alpha: add,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                OutputState {
                    blend: Blend {
                        color: multiply,
                        alpha: multiply,
                        ..Default::default()
                    },
                    color_mask: if self.mask_revealage {
                        (false, false, false, false)
                    } else {
                        (true, true, true, true)
                    },
                },
            ];
            let mut transparent = MultiOutputFrameBuffer::with_depth_buffer(
                display,
                [
                    ("accumulation", &targets.accumulation),
                    ("revealage", &targets.revealage),
                ],
                &targets.depth,
            )
            .unwrap();
            transparent
                .draw(
                    &self.panes,
                    NoIndices(PrimitiveType::TrianglesList),
                    &self.oit_program,
                    &uniform! { view_projection: view_projection },
                    &safegl::DrawParameters {
                        depth: depth_test,
                        outputs: &outputs,
                        ..Default::default()
                    },
                )
                .unwrap();
        } else {
            // 2. The transparent surfaces blended over the opaque scene, in the order of the
            // vertex buffer instead of from back to front.
            opaque
                .draw(
                    &self.panes,
                    NoIndices(PrimitiveType::TrianglesList),
                    &self.alpha_program,
                    &uniform! { view_projection: view_projection },
                    &safegl::DrawParameters {
                        depth: depth_test,
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .unwrap();
        }

        // Nothing reads the depth buffer after this point: the GPU can drop it.
        opaque.invalidate(InvalidateAttachments::DEPTH_STENCIL);

        // 3. Composite the transparent surfaces over the opaque scene.
        // plain alpha blending leaves the revealage untouched, so there's nothing to show
        let view = match (self.order_independent, self.view) {
            (false, _) => 1,
            (true, View::Revealage) => 2,
            (true, View::Composite) => 0,
        };
        let mut frame = display.draw();
        frame
            .draw(
                EmptyVertexAttributes { len: 3 },
                NoIndices(PrimitiveType::TrianglesList),
                &self.composite_program,
                &uniform! {
                    opaque: &targets.opaque,
                    accumulation: &targets.accumulation,
                    revealage: &targets.revealage,
                    view: view,
                },
                &Default::default(),
            )
            .unwrap();
        frame.invalidate(InvalidateAttachments::DEPTH_STENCIL);
        frame.finish().unwrap();

        // The intermediate targets are fully redrawn next frame.
        opaque.invalidate(InvalidateAttachments::COLOR);
        MultiOutputFrameBuffer::new(
            display,
            [
                ("accumulation", &targets.accumulation),
                ("revealage", &targets.revealage),
            ],
        )
        .unwrap()
        .invalidate(InvalidateAttachments::COLOR);

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
            KeyCode::KeyO => self.order_independent = !self.order_independent,
            KeyCode::KeyR => {
                self.view = match self.view {
                    View::Composite => View::Revealage,
                    View::Revealage => View::Composite,
                }
            }
            KeyCode::KeyM => self.mask_revealage = !self.mask_revealage,
            KeyCode::Space => self.camera_paused = !self.camera_paused,
            _ => return,
        }
        window.set_title(&format!(
            "{} - {}{}{}",
            Self::WINDOW_TITLE,
            if self.order_independent {
                "order-independent transparency"
            } else {
                "unsorted alpha blending"
            },
            if self.view == View::Revealage {
                ", revealage"
            } else {
                ""
            },
            if self.mask_revealage {
                ", revealage masked"
            } else {
                ""
            },
        ));
    }
}

pub fn main() {
    State::<Application>::run_loop();
}
