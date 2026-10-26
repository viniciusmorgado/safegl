/*!
Test supports module.

*/

#![allow(dead_code)]

use safegl::backend::{Context, Facade};
use safegl::index::PrimitiveType;
use safegl::Display;

use safegl::winit::application::ApplicationHandler;
use safegl::winit::event::WindowEvent;
use safegl::winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use safegl::winit::window::{Window, WindowId};
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextAttributesBuilder, NotCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;

use std::env;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{mpsc::Sender, Once, RwLock};
use std::thread;

// The code below here down to `build_display` is a workaround due to a lack of a test initialization hook

// There is a Wayland version of this extension trait but the X11 version also works on Wayland
#[cfg(windows)]
use safegl::winit::platform::windows::EventLoopBuilderExtWindows;
#[cfg(unix)]
use safegl::winit::platform::x11::EventLoopBuilderExtX11;

type DisplayRequest = Sender<(Window, NotCurrentContext, Surface<WindowSurface>)>;

struct Tests {}

impl ApplicationHandler<DisplayRequest> for Tests {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, request: DisplayRequest) {
        let window_attributes = Window::default_attributes().with_visible(false);
        let config_template_builder = ConfigTemplateBuilder::new();
        let (window, gl_config) = DisplayBuilder::new()
            .with_window_attributes(Some(window_attributes))
            .build(event_loop, config_template_builder, |mut configs| {
                // Just use the first configuration since we don't have any special preferences here
                configs.next().unwrap()
            })
            .unwrap();
        let window = window.unwrap();
        let raw_window_handle = window.window_handle().unwrap().as_raw();

        // Then the configuration which decides which OpenGL version we'll end up using. Unless
        // SAFEGL_GL_VERSION is set, we try the context versions of SafeGL's tier in order, because
        // glutin's default (3.3 core) is below SafeGL's minimum.
        let not_current_gl_context = parse_version()
            .into_iter()
            .find_map(|version| {
                let context_attributes = ContextAttributesBuilder::new()
                    .with_context_api(version)
                    .build(Some(raw_window_handle));
                unsafe {
                    gl_config
                        .display()
                        .create_context(&gl_config, &context_attributes)
                        .ok()
                }
            })
            .expect("failed to create context");

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(800).unwrap(),
            NonZeroU32::new(600).unwrap(),
        );

        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        request
            .send((window, not_current_gl_context, surface))
            .unwrap();
    }
}

pub struct WindowDisplay {
    display: Display<WindowSurface>,
    window: Window,
}

