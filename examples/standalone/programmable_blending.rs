//! Blend modes that fixed-function blending can't do, with the texture barrier; and clearing and
//! copying textures without a framebuffer.
//!
//! Soft round brushes paint on a canvas texture. The canvas is the framebuffer they draw to and,
//! at the same time, a texture their fragment shader reads: each fragment reads the texel under
//! it, blends the brush color into it with a "Photoshop" blend mode, and writes the result back.
//! Reading a texture attached to the framebuffer being drawn to is normally undefined;
//! `texture::texture_barrier` between the brushes makes it well defined, because each draw
//! then reads only texels that no other draw since the last barrier has written.
//!
//! - `1`: overlay, `2`: soft light, `3`: color dodge.
//! - `Space`: copy the canvas into the snapshot shown in the bottom-left corner
//!   (`TextureAnyMipmap::copy_to`, `glCopyImageSubData`). A snapshot is also taken every
//!   4 seconds.
//! - `C`: clear the canvas to grey (`TextureAnyMipmap::clear`, `glClearTexImage`).
//!
//! The first blend mode can also be given on the command line:
//! `cargo run --example programmable_blending -- 2`.

#[macro_use]
extern crate safegl;

use glutin::surface::WindowSurface;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{texture_barrier, MipmapsOption, Texture2d, UncompressedFloatFormat};
use safegl::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler};
use safegl::vertex::EmptyVertexAttributes;
use safegl::winit::event::{ElementState, KeyEvent, WindowEvent};
use safegl::winit::keyboard::{KeyCode, PhysicalKey};
use safegl::{Display, Program, Surface};
use std::time::Instant;
use support::{ApplicationContext, State};

mod support;

const MODES: [&str; 3] = ["overlay", "soft light", "color dodge"];

/// Brushes: color, radius in pixels, and the frequencies of their path.
const BRUSHES: [([f32; 3], f32, f32, f32); 8] = [
    ([0.95, 0.30, 0.20], 70.0, 0.31, 0.47),
    ([0.20, 0.55, 0.95], 90.0, 0.23, 0.37),
    ([0.95, 0.80, 0.20], 55.0, 0.41, 0.29),
    ([0.30, 0.85, 0.45], 80.0, 0.19, 0.53),
    ([0.70, 0.30, 0.90], 65.0, 0.37, 0.21),
    ([0.10, 0.10, 0.15], 50.0, 0.27, 0.43),
    ([0.95, 0.95, 0.90], 60.0, 0.45, 0.33),
    ([0.95, 0.50, 0.70], 75.0, 0.17, 0.39),
];

/// The canvas being painted, and the last snapshot of it.
struct Canvas {
    paint: Texture2d,
    snapshot: Texture2d,
}

impl Canvas {
    fn new(display: &Display<WindowSurface>, (width, height): (u32, u32)) -> Canvas {
        let texture = || {
            Texture2d::empty_with_format(
                display,
                UncompressedFloatFormat::F16F16F16F16,
                MipmapsOption::NoMipmap,
                width,
                height,
            )
            .unwrap()
        };
        let canvas = Canvas {
            paint: texture(),
            snapshot: texture(),
        };
        canvas.clear();
        canvas
            .snapshot
            .main_level()
            .clear((0.0f32, 0.0f32, 0.0f32, 1.0f32))
            .unwrap();
        canvas
    }

    /// Clears the canvas to grey, which overlay and soft light leave unchanged with a grey
    /// brush.
    fn clear(&self) {
        self.paint
            .main_level()
            .clear((0.5f32, 0.5f32, 0.5f32, 1.0f32))
            .unwrap();
    }

    /// Copies the canvas into the snapshot, texel by texel, without a framebuffer.
    fn snapshot(&self) {
        self.paint
            .main_level()
            .copy_to(&self.snapshot.main_level())
            .unwrap();
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.paint.width(), self.paint.height())
    }
}

struct Application {
    brush_program: Program,
    present_program: Program,
    canvas: Option<Canvas>,
    mode: usize,
    start: Instant,
    last_snapshot: Instant,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "SafeGL programmable_blending example";

