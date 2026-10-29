//! The displays of the tests on desktop OpenGL and OpenGL ES: hidden windows of an event loop
//! that runs on its own thread, or with `SAFEGL_TEST_DISPLAY=headless` at build time, pbuffers
//! of an EGL device without window system. The environment variables choose the contexts.

use std::env;

// Android has no windows for the tests: they draw without window system there
#[cfg(not(any(safegl_test_headless, target_os = "android")))]
mod windowed;
#[cfg(not(any(safegl_test_headless, target_os = "android")))]
pub use windowed::*;
#[cfg(any(safegl_test_headless, target_os = "android"))]
mod headless;
#[cfg(any(safegl_test_headless, target_os = "android"))]
pub use headless::*;

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
