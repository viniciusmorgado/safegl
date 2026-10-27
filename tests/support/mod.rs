/*!
Test supports module.

*/

#![allow(dead_code)]

use safegl::backend::{Context, Facade};
use safegl::index::PrimitiveType;
use safegl::Display;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::NotCurrentContext;
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use safegl::winit::application::ApplicationHandler;
use safegl::winit::event::WindowEvent;
use safegl::winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use safegl::winit::window::{Window, WindowId};

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

/// What the event loop thread creates for the tests.
#[derive(Debug)]
enum DisplayRequest {
    /// A hidden window, with a context and a surface for it.
    Display(Sender<Result<(Window, NotCurrentContext, Surface<WindowSurface>), String>>),
    /// A hidden window, with a surface and the configuration of the contexts that can use it.
    Window(Sender<Result<(Window, glutin::config::Config, Surface<WindowSurface>), String>>),
}

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

    // a failure is sent to the test that made the request: a panic would end the event loop,
    // and with it every test that comes after
    fn user_event(&mut self, event_loop: &ActiveEventLoop, request: DisplayRequest) {
        let window = create_window(event_loop);
        match request {
            DisplayRequest::Display(sender) => {
                let display = window.and_then(|(window, gl_config, surface)| {
                    let raw_window_handle = window.window_handle().unwrap().as_raw();
                    let context =
                        create_context(&gl_config, raw_window_handle, None, parse_version())
                            .ok_or("failed to create a context")?;
                    Ok((window, context, surface))
                });
                // the test may have ended already
                let _ = sender.send(display);
            }
            DisplayRequest::Window(sender) => {
                let _ = sender.send(window);
            }
        }
    }
}

/// Sends `request` to the event loop and waits for its answer, panicking with the error that
/// the event loop gave.
fn request<T>(request: impl FnOnce(Sender<Result<T, String>>) -> DisplayRequest) -> T {
    let (sender, receiver) = std::sync::mpsc::channel();
    event_loop_proxy().send_event(request(sender)).unwrap();
    receiver
        .recv()
        .expect("the event loop of the tests has stopped")
        .unwrap_or_else(|error| panic!("{error}"))
}

/// Creates a hidden window, its surface, and the configuration of the contexts that can use it.
fn create_window(
    event_loop: &ActiveEventLoop,
) -> Result<(Window, glutin::config::Config, Surface<WindowSurface>), String> {
    {
        let window_attributes = Window::default_attributes().with_visible(false);
        let config_template_builder = ConfigTemplateBuilder::new();
        let (window, gl_config) = DisplayBuilder::new()
            .with_preference(parse_display_api())
            .with_window_attributes(Some(window_attributes))
            .build(event_loop, config_template_builder, |mut configs| {
                // Just use the first configuration since we don't have any special preferences here
                configs.next().unwrap()
            })
            .map_err(|error| format!("failed to create a window: {error}"))?;
        let window = window.ok_or("failed to create a window")?;
        let raw_window_handle = window.window_handle().unwrap().as_raw();

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(800).unwrap(),
            NonZeroU32::new(600).unwrap(),
        );

        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .map_err(|error| format!("failed to create a window surface: {error}"))?
        };

        Ok((window, gl_config, surface))
    }
}

/// Creates a context for the windows of `gl_config`, of the first of `apis` that works, which
/// shares its objects with `shared`.
fn create_context(
    gl_config: &glutin::config::Config,
    raw_window_handle: raw_window_handle::RawWindowHandle,
    shared: Option<&glutin::context::PossiblyCurrentContext>,
    apis: Vec<glutin::context::ContextApi>,
) -> Option<NotCurrentContext> {
    apis.into_iter().find_map(|version| {
        let mut context_attributes = safegl::backend::glutin::context_attributes_builder(version);
        if let Some(robustness) = parse_robustness() {
            context_attributes = context_attributes.with_robustness(robustness);
        }
        if let Some(shared) = shared {
            context_attributes = context_attributes.with_sharing(shared);
        }
        let context_attributes = context_attributes.build(Some(raw_window_handle));
        unsafe {
            gl_config
                .display()
                .create_context(gl_config, &context_attributes)
                .ok()
        }
    })
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
    build_display_with_debug(Default::default())
}

/// Returns a proxy of the event loop, which runs on its own thread, started on the first call.
fn event_loop_proxy() -> EventLoopProxy<DisplayRequest> {
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

    EVENT_LOOP_PROXY.read().unwrap().as_ref().unwrap().clone()
}

