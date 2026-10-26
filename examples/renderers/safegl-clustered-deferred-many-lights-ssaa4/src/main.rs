mod benchmark;
mod clusters;
mod deferred;
mod fps;
mod model;
mod ssaa;
mod validation;

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use safegl::{
    glutin::{
        config::ConfigTemplateBuilder,
        context::{ContextApi, ContextAttributesBuilder, GlProfile, Version},
        surface::WindowSurface,
    },
    winit::{
        application::ApplicationHandler,
        event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        keyboard::{Key, NamedKey},
        window::{Window, WindowId},
    },
    Display, Surface,
};
use std::path::PathBuf;

const WIDTH: u32 = 1100;
const HEIGHT: u32 = 900;

#[derive(Default)]
struct Options {
    screenshot: Option<PathBuf>,
    frames: Option<u32>,
    all_lights: bool,
    validate_culling: bool,
    benchmark: bool,
}

impl Options {
    fn parse() -> Result<Self> {
        let mut options = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--screenshot" => {
                    options.screenshot =
                        Some(args.next().context("--screenshot needs a PNG path")?.into())
                }
                "--benchmark" => options.benchmark = true,
                "--all-lights" => options.all_lights = true,
                "--validate-culling" => options.validate_culling = true,
                "--frames" => {
                    let frames = args
                        .next()
                        .context("--frames needs a positive count")?
                        .parse()?;
                    anyhow::ensure!(frames > 0, "--frames must be positive");
                    options.frames = Some(frames);
                }
                "--help" | "-h" => {
                    println!("Usage: safegl-clustered-deferred-many-lights-ssaa4 [--screenshot output.png] [--frames N] [--all-lights] [--validate-culling] [--benchmark]\n\nDrag left mouse / arrow keys: orbit | Wheel / +/-: zoom | R: reset | Esc: quit\n--screenshot renders a 1100x900 PNG after FPS warmup and exits.\n--frames N exits after N window frames for a GPU smoke test.\n--all-lights evaluates all 256 lights; --validate-culling checks clustered output against it.");
                    std::process::exit(0);
                }
                _ => bail!("Unknown argument: {arg}; use --help"),
            }
        }
        Ok(options)
    }
}

struct Camera {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.15,
            pitch: 0.08,
            distance: 3.3,
        }
    }
}

impl Camera {
    fn eye(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        ) * self.distance
    }
    fn orbit(&mut self, x: f32, y: f32) {
        self.yaw += x;
        self.pitch = (self.pitch + y).clamp(-1.45, 1.45);
    }
    fn zoom(&mut self, amount: f32) {
        self.distance = (self.distance * (-amount * 0.12).exp()).clamp(1.4, 12.0);
    }
}

struct Renderer {
    // Rust drops fields in declaration order: release GPU objects, then the
    // context/surface, and only then the native window they depend on.
    deferred: deferred::DeferredRenderer,
    ssaa: ssaa::Ssaa,
    all_lights: bool,
    model: model::Model,
    fps: fps::FpsCounter,
    display: Display<WindowSurface>,
    window: Window,
    camera: Camera,
    dragging: bool,
    cursor: Option<(f64, f64)>,
}

impl Renderer {
    fn new(event_loop: &ActiveEventLoop, all_lights: bool) -> Result<Self> {
        let attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 6))))
            .with_profile(GlProfile::Core);
        let (window, display) = safegl::backend::glutin::SimpleWindowBuilder::new()
            .with_title(
                "SafeGL | Clustered deferred | 256 lights | SSAA 4x | Drag to orbit · Scroll to zoom · R reset",
            )
            .with_inner_size(WIDTH, HEIGHT)
            .with_config_template_builder(ConfigTemplateBuilder::new().with_depth_size(24))
            .with_vsync(false)
            .with_context_attributes_builder(attributes)
            .build(event_loop);
        anyhow::ensure!(
            *display.get_opengl_version() >= safegl::Version(safegl::Api::Gl, 4, 6),
            "OpenGL 4.6 Core is required"
        );
        println!("OpenGL: {}", display.get_opengl_version_string());
        println!("VSync: disabled (swap interval 0); frame rate: uncapped");
        println!("GPU: {}", display.get_opengl_renderer_string());
        let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/ferris.glb");
        let model = model::Model::load(&display, &model_path)?;
        let deferred = deferred::DeferredRenderer::new(&display)?;
        println!("Clustered deferred; 256 point lights + weak directional fill; SSAA 4x");
        let fps = fps::FpsCounter::new(&display)?;
        let ssaa = ssaa::Ssaa::new(&display)?;
        Ok(Self {
            ssaa,
            fps,
            window,
            display,
            deferred,
            all_lights,
            model,
            camera: Camera::default(),
            dragging: false,
            cursor: None,
        })
    }

    fn draw<S: Surface>(&mut self, surface: &mut S) -> Result<()> {
        let (width, height) = surface.get_dimensions();
        let aspect = width as f32 / height.max(1) as f32;
        // Increase vertical FOV for portrait windows to retain horizontal framing.
        let fov = if aspect < 1.0 {
            2.0 * (22.5_f32.to_radians().tan() / aspect).atan()
        } else {
            45.0_f32.to_radians()
        };
        let projection = Mat4::perspective_rh_gl(fov, aspect, 0.05, 100.0);
        let eye = self.camera.eye();
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
        self.ssaa.prepare(&self.display, (width, height))?;
        let mut supersampled =
            safegl::framebuffer::SimpleFrameBuffer::new(&self.display, self.ssaa.color())?;
        self.deferred.draw(
            &self.display,
            &self.model,
            (projection, view, eye),
            &mut supersampled,
            self.all_lights,
        )?;
        drop(supersampled);
        self.ssaa.resolve(surface)?;
        self.fps.draw(surface, self.window.scale_factor() as f32)?;
        Ok(())
    }

    fn frame(&mut self) -> Result<()> {
        let mut target = self.display.draw();
        // Always finish the Frame, including on a failed draw.
        let draw_result = self.draw(&mut target);
        let finish_result = target.finish();
        draw_result?;
        finish_result.context("Presenting frame")?;
        self.fps.presented(&self.display)?;
        Ok(())
    }

    fn screenshot(&mut self, path: &std::path::Path) -> Result<()> {
        // Reuse the same deferred and shading passes; an offscreen target makes capture independent
        // of window visibility, compositor timing, and front-buffer availability.
        let color = safegl::texture::SrgbTexture2d::empty(&self.display, WIDTH, HEIGHT)?;
        let mut target = safegl::framebuffer::SimpleFrameBuffer::new(&self.display, &color)?;
        self.draw(&mut target)?;
        let pixels: safegl::texture::RawImage2d<'_, u8> = color.read();
        let image =
            image::RgbaImage::from_raw(pixels.width, pixels.height, pixels.data.into_owned())
                .context("Invalid screenshot dimensions")?;
        image::imageops::flip_vertical(&image)
            .save(path)
            .with_context(|| format!("Saving {}", path.display()))?;
        println!("Saved screenshot: {}", path.display());
        Ok(())
    }
}