    fn new(display: &Display<WindowSurface>) -> Self {
        if !safegl::texture::is_texture_barrier_supported(display) {
            panic!(
                "This example needs OpenGL 4.5, GL_ARB_texture_barrier or GL_NV_texture_barrier"
            );
        }

        // The canvas holds the colors to display as they are, so the programs output them
        // without sRGB conversion (`from_source` sets `outputs_srgb`).
        let brush_program = Program::from_source(
            display,
            "
                #version 330 core

                // in pixels
                uniform vec2 center;
                uniform float radius;
                uniform vec2 canvas_size;

                void main() {
                    // a square around the brush, as a triangle strip
                    vec2 corner = vec2(gl_VertexID & 1, gl_VertexID >> 1) * 2.0 - 1.0;
                    vec2 pixel = center + corner * radius;
                    gl_Position = vec4(pixel / canvas_size * 2.0 - 1.0, 0.0, 1.0);
                }
            ",
            "
                #version 330 core

                // the canvas that this draw call is also drawing to
                uniform sampler2D canvas;
                uniform vec2 center;
                uniform float radius;
                uniform vec3 paint;
                uniform int mode;

                out vec4 color;

                vec3 overlay(vec3 base, vec3 blend) {
                    return mix(2.0 * base * blend,
                               1.0 - 2.0 * (1.0 - base) * (1.0 - blend),
                               step(0.5, base));
                }

                vec3 soft_light(vec3 base, vec3 blend) {
                    vec3 d = mix(((16.0 * base - 12.0) * base + 4.0) * base, sqrt(base),
                                 step(0.25, base));
                    return mix(base - (1.0 - 2.0 * blend) * base * (1.0 - base),
                               base + (2.0 * blend - 1.0) * (d - base),
                               step(0.5, blend));
                }

                vec3 color_dodge(vec3 base, vec3 blend) {
                    return min(base / max(1.0 - blend, 0.001), 1.0);
                }

                void main() {
                    // The texel this fragment will overwrite. No other draw call since the
                    // last barrier wrote it, so the value is the one left by earlier brushes.
                    vec3 base = texelFetch(canvas, ivec2(gl_FragCoord.xy), 0).rgb;

                    vec3 blended;
                    if (mode == 0) {
                        blended = overlay(base, paint);
                    } else if (mode == 1) {
                        blended = soft_light(base, paint);
                    } else {
                        blended = color_dodge(base, paint);
                    }

                    // a soft round brush, painting a little at a time
                    float distance = length(gl_FragCoord.xy - center) / radius;
                    float strength = (1.0 - smoothstep(0.5, 1.0, distance)) * 0.08;
                    color = vec4(mix(base, blended, strength), 1.0);
                }
            ",
            None,
        )
        .unwrap();

        let present_program = Program::from_source(
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

                uniform sampler2D paint;
                uniform sampler2D snapshot;
                // the snapshot is shown at a quarter of the size, 16 pixels from the corner
                uniform vec2 inset_size;

                out vec4 color;

                void main() {
                    vec2 inset = (gl_FragCoord.xy - 16.0) / inset_size;
                    if (all(greaterThanEqual(inset, vec2(0.0))) && all(lessThan(inset, vec2(1.0)))) {
                        color = texture(snapshot, inset);
                    } else if (all(greaterThanEqual(inset, vec2(-0.01)))
                               && all(lessThan(inset, vec2(1.01)))) {
                        color = vec4(1.0);
                    } else {
                        color = texelFetch(paint, ivec2(gl_FragCoord.xy), 0);
                    }
                }
            ",
            None,
        )
        .unwrap();

        let mode = std::env::args()
            .nth(1)
            .and_then(|arg| arg.parse::<usize>().ok())
            .filter(|mode| (1..=MODES.len()).contains(mode))
            .map_or(0, |mode| mode - 1);
        println!("blend mode: {}", MODES[mode]);

        Application {
            brush_program,
            present_program,
            canvas: None,
            mode,
            start: Instant::now(),
            last_snapshot: Instant::now(),
        }
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let dimensions = display.get_framebuffer_dimensions();
        if dimensions.0 == 0 || dimensions.1 == 0 {
            return;
        }
        if self.canvas.as_ref().map(Canvas::dimensions) != Some(dimensions) {
            self.canvas = Some(Canvas::new(display, dimensions));
        }
        let canvas = self.canvas.as_ref().unwrap();

        // Each brush is one draw call that reads and writes the canvas. The barrier after it
        // makes its writes visible to the reads of the next brush, which may overlap it.
        if self.last_snapshot.elapsed().as_secs_f32() >= 4.0 {
            canvas.snapshot();
            self.last_snapshot = Instant::now();
        }

        let time = self.start.elapsed().as_secs_f32();
        let (width, height) = (dimensions.0 as f32, dimensions.1 as f32);
        let mut surface = canvas.paint.as_surface();
        for (index, (paint, radius, fx, fy)) in BRUSHES.iter().enumerate() {
            let phase = index as f32 * 1.7;
            let center = [
                width * (0.5 + 0.4 * (time * fx + phase).sin()),
                height * (0.5 + 0.4 * (time * fy + phase * 0.6).cos()),
            ];
            surface
                .draw(
                    EmptyVertexAttributes { len: 4 },
                    NoIndices(PrimitiveType::TriangleStrip),
                    &self.brush_program,
                    &uniform! {
                        canvas: Sampler::new(&canvas.paint)
                            .magnify_filter(MagnifySamplerFilter::Nearest)
                            .minify_filter(MinifySamplerFilter::Nearest),
                        center: center,
                        radius: *radius,
                        canvas_size: [width, height],
                        paint: *paint,
                        mode: self.mode as i32,
                    },
                    &Default::default(),
                )
                .unwrap();
            texture_barrier(display).unwrap();
        }
        drop(surface);

        let mut frame = display.draw();
        frame
            .draw(
                EmptyVertexAttributes { len: 3 },
                NoIndices(PrimitiveType::TrianglesList),
                &self.present_program,
                &uniform! {
                    paint: &canvas.paint,
                    snapshot: Sampler::new(&canvas.snapshot)
                        .magnify_filter(MagnifySamplerFilter::Linear)
                        .minify_filter(MinifySamplerFilter::Linear),
                    inset_size: [width / 4.0, height / 4.0],
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

        match key {
            KeyCode::Digit1 => self.mode = 0,
            KeyCode::Digit2 => self.mode = 1,
            KeyCode::Digit3 => self.mode = 2,
            KeyCode::Space => {
                if let Some(canvas) = &self.canvas {
                    canvas.snapshot();
                }
            }
            KeyCode::KeyC => {
                if let Some(canvas) = &self.canvas {
                    canvas.clear();
                }
            }
            _ => return,
        }
        window.set_title(&format!(
            "{} - blend mode: {}",
            Self::WINDOW_TITLE,
            MODES[self.mode]
        ));
    }
}

fn main() {
    State::<Application>::run_loop();
}