/// Builds a display for tests, with the given debug callback.
pub fn build_display_with_debug(debug: safegl::debug::DebugCallbackBehavior) -> WindowDisplay {
    // the event loop creates the window and the context
    let (window, not_current_gl_context, surface) = request(DisplayRequest::Display);

    // Now use our surface to make our context current and finally create our display
    let current_context = not_current_gl_context.make_current(&surface).unwrap();

    WindowDisplay {
        display: Display::with_debug(current_context, surface, debug).unwrap(),
        window,
    }
}

/// Builds a display whose context is created with the given attributes, or returns `None` if
/// the driver can't create such a context.
///
/// The display keeps its window, which must outlive the context and the surface: a window
/// destroyed under a current context breaks the next contexts made current with GLX.
pub fn try_build_display_with(
    attributes: glutin::context::ContextAttributesBuilder,
) -> Option<Result<WindowDisplay, safegl::IncompatibleOpenGl>> {
    let (window, gl_config, surface) = request(DisplayRequest::Window);

    let raw_window_handle = window.window_handle().unwrap().as_raw();
    let context = unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &attributes.build(Some(raw_window_handle)))
            .ok()?
    };
    let context = context.make_current(&surface).unwrap();
    Some(
        Display::new(context, surface)
            .map(|display| WindowDisplay { display, window })
            .map_err(|error| match error {
                safegl::backend::glutin::DisplayCreationError::IncompatibleOpenGl(error) => error,
                error => panic!("{error}"),
            }),
    )
}

/// Returns true if the context compiles GLSL `desktop` on desktop OpenGL, or GLSL `es` ES on
/// OpenGL ES, written like `#version` numbers (430, 310).
///
/// A context can have the API of a feature through an extension without the GLSL version that
/// the tests write their shaders in.
pub fn glsl_supported(display: &safegl::Display<WindowSurface>, desktop: u32, es: u32) -> bool {
    let version = if is_gles(display) {
        safegl::Version(safegl::Api::GlEs, (es / 100) as u8, (es / 10 % 10) as u8)
    } else {
        safegl::Version(safegl::Api::Gl, (desktop / 100) as u8, (desktop / 10 % 10) as u8)
    };
    display.is_glsl_version_supported(&version)
}

/// Returns true if the display uses OpenGL ES, for the few tests of features it doesn't have.
pub fn is_gles(display: &safegl::Display<WindowSurface>) -> bool {
    display.get_opengl_version().0 == safegl::Api::GlEs
}

/// Moves an existing display to a new window, with a new context that shares its objects.
///
/// In real applications this is used for things such as switching to fullscreen. Some things are
/// invalidated during a rebuild, and this has to be handled by SafeGL.
pub fn rebuild_display(display: &mut WindowDisplay) {
    try_rebuild_display(display, parse_version())
        .expect("failed to create context")
        .unwrap();
}

/// Same as `rebuild_display`, with a context of the first of `apis` that the driver creates.
/// Returns `None` if it creates none, otherwise the result of `Display::rebuild`.
pub fn try_rebuild_display(
    display: &mut WindowDisplay,
    apis: Vec<glutin::context::ContextApi>,
) -> Option<Result<(), safegl::IncompatibleOpenGl>> {
    let (window, gl_config, surface) = request(DisplayRequest::Window);

    let raw_window_handle = window.window_handle().unwrap().as_raw();
    let context = create_context(
        &gl_config,
        raw_window_handle,
        Some(&display.display.gl_context()),
        apis,
    )?;
    let context = context.make_current(&surface).unwrap();
    let result = unsafe { display.display.rebuild(context, surface) };

    // the old window is closed once the display stopped using it
    if result.is_ok() {
        display.window = window;
    }
    Some(result)
}

/// The display API to try first: EGL with `SAFEGL_GL_DISPLAY=egl`, otherwise the one of the
/// platform (GLX or WGL), as glutin does by default.
fn parse_display_api() -> glutin_winit::ApiPreference {
    match env::var("SAFEGL_GL_DISPLAY").as_deref() {
        Ok("egl") => glutin_winit::ApiPreference::PreferEgl,
        Ok("glx") | Ok("wgl") | Err(_) => glutin_winit::ApiPreference::FallbackEgl,
        Ok(other) => panic!("SAFEGL_GL_DISPLAY must be `egl`, `glx` or `wgl`, not `{other}`"),
    }
}

/// The context APIs that the tests request, in order: the one of `SAFEGL_GL_VERSION` (such as
/// `OpenGL 4.0` or `OpenGL ES 3.2`), or else those of the selected levels.
pub fn context_apis() -> Vec<glutin::context::ContextApi> {
    parse_version()
}