#[derive(Default)]
struct App {
    options: Options,
    renderer: Option<Renderer>,
    error: Option<anyhow::Error>,
    frame_count: u32,
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: anyhow::Error) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.renderer = None;
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.renderer = None;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_none() {
            match Renderer::new(event_loop, self.options.all_lights) {
                Ok(renderer) => {
                    renderer.window.request_redraw();
                    self.renderer = Some(renderer);
                }
                Err(error) => self.fail(event_loop, error),
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &self.renderer {
            let size = renderer.window.inner_size();
            if size.width > 0 && size.height > 0 {
                renderer.window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    renderer.display.resize(size.into());
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if self.options.benchmark {
                    if let Err(error) = benchmark::run(renderer) {
                        self.fail(event_loop, error);
                    } else {
                        event_loop.exit();
                    }
                    return;
                }
                if self.options.validate_culling {
                    if let Err(error) = validation::check(renderer) {
                        self.fail(event_loop, error);
                    } else {
                        event_loop.exit();
                    }
                    return;
                }
                let size = renderer.window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }
                if let Err(error) = renderer.frame() {
                    self.fail(event_loop, error);
                    return;
                }
                self.frame_count += 1;
                if let Some(path) = &self.options.screenshot {
                    if renderer.fps.value.is_none() {
                        return;
                    }
                    if let Err(error) = renderer.screenshot(path) {
                        self.fail(event_loop, error);
                        return;
                    }
                    renderer.display.assert_no_error(None);
                    event_loop.exit();
                } else if let Some(frames) = self.options.frames {
                    if self.frame_count >= frames {
                        renderer.display.assert_no_error(None);
                        println!("GPU smoke test passed: {} frames", self.frame_count);
                        if let Some(fps) = renderer.fps.value {
                            println!("Latest FPS counter sample: {fps:.0}");
                        }
                        event_loop.exit();
                    } else {
                        renderer.window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => renderer.dragging = state == ElementState::Pressed,
            WindowEvent::CursorMoved { position, .. } => {
                if renderer.dragging {
                    if let Some((x, y)) = renderer.cursor {
                        renderer.camera.orbit(
                            (x - position.x) as f32 * 0.006,
                            (position.y - y) as f32 * 0.006,
                        );
                        renderer.window.request_redraw();
                    }
                }
                renderer.cursor = Some((position.x, position.y));
            }
            WindowEvent::Focused(false) => {
                renderer.dragging = false;
                renderer.cursor = None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 80.0,
                };
                renderer.camera.zoom(amount);
                renderer.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::ArrowLeft) => renderer.camera.orbit(-0.1, 0.0),
                    Key::Named(NamedKey::ArrowRight) => renderer.camera.orbit(0.1, 0.0),
                    Key::Named(NamedKey::ArrowUp) => renderer.camera.orbit(0.0, 0.1),
                    Key::Named(NamedKey::ArrowDown) => renderer.camera.orbit(0.0, -0.1),
                    Key::Character(key) if key.eq_ignore_ascii_case("r") => {
                        renderer.camera = Camera::default()
                    }
                    Key::Character(key) if key == "+" || key == "=" => renderer.camera.zoom(1.0),
                    Key::Character(key) if key == "-" => renderer.camera.zoom(-1.0),
                    _ => (),
                }
                renderer.window.request_redraw();
            }
            _ => (),
        }
    }
}

fn main() -> Result<()> {
    let options = Options::parse()?;
    let event_loop = EventLoop::builder().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        options,
        ..Default::default()
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error);
    }
    Ok(())
}
