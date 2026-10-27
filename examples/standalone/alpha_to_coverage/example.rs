use crate::support::{ApplicationContext, State};
use glutin::surface::WindowSurface;
use nalgebra::{Matrix4, Point3, Vector3};
use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::ProgramCreationInput;
use safegl::texture::{
    DepthFormat, MipmapsOption, RawImage2d, SrgbTexture2d, Texture2d, Texture2dMultisample,
    UncompressedFloatFormat,
};
use safegl::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler};
use safegl::vertex::EmptyVertexAttributes;
use safegl::winit::event::{ElementState, KeyEvent, WindowEvent};
use safegl::winit::keyboard::{KeyCode, PhysicalKey};
use safegl::{BlitTarget, Display, Program, Surface};
use std::time::Instant;

/// Samples per pixel of the offscreen target.
const SAMPLES: u32 = 4;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, normal, tex_coords);

#[derive(Copy, Clone)]
struct Instance {
    instance_offset: [f32; 3],
    instance_angle: f32,
    instance_scale: f32,
}
implement_vertex!(Instance, instance_offset, instance_angle, instance_scale);

#[derive(Copy, Clone)]
struct GroundVertex {
    position: [f32; 3],
}
implement_vertex!(GroundVertex, position);

#[derive(Copy, Clone, PartialEq)]
enum Mode {
    AlphaTest,
    AlphaToCoverage,
    Sharpened,
    SampleShading,
    /// Sharpened alpha-to-coverage with the given number of samples kept by the sample mask.
    SampleMask(u32),
}

impl Mode {
    /// The mode of a number key, `kept` being the number of samples for the sample mask.
    fn from_number(number: u32, kept: u32) -> Option<Mode> {
        Some(match number {
            1 => Mode::AlphaTest,
            2 => Mode::AlphaToCoverage,
            3 => Mode::Sharpened,
            4 => Mode::SampleShading,
            5 => Mode::SampleMask(kept.clamp(1, SAMPLES)),
            _ => return None,
        })
    }

    fn describe(self) -> String {
        match self {
            Mode::AlphaTest => "1: alpha test".to_owned(),
            Mode::AlphaToCoverage => "2: alpha-to-coverage".to_owned(),
            Mode::Sharpened => "3: alpha-to-coverage, sharpened alpha".to_owned(),
            Mode::SampleShading => {
                "4: sharpened alpha-to-coverage, shading every sample".to_owned()
            }
            Mode::SampleMask(kept) => {
                format!("5: sharpened alpha-to-coverage, sample mask keeping {kept} of {SAMPLES}")
            }
        }
    }

    /// Value of the `alpha_mode` uniform of the fern shader.
    fn alpha_mode(self) -> i32 {
        match self {
            Mode::AlphaTest => 0,
            Mode::AlphaToCoverage => 1,
            _ => 2,
        }
    }
}

/// The multisampled target the scene is drawn into, and the texture it's resolved into.
struct Targets {
    color: Texture2dMultisample,
    depth: DepthRenderBuffer,
    resolved: Texture2d,
}

impl Targets {
    fn new(display: &Display<WindowSurface>, (width, height): (u32, u32)) -> Targets {
        // a float format, so that the resolve averages linear colors
        let color = Texture2dMultisample::empty_with_format(
            display,
            UncompressedFloatFormat::F16F16F16F16,
            MipmapsOption::NoMipmap,
            width,
            height,
            SAMPLES,
        )
        .unwrap();
        let depth =
            DepthRenderBuffer::new_multisample(display, DepthFormat::F32, width, height, SAMPLES)
                .unwrap();
        let resolved = Texture2d::empty_with_format(
            display,
            UncompressedFloatFormat::F16F16F16F16,
            MipmapsOption::NoMipmap,
            width,
            height,
        )
        .unwrap();
        Targets {
            color,
            depth,
            resolved,
        }
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.resolved.width(), self.resolved.height())
    }
}

/// The four fern variants of the model, which share one vertex buffer and one index buffer.
struct Ferns {
    vertices: safegl::VertexBuffer<Vertex>,
    indices: safegl::IndexBuffer<u32>,
    /// Range of `indices` used by each variant.
    variants: Vec<std::ops::Range<usize>>,
    /// Instances of each variant.
    instances: Vec<safegl::VertexBuffer<Instance>>,
    /// Leaf color in RGB, cut-out in alpha.
    texture: SrgbTexture2d,
}