/// The robustness of the contexts of the tests: `SAFEGL_ROBUSTNESS=robust` for robust buffer
/// access, `lose` for robust buffer access with contexts lost on GPU resets, and `no_error` for
/// contexts without error checking. A context that the driver can't create this way fails.
fn parse_robustness() -> Option<glutin::context::Robustness> {
    use glutin::context::Robustness;

    match env::var("SAFEGL_ROBUSTNESS").as_deref() {
        Ok("robust") => Some(Robustness::RobustNoResetNotification),
        Ok("lose") => Some(Robustness::RobustLoseContextOnReset),
        Ok("no_error") => Some(Robustness::NoError),
        Ok(other) => panic!("SAFEGL_ROBUSTNESS must be `robust`, `lose` or `no_error`, not `{other}`"),
        Err(_) => None,
    }
}

/// Returns true if the contexts of the tests check errors: they don't with
/// `SAFEGL_ROBUSTNESS=no_error`, where an invalid OpenGL call is undefined behavior, so the tests
/// that make one on purpose can't run.
pub fn contexts_check_errors() -> bool {
    !matches!(parse_robustness(), Some(glutin::context::Robustness::NoError))
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
        Err(_) => safegl::backend::glutin::level_context_apis(),
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
            330 => {
                vertex: "
                    #version 330

                    in vec2 position;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                ",
                fragment: "
                    #version 330

                    out vec4 f_color;

                    void main() {
                        f_color = vec4(1.0, 0.0, 0.0, 1.0);
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
) -> (
    safegl::vertex::VertexBufferAny,
    safegl::index::IndexBufferAny,
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
    )
}

/// Same as `build_rectangle_vb_ib`, as a list of two triangles drawn without indices, for
/// transform feedback on OpenGL ES 3.0 and 3.1, which only captures such draws.
pub fn build_rectangle_triangles<F: ?Sized>(
    facade: &F,
) -> (safegl::vertex::VertexBufferAny, safegl::index::NoIndices)
where
    F: Facade,
{
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    let vertex = |x, y| Vertex { position: [x, y] };
    let vertices = [
        vertex(-1.0, 1.0),
        vertex(1.0, 1.0),
        vertex(-1.0, -1.0),
        vertex(-1.0, -1.0),
        vertex(1.0, 1.0),
        vertex(1.0, -1.0),
    ];
    (
        safegl::VertexBuffer::new(facade, &vertices).unwrap().into(),
        safegl::index::NoIndices(PrimitiveType::TrianglesList),
    )
}

/// Builds a texture suitable for rendering.
pub fn build_renderable_texture<F: ?Sized>(facade: &F) -> safegl::Texture2d
where
    F: Facade,
{
    safegl::Texture2d::empty(facade, 1024, 1024).unwrap()
}

/// Translates a GLSL 1.10 shader to the GLSL of the context: `#version 330` on desktop OpenGL,
/// and `#version 300 es` with default precisions on OpenGL ES, whose drivers don't all accept
/// GLSL 1.10.
pub fn modern_glsl(display: &Display<WindowSurface>, source: &str, fragment: bool) -> String {
    /// Replaces the identifier `word` by `by` where it isn't part of a longer identifier.
    fn replace_word(line: &str, word: &str, by: &str) -> String {
        let is_identifier = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let mut out = String::with_capacity(line.len());
        let mut rest = line;
        while let Some(position) = rest.find(word) {
            let before = rest[..position].chars().next_back();
            let after = rest[position + word.len()..].chars().next();
            out.push_str(&rest[..position]);
            if before.is_some_and(is_identifier) || after.is_some_and(is_identifier) {
                out.push_str(word);
            } else {
                out.push_str(by);
            }
            rest = &rest[position + word.len()..];
        }
        out.push_str(rest);
        out
    }

    let mut out = String::from(if is_gles(display) {
        "#version 300 es\nprecision highp float;\nprecision highp int;\n"
    } else {
        "#version 330\n"
    });
    if fragment {
        out.push_str("out vec4 support_frag_color;\n");
    }
    for line in source.lines() {
        if line.trim_start().starts_with("#version") {
            continue;
        }
        let line = replace_word(line, "attribute", "in");
        let line = replace_word(&line, "varying", if fragment { "in" } else { "out" });
        let line = replace_word(&line, "gl_FragColor", "support_frag_color");
        let line = replace_word(&line, "texture2D", "texture");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Builds a program from GLSL 1.10 shaders translated with `modern_glsl`.
pub fn program_from_legacy(
    display: &Display<WindowSurface>,
    vertex: &str,
    fragment: &str,
) -> Result<safegl::Program, safegl::ProgramCreationError> {
    safegl::Program::from_source(
        display,
        &modern_glsl(display, vertex, false),
        &modern_glsl(display, fragment, true),
        None,
    )
}
