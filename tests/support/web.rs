//! The displays of the tests in browsers: a WebGL 2 context on a canvas of the test page, run by
//! `wasm-bindgen-test-runner` in a headless browser.

use safegl::backend::web::WebDisplay;
use safegl::backend::{Context, Facade};
use std::ops::Deref;
use std::rc::Rc;
use wasm_bindgen::JsCast;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// The display type that the helpers of the tests take.
pub type Backend = WebDisplay;

/// The display of a test. Its context is lost when it is dropped, since a page can only keep a
/// few WebGL contexts and every test of a file runs in the same page.
pub struct TestDisplay {
    display: WebDisplay,
}

impl Deref for TestDisplay {
    type Target = WebDisplay;

    fn deref(&self) -> &WebDisplay {
        &self.display
    }
}

impl Facade for TestDisplay {
    fn get_context(&self) -> &Rc<Context> {
        self.display.get_context()
    }
}

impl Drop for TestDisplay {
    fn drop(&mut self) {
        let lose = self
            .display
            .canvas()
            .get_context("webgl2")
            .ok()
            .flatten()
            .and_then(|gl| gl.dyn_into::<web_sys::WebGl2RenderingContext>().ok())
            .and_then(|gl| gl.get_extension("WEBGL_lose_context").ok().flatten())
            .map(|extension| extension.unchecked_into::<web_sys::WebglLoseContext>());
        if let Some(lose) = lose {
            lose.lose_context();
        }
    }
}

/// Builds a display for tests.
pub fn build_display() -> TestDisplay {
    build_display_with_debug(Default::default())
}

/// Builds a display for tests, with the given debug callback. The canvas has the size of the
/// windows of the native tests, a stencil buffer, no antialiasing (so that its pixels can be
/// read exactly), and keeps what is drawn after it is presented.
pub fn build_display_with_debug(debug: safegl::debug::DebugCallbackBehavior) -> TestDisplay {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_width(800);
    canvas.set_height(600);
    let attributes = web_sys::WebGlContextAttributes::new();
    attributes.set_antialias(false);
    attributes.set_stencil(true);
    attributes.set_depth(true);
    attributes.set_preserve_drawing_buffer(true);
    let gl: web_sys::WebGl2RenderingContext = canvas
        .get_context_with_context_options("webgl2", &attributes)
        .unwrap()
        .expect("the browser has no WebGL 2")
        .dyn_into()
        .unwrap();
    TestDisplay {
        display: WebDisplay::from_context_with_debug(canvas, gl, debug).unwrap(),
    }
}

/// Returns true if the contexts of the tests check errors, which WebGL contexts always do.
pub fn contexts_check_errors() -> bool {
    true
}

/// The extensions that `SAFEGL_DISABLE_EXTENSIONS` hides, from the environment of the build,
/// since browsers have none.
pub fn disabled_extensions() -> String {
    option_env!("SAFEGL_DISABLE_EXTENSIONS")
        .unwrap_or_default()
        .to_owned()
}

/// Waits for the next task of the browser's event loop, in which WebGL updates the results of
/// queries and the status of fences.
pub async fn next_task() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback(&resolve)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

/// Waits, task after task of the event loop, until `ready` returns true, for at most ten
/// seconds.
#[allow(dead_code)]
pub async fn until(ready: impl Fn() -> bool) {
    let start = js_sys::Date::now();
    while !ready() {
        assert!(js_sys::Date::now() - start < 10_000.0, "never ready");
        next_task().await;
    }
}