impl Ferns {
    fn load(display: &Display<WindowSurface>) -> Ferns {
        let gltf =
            gltf::Gltf::from_slice(include_bytes!("../support/fern_02/fern_02_1k.gltf")).unwrap();
        let bin: &[u8] = include_bytes!("../support/fern_02/fern_02.bin");

        // Each mesh of the model is one variant of the fern. The translations of their nodes,
        // which lay the variants out side by side, are ignored: each is centred on its origin.
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut variants = Vec::new();
        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|_| Some(bin));
                let base = vertices.len() as u32;
                let positions = reader.read_positions().unwrap();
                let normals = reader.read_normals().unwrap();
                let tex_coords = reader.read_tex_coords(0).unwrap().into_f32();
                vertices.extend(positions.zip(normals).zip(tex_coords).map(
                    |((position, normal), tex_coords)| Vertex {
                        position,
                        normal,
                        tex_coords,
                    },
                ));
                let start = indices.len();
                indices.extend(reader.read_indices().unwrap().into_u32().map(|i| base + i));
                variants.push(start..indices.len());
            }
        }

        // A grid of ferns, each with its own variant, rotation and size.
        const ROWS: i32 = 9;
        let mut per_variant = vec![Vec::new(); variants.len()];
        for row in 0..ROWS {
            for column in 0..ROWS {
                let hash = ((row * 7919 + column * 104_729) % 997) as f32 / 997.0;
                per_variant[((row * 3 + column) as usize) % variants.len()].push(Instance {
                    instance_offset: [
                        (column - ROWS / 2) as f32 * 0.8 + hash * 0.3,
                        0.0,
                        (row - ROWS / 2) as f32 * 0.8 - hash * 0.3,
                    ],
                    instance_angle: hash * std::f32::consts::TAU,
                    instance_scale: 0.9 + hash * 0.5,
                });
            }
        }
        let instances = per_variant
            .iter()
            .map(|instances| safegl::VertexBuffer::new(display, instances).unwrap())
            .collect();

        // The diffuse texture of the model is a JPEG without alpha; Poly Haven ships the
        // cut-out as a separate grayscale image, which becomes the alpha channel here.
        let diffuse = image::load_from_memory(include_bytes!(
            "../support/fern_02/textures/fern_02_diff_1k.jpg"
        ))
        .unwrap()
        .to_rgb8();
        let alpha = image::load_from_memory(include_bytes!(
            "../support/fern_02/textures/fern_02_alpha_1k.png"
        ))
        .unwrap()
        .to_luma8();
        let mut rgba = Vec::with_capacity(diffuse.as_raw().len() / 3 * 4);
        for (color, alpha) in diffuse.pixels().zip(alpha.pixels()) {
            rgba.extend_from_slice(&[color[0], color[1], color[2], alpha[0]]);
        }
        // glTF puts the origin of texture coordinates at the top left of the image, which is
        // the first row in memory, so the rows aren't flipped
        let image = RawImage2d::from_raw_rgba(rgba, diffuse.dimensions());
        let texture =
            SrgbTexture2d::with_mipmaps(display, image, MipmapsOption::AutoGeneratedMipmaps)
                .unwrap();

        Ferns {
            vertices: safegl::VertexBuffer::new(display, &vertices).unwrap(),
            indices: safegl::IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices)
                .unwrap(),
            variants,
            instances,
            texture,
        }
    }
}

/// The mode given on the command line, alpha test by default.
fn initial_mode() -> Mode {
    let mut args = std::env::args().skip(1).map(|arg| arg.parse().unwrap_or(0));
    let number = args.next().unwrap_or(1);
    let mode = Mode::from_number(number, args.next().unwrap_or(1)).unwrap_or(Mode::AlphaTest);
    println!("{}", mode.describe());
    mode
}

