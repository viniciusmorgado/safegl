//! The displays of the tests on desktop OpenGL and OpenGL ES: hidden windows of an event loop
//! that runs on its own thread, with contexts chosen by the environment variables.

use safegl::backend::{Context, Facade};
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
use std::io::Write;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{mpsc::Sender, Mutex, Once, RwLock};
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
    let stopped = || -> ! {
        let reason = EVENT_LOOP_STOPPED.lock().unwrap().clone();
        panic!("the event loop of the tests has stopped: {reason:?}")
    };
    let (sender, receiver) = std::sync::mpsc::channel();
    if event_loop_proxy().send_event(request(sender)).is_err() {
        stopped();
    }
    match receiver.recv() {
        Ok(result) => result.unwrap_or_else(|error| panic!("{error}")),
        Err(_) => stopped(),
    }
}

/// Why the event loop of the tests stopped, if it did.
static EVENT_LOOP_STOPPED: Mutex<Option<String>> = Mutex::new(None);

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

                // the event loop thread inherits the output capture of the test that started it,
                // which hides its panics: the reason it stops is written to the standard error
                // directly and given to the tests that come after
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut app = Tests {};
                    event_loop.run_app(&mut app)
                }));
                let reason = match result {
                    Ok(Ok(())) => "it exited".to_owned(),
                    Ok(Err(error)) => format!("{error}"),
                    Err(payload) => payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "it panicked".to_owned()),
                };
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "the event loop of the tests stopped: {reason}"
                );
                *EVENT_LOOP_STOPPED.lock().unwrap() = Some(reason);
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


/// The display type that the helpers of the tests take.
pub type Backend = Display<WindowSurface>;

/// The display of a test, which keeps its window.
pub type TestDisplay = WindowDisplay;

/// The extensions that `SAFEGL_DISABLE_EXTENSIONS` hides.
pub fn disabled_extensions() -> String {
    env::var("SAFEGL_DISABLE_EXTENSIONS").unwrap_or_default()
}

/// Runs a test of `async_test!` to completion: `until` doesn't wait outside of browsers.
#[allow(dead_code)]
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match std::pin::pin!(future).poll(&mut context) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => unreachable!("only browsers wait for the next task"),
    }
}

/// Returns at once: outside of browsers, the results of queries and fences can be waited for.
#[allow(dead_code)]
pub async fn until(_ready: impl Fn() -> bool) {}