impl Deref for WindowDisplay {
    type Target = Display<WindowSurface>;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl Facade for WindowDisplay {
    fn get_context(&self) -> &Rc<Context> {
        self.display.get_context()
    }
}

/// Builds a display for tests.
pub fn build_display() -> WindowDisplay {
    static EVENT_LOOP_PROXY: RwLock<Option<EventLoopProxy<DisplayRequest>>> = RwLock::new(None);
    static INIT_EVENT_LOOP: Once = Once::new();

    // Initialize event loop in a separate thread and store a proxy
    INIT_EVENT_LOOP.call_once(|| {
        let (sender, receiver) = std::sync::mpsc::channel();

        thread::Builder::new()
            .name("event_loop".into())
            .spawn(move || {
                let event_loop_res = if cfg!(unix) || cfg!(windows) {
                    EventLoop::with_user_event().with_any_thread(true).build()
                } else {
                    EventLoop::with_user_event().build()
                };
                let event_loop = event_loop_res.expect("event loop building");

                sender.send(event_loop.create_proxy()).unwrap();

                let mut app = Tests {};
                event_loop.run_app(&mut app).unwrap();
            })
            .unwrap();

        *EVENT_LOOP_PROXY.write().unwrap() = Some(receiver.recv().unwrap());
    });

    let (sender, receiver) = std::sync::mpsc::channel();

    // Request event loop create display building pieces and send them back
    EVENT_LOOP_PROXY
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .send_event(sender)
        .unwrap();

    // Block until required display building pieces are received
    let (window, not_current_gl_context, surface) = receiver.recv().unwrap();

    // Now use our surface to make our context current and finally create our display
    let current_context = not_current_gl_context.make_current(&surface).unwrap();

    WindowDisplay {
        display: Display::from_context_surface(current_context, surface).unwrap(),
        window,
    }
}

/// Rebuilds an existing display.
///
/// In real applications this is used for things such as switching to fullscreen. Some things are
/// invalidated during a rebuild, and this has to be handled by SafeGL.
/// Returns true if the display uses OpenGL ES, for the few tests of features it doesn't have.
pub fn is_gles(display: &safegl::Display<WindowSurface>) -> bool {
    display.get_opengl_version().0 == safegl::Api::GlEs
}

pub fn rebuild_display(_display: &safegl::Display<WindowSurface>) {
    todo!();
    /*
    let version = parse_version();
    let event_loop = safegl::winit::event_loop::EventLoop::new();
    let wb = safegl::winit::window::WindowBuilder::new().with_visible(false);
    let cb = glutin::ContextBuilder::new()
        .with_gl_debug_flag(true)
        .with_gl(version);
    display.rebuild(wb, cb, &event_loop).unwrap();
     */
}

fn parse_version() -> Vec<glutin::context::ContextApi> {
    match env::var("SAFEGL_GL_VERSION") {
        Ok(version) => {
            // expects "OpenGL 3.3" for example

            let mut iter = version.rsplitn(2, ' ');

            let version = iter.next().unwrap();
            let ty = iter.next().unwrap();

            let mut iter = version.split('.');
            let major = iter.next().unwrap().parse().unwrap();
            let minor = iter.next().unwrap().parse().unwrap();

            vec![if ty == "OpenGL" {
                glutin::context::ContextApi::OpenGl(Some(glutin::context::Version::new(
                    major, minor,
                )))
            } else if ty == "OpenGL ES" {
                glutin::context::ContextApi::Gles(Some(glutin::context::Version::new(major, minor)))
            } else if ty == "WebGL" {
                glutin::context::ContextApi::Gles(Some(glutin::context::Version::new(major, minor)))
            } else {
                panic!();
            }]
        }
        Err(_) => safegl::backend::glutin::tier_context_apis(),
    }
}

/// Builds a 2x2 unicolor texture.
pub fn build_unicolor_texture2d<F: ?Sized>(
    facade: &F,
    red: f32,
    green: f32,
    blue: f32,
) -> safegl::Texture2d
where
    F: Facade,
{
    let color = (
        (red * 255.0) as u8,
        (green * 255.0) as u8,
        (blue * 255.0) as u8,
    );

    safegl::texture::Texture2d::new(facade, vec![vec![color, color], vec![color, color]]).unwrap()
}

/// Builds a 2x2 depth texture.
pub fn build_constant_depth_texture<F>(facade: &F, depth: f32) -> safegl::texture::DepthTexture2d
where
    F: Facade + ?Sized,
{
    safegl::texture::DepthTexture2d::new(facade, vec![vec![depth, depth], vec![depth, depth]])
        .unwrap()
}

/// Builds a vertex buffer, index buffer, and program, to draw red `(1.0, 0.0, 0.0, 1.0)` to the whole screen.
pub fn build_fullscreen_red_pipeline<F: ?Sized>(
    facade: &F,
) -> (
    safegl::vertex::VertexBufferAny,
    safegl::index::IndexBufferAny,
    safegl::Program,
)
where
    F: Facade,
{
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    (
        safegl::VertexBuffer::new(
            facade,
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
        .unwrap()
        .into(),
        safegl::IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3])
            .unwrap()
            .into(),
        program!(facade,
            110 => {
                vertex: "
                    #version 110

                    attribute vec2 position;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                ",
                fragment: "
                    #version 110

                    void main() {
                        gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
                    }
                ",
            },
            100 => {
                vertex: "
                    #version 100

                    attribute lowp vec2 position;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                ",
                fragment: "
                    #version 100

                    void main() {
                        gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
                    }
                ",
            },
        )
        .unwrap(),
    )
}

/// Builds a vertex buffer and an index buffer corresponding to a rectangle.
///
/// The vertex buffer has the "position" attribute of type "vec2".
pub fn build_rectangle_vb_ib<F: ?Sized>(
    facade: &F,
) -> (safegl::vertex::VertexBufferAny, safegl::index::IndexBufferAny)
where
    F: Facade,
{
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    (
        safegl::VertexBuffer::new(
            facade,
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
        .unwrap()
        .into(),
        safegl::IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3])
            .unwrap()
            .into(),
    )
}

/// Builds a texture suitable for rendering.
pub fn build_renderable_texture<F: ?Sized>(facade: &F) -> safegl::Texture2d
where
    F: Facade,
{
    safegl::Texture2d::empty(facade, 1024, 1024).unwrap()
}
