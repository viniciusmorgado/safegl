#![cfg(all(feature = "webgl", target_family = "wasm", not(target_os = "emscripten")))]
/*!

Backend implementation for WebGL 2 in browsers.

The browser owns the drawing buffer: there is no explicit buffer swap, the canvas is presented
when the current `requestAnimationFrame` callback returns. Finish every `Frame` before yielding
back to the browser, and don't expect the default framebuffer to keep its contents across frames.

# Features

Only available on `wasm32` targets with the `webgl` feature enabled.

*/
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

use crate::backend::{Backend, Facade, SwapInterval};
use crate::context::{self, Context};
use crate::debug;
use crate::{Frame, FrameError, IncompatibleOpenGl, SwapBuffersError};

/// An implementation of the `Backend` trait for a canvas' WebGL 2 context.
pub struct WebGlBackend {
    canvas: HtmlCanvasElement,
    gl: WebGl2RenderingContext,
}

impl WebGlBackend {
    /// Wraps an existing WebGL 2 context of `canvas`.
    pub fn new(canvas: HtmlCanvasElement, gl: WebGl2RenderingContext) -> WebGlBackend {
        WebGlBackend { canvas, gl }
    }

    /// Returns the canvas the context draws to.
    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }
}

unsafe impl Backend for WebGlBackend {
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        // the browser presents the canvas when control returns to it
        if self.gl.is_context_lost() {
            Err(SwapBuffersError::ContextLost)
        } else {
            Ok(())
        }
    }

    unsafe fn create_glow_context(&self) -> glow::Context {
        glow::Context::from_webgl2_context(self.gl.clone())
    }

    fn webgl2_context(&self) -> WebGl2RenderingContext {
        self.gl.clone()
    }

    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        (
            self.gl.drawing_buffer_width() as u32,
            self.gl.drawing_buffer_height() as u32,
        )
    }

    fn resize(&self, new_size: (u32, u32)) {
        self.canvas.set_width(new_size.0);
        self.canvas.set_height(new_size.1);
    }

    fn set_swap_interval(&self, _interval: SwapInterval) {
        // the browser decides when the canvas is presented
    }

    fn is_current(&self) -> bool {
        true
    }

    unsafe fn make_current(&self) {}

    fn is_context_lost(&self) -> bool {
        self.gl.is_context_lost()
    }
}

/// Error that can happen while creating a `WebDisplay`.
#[derive(Debug)]
pub enum WebDisplayCreationError {
    /// The browser couldn't create a WebGL 2 context for the canvas.
    WebGl2NotSupported,
    /// The WebGL implementation doesn't provide what SafeGL needs.
    IncompatibleOpenGl(IncompatibleOpenGl),
}

impl fmt::Display for WebDisplayCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebDisplayCreationError::WebGl2NotSupported => {
                f.write_str("the browser couldn't create a WebGL 2 context")
            }
            WebDisplayCreationError::IncompatibleOpenGl(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for WebDisplayCreationError {}

impl From<IncompatibleOpenGl> for WebDisplayCreationError {
    fn from(err: IncompatibleOpenGl) -> WebDisplayCreationError {
        WebDisplayCreationError::IncompatibleOpenGl(err)
    }
}

/// A facade for drawing on a canvas with WebGL 2.
#[derive(Clone)]
pub struct WebDisplay {
    // contains everything related to the current SafeGL context and its state
    context: Rc<Context>,
    backend: Rc<WebGlBackend>,
}

impl fmt::Debug for WebDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[safegl::backend::web::WebDisplay]")
    }
}

impl WebDisplay {
    /// Creates a WebGL 2 context for `canvas` and a display that draws to it.
    pub fn from_canvas(canvas: HtmlCanvasElement) -> Result<WebDisplay, WebDisplayCreationError> {
        Self::from_canvas_with_debug(canvas, Default::default())
    }

    /// The same as `from_canvas`, but allows for specifying debug callback behaviour.
    pub fn from_canvas_with_debug(
        canvas: HtmlCanvasElement,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<WebDisplay, WebDisplayCreationError> {
        let gl = canvas
            .get_context("webgl2")
            .ok()
            .flatten()
            .and_then(|context| context.dyn_into::<WebGl2RenderingContext>().ok())
            .ok_or(WebDisplayCreationError::WebGl2NotSupported)?;

        Ok(Self::from_context_with_debug(canvas, gl, debug)?)
    }

    /// Creates a display from a WebGL 2 context that was already created for `canvas`, for
    /// example with custom context attributes.
    pub fn from_context(
        canvas: HtmlCanvasElement,
        gl: WebGl2RenderingContext,
    ) -> Result<WebDisplay, IncompatibleOpenGl> {
        Self::from_context_with_debug(canvas, gl, Default::default())
    }

    /// The same as `from_context`, but allows for specifying debug callback behaviour.
    pub fn from_context_with_debug(
        canvas: HtmlCanvasElement,
        gl: WebGl2RenderingContext,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<WebDisplay, IncompatibleOpenGl> {
        let backend = Rc::new(WebGlBackend::new(canvas, gl));
        let context = unsafe { context::Context::new(backend.clone(), false, debug) }?;
        Ok(WebDisplay { context, backend })
    }

    /// Returns the canvas this display draws to.
    pub fn canvas(&self) -> &HtmlCanvasElement {
        self.backend.canvas()
    }

    /// Resizes the canvas' drawing buffer.
    pub fn resize(&self, new_size: (u32, u32)) {
        self.backend.resize(new_size)
    }

    /// Start drawing on the canvas.
    ///
    /// The returned `Frame` must be finished before control returns to the browser.
    ///
    /// # Panic
    ///
    /// Panics if the previous `Frame` hasn't been finished: there can only be one at a time.
    /// `try_draw` returns an error instead.
    pub fn draw(&self) -> Frame {
        self.try_draw()
            .expect("the previous `Frame` must be finished before drawing a new one")
    }

    /// Same as `draw`, but returns `FrameError::AlreadyDrawing` if the previous `Frame` hasn't
    /// been finished.
    pub fn try_draw(&self) -> Result<Frame, FrameError> {
        Frame::new(
            self.context.clone(),
            self.backend.get_framebuffer_dimensions(),
        )
    }
}

impl Deref for WebDisplay {
    type Target = Context;

    fn deref(&self) -> &Context {
        &self.context
    }
}

impl Facade for WebDisplay {
    fn get_context(&self) -> &Rc<Context> {
        &self.context
    }
}