/// Builds a program whose fragment shader outputs linear colors, which SafeGL encodes to sRGB
/// when the target is an sRGB framebuffer.
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
    ferns: Ferns,
    fern_program: Program,
    ground: safegl::VertexBuffer<GroundVertex>,
    ground_program: Program,
    present_program: Program,
    targets: Option<Targets>,
    mode: Mode,
    camera_angle: f32,
    camera_distance: f32,
    camera_paused: bool,
    last_update: Instant,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "SafeGL alpha_to_coverage example";

    fn new(display: &Display<WindowSurface>) -> Self {
        let fern_program = program(
            display,
            "
                #version 330 core

                uniform mat4 view_projection;

                in vec3 position;
                in vec3 normal;
                in vec2 tex_coords;
                in vec3 instance_offset;
                in float instance_angle;
                in float instance_scale;

                out vec3 v_normal;
                out vec2 v_tex_coords;

                void main() {
                    float c = cos(instance_angle);
                    float s = sin(instance_angle);
                    mat3 rotation = mat3(c, 0.0, -s, 0.0, 1.0, 0.0, s, 0.0, c);
                    v_normal = rotation * normal;
                    v_tex_coords = tex_coords;
                    vec3 world = rotation * position * instance_scale + instance_offset;
                    gl_Position = view_projection * vec4(world, 1.0);
                }
            ",
            "
                #version 330 core

                uniform sampler2D albedo;
                // 0: alpha test, 1: alpha of the texture, 2: sharpened alpha
                uniform int alpha_mode;

                in vec3 v_normal;
                in vec2 v_tex_coords;

                out vec4 color;

                const vec3 SUN = normalize(vec3(0.4, 1.0, 0.3));

                void main() {
                    vec4 texel = texture(albedo, v_tex_coords);
                    float alpha = texel.a;
                    if (alpha_mode == 0) {
                        if (alpha < 0.5) {
                            discard;
                        }
                        alpha = 1.0;
                    } else if (alpha_mode == 2) {
                        // Rescale the alpha around the 0.5 cut-off by how fast it changes
                        // across the screen, so that the edge spans about one pixel at every
                        // distance and mipmap level (Ben Golus, \"Anti-aliased Alpha Test\").
                        alpha = (alpha - 0.5) / max(fwidth(alpha), 0.0001) + 0.5;
                        alpha = clamp(alpha, 0.0, 1.0);
                    }

                    vec3 n = normalize(gl_FrontFacing ? v_normal : -v_normal);
                    float light = 0.3 + 0.7 * max(dot(n, SUN), 0.0);
                    color = vec4(texel.rgb * light, alpha);
                }
            ",
        );

        let ground_program = program(
            display,
            "
                #version 330 core
                uniform mat4 view_projection;
                in vec3 position;
                void main() {
                    gl_Position = view_projection * vec4(position, 1.0);
                }
            ",
            "
                #version 330 core
                out vec4 color;
                void main() {
                    color = vec4(0.09, 0.06, 0.035, 1.0);
                }
            ",
        );

        // Shows the resolved scene, with a magnified inset of the middle of the screen.
        let present_program = program(
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

                uniform sampler2D scene;
                // lower-left corners, in pixels, of the inset and of the magnified area
                uniform ivec2 inset_origin;
                uniform ivec2 source_origin;

                out vec4 color;

                const int ZOOM = 8;
                const int INSET = 256;
                const int SOURCE = INSET / ZOOM;

                bool on_border(ivec2 local, int size) {
                    return all(greaterThanEqual(local, ivec2(-1)))
                        && all(lessThanEqual(local, ivec2(size)))
                        && (any(lessThan(local, ivec2(0))) || any(greaterThanEqual(local, ivec2(size))));
                }

                void main() {
                    ivec2 pixel = ivec2(gl_FragCoord.xy);
                    ivec2 inset = pixel - inset_origin;
                    if (on_border(inset, INSET) || on_border(pixel - source_origin, SOURCE)) {
                        color = vec4(1.0);
                        return;
                    }
                    if (all(greaterThanEqual(inset, ivec2(0))) && all(lessThan(inset, ivec2(INSET)))) {
                        pixel = source_origin + inset / ZOOM;
                    }
                    color = vec4(texelFetch(scene, pixel, 0).rgb, 1.0);
                }
            ",
        );

        let ground = safegl::VertexBuffer::new(
            display,
            &[
                GroundVertex {
                    position: [-6.0, 0.0, -6.0],
                },
                GroundVertex {
                    position: [6.0, 0.0, -6.0],
                },
                GroundVertex {
                    position: [-6.0, 0.0, 6.0],
                },
                GroundVertex {
                    position: [6.0, 0.0, 6.0],
                },
            ],
        )
        .unwrap();

        Application {
            ferns: Ferns::load(display),
            fern_program,
            ground,
            ground_program,
            present_program,
            targets: None,
            mode: initial_mode(),
            camera_angle: 0.6,
            camera_distance: 2.2,
            camera_paused: false,
            last_update: Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        if !self.camera_paused {
            self.camera_angle += (now - self.last_update).as_secs_f32() * 0.1;
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
            self.camera_angle.cos() * self.camera_distance,
            0.3 * self.camera_distance,
            self.camera_angle.sin() * self.camera_distance,
        );
        let view = Matrix4::look_at_rh(&eye, &Point3::new(0.0, 0.25, 0.0), &Vector3::y());
        let aspect = dimensions.0 as f32 / dimensions.1 as f32;
        let projection = Matrix4::new_perspective(aspect, 0.9, 0.05, 50.0);
        let view_projection: [[f32; 4]; 4] = (projection * view).into();

        let depth_test = safegl::Depth {
            test: safegl::DepthTest::IfLess,
            write: true,
            ..Default::default()
        };

        let mut scene =
            SimpleFrameBuffer::with_depth_buffer(display, &targets.color, &targets.depth).unwrap();
        scene.clear_color_and_depth((0.32, 0.45, 0.62, 1.0), 1.0);

        scene
            .draw(
                &self.ground,
                NoIndices(PrimitiveType::TriangleStrip),
                &self.ground_program,
                &uniform! { view_projection: view_projection },
                &safegl::DrawParameters {
                    depth: depth_test,
                    ..Default::default()
                },
            )
            .unwrap();

        let params = safegl::DrawParameters {
            depth: depth_test,
            alpha_to_coverage: self.mode != Mode::AlphaTest,
            sample_shading: (self.mode == Mode::SampleShading).then_some(1.0),
            // keep the first `kept` samples of each pixel
            sample_mask: match self.mode {
                Mode::SampleMask(kept) => Some((1 << kept) - 1),
                _ => None,
            },
            ..Default::default()
        };
        let albedo = Sampler::new(&self.ferns.texture)
            .minify_filter(MinifySamplerFilter::LinearMipmapLinear)
            .magnify_filter(MagnifySamplerFilter::Linear)
            .anisotropy(8);
        let uniforms = uniform! {
            view_projection: view_projection,
            albedo: albedo,
            alpha_mode: self.mode.alpha_mode(),
        };
        for (variant, instances) in self.ferns.variants.iter().zip(&self.ferns.instances) {
            scene
                .draw(
                    (&self.ferns.vertices, instances.per_instance()),
                    self.ferns.indices.slice(variant.clone()).unwrap(),
                    &self.fern_program,
                    &uniforms,
                    &params,
                )
                .unwrap();
        }

        // The resolve: blitting a multisampled image into a single-sampled one averages the
        // samples of each pixel.
        let (width, height) = dimensions;
        scene.blit_whole_color_to(
            &targets.resolved.as_surface(),
            &BlitTarget {
                left: 0,
                bottom: 0,
                width: width as i32,
                height: height as i32,
            },
            MagnifySamplerFilter::Nearest,
        ).unwrap();

        let inset = 256;
        let source = inset / 8;
        let mut frame = display.draw();
        frame
            .draw(
                EmptyVertexAttributes { len: 3 },
                NoIndices(PrimitiveType::TrianglesList),
                &self.present_program,
                &uniform! {
                    scene: &targets.resolved,
                    inset_origin: [width as i32 - inset - 16, 16],
                    source_origin: [(width as i32 - source) / 2, (height as i32 - source) / 2],
                },
                &Default::default(),
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

        let kept = match self.mode {
            // pressing 5 again keeps one more sample
            Mode::SampleMask(kept) => kept % SAMPLES + 1,
            _ => 1,
        };
        match key {
            KeyCode::Digit1 => self.mode = Mode::AlphaTest,
            KeyCode::Digit2 => self.mode = Mode::AlphaToCoverage,
            KeyCode::Digit3 => self.mode = Mode::Sharpened,
            KeyCode::Digit4 => self.mode = Mode::SampleShading,
            KeyCode::Digit5 => self.mode = Mode::SampleMask(kept),
            KeyCode::Space => self.camera_paused = !self.camera_paused,
            KeyCode::ArrowUp => self.camera_distance = (self.camera_distance * 0.9).max(0.6),
            KeyCode::ArrowDown => self.camera_distance = (self.camera_distance / 0.9).min(8.0),
            _ => return,
        }
        window.set_title(&format!(
            "{} - {}",
            Self::WINDOW_TITLE,
            self.mode.describe()
        ));
    }
}

pub fn main() {
    State::<Application>::run_loop();
}
