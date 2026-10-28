#![cfg(all(feature = "webgl", target_family = "wasm", not(target_os = "emscripten")))]
/*!

Backend implementation for WebGL 2 in browsers.

The browser owns the drawing buffer: there is no explicit buffer swap, the canvas is presented
when the current `requestAnimationFrame` callback returns. Finish every `Frame` before yielding
back to the browser, and don't expect the default framebuffer to keep its contents across frames.

# Context loss

The browser can lose the WebGL context at any time, for example when the GPU is reset or when
too many contexts exist. SafeGL asks the browser to restore it (`preventDefault` on the
`webglcontextlost` event). While the context is lost, `Context::is_context_lost` returns true and
the commands do nothing. Once the browser restored it, `WebDisplay::is_context_restored` returns
true, and `WebDisplay::restore` gives a new display that draws with it. Every object of the lost
context is gone, and must be created again with the new display.

# Features

Only available on `wasm32` targets with the `webgl` feature enabled.

*/
use std::cell::Cell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    Event, HtmlCanvasElement, WebGl2RenderingContext, WebGlContextAttributes, WebGlPowerPreference,
};

use crate::backend::{Backend, Facade, SwapInterval};
use crate::context::{self, Context};
use crate::debug;
use crate::{Frame, FrameError, IncompatibleOpenGl, SwapBuffersError};

/// An implementation of the `Backend` trait for a canvas' WebGL 2 context.
pub struct WebGlBackend {
    canvas: HtmlCanvasElement,
    gl: WebGl2RenderingContext,
    /// Whether the browser restored the context after losing it.
    restored: Rc<Cell<bool>>,
    /// The listeners of the loss and of the restoration of the context, removed on drop.
    listeners: [(&'static str, Closure<dyn FnMut(Event)>); 2],
}

impl WebGlBackend {
    /// Wraps an existing WebGL 2 context of `canvas`.
    ///
    /// The backend listens to the loss of the context, so that the browser restores it.
    pub fn new(canvas: HtmlCanvasElement, gl: WebGl2RenderingContext) -> WebGlBackend {
        let restored = Rc::new(Cell::new(false));
        // without `preventDefault`, the browser never restores a lost context
        let lost = Closure::<dyn FnMut(Event)>::new(|event: Event| event.prevent_default());
        let flag = restored.clone();
        let restore = Closure::<dyn FnMut(Event)>::new(move |_: Event| flag.set(true));
        let listeners = [("webglcontextlost", lost), ("webglcontextrestored", restore)];
        for (name, listener) in &listeners {
            let listener = listener.as_ref().unchecked_ref();
            let _ = canvas.add_event_listener_with_callback(name, listener);
        }
        WebGlBackend {
            canvas,
            gl,
            restored,
            listeners,
        }
    }

    /// Returns the canvas the context draws to.
    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    /// Returns true if the browser restored the context after losing it.
    pub fn is_context_restored(&self) -> bool {
        self.restored.get() && !self.gl.is_context_lost()
    }
}

impl Drop for WebGlBackend {
    fn drop(&mut self) {
        for (name, listener) in &self.listeners {
            let listener = listener.as_ref().unchecked_ref();
            let _ = self.canvas.remove_event_listener_with_callback(name, listener);
        }
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
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum WebDisplayCreationError {
    /// The browser couldn't create a WebGL 2 context for the canvas.
    WebGl2NotSupported,
    /// The WebGL implementation doesn't provide what SafeGL needs.
    IncompatibleOpenGl(IncompatibleOpenGl),
    /// `WebDisplay::restore` was called before the browser restored the lost context.
    ContextNotRestored,
}

impl fmt::Display for WebDisplayCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebDisplayCreationError::WebGl2NotSupported => {
                f.write_str("the browser couldn't create a WebGL 2 context")
            }
            WebDisplayCreationError::IncompatibleOpenGl(err) => fmt::Display::fmt(err, f),
            WebDisplayCreationError::ContextNotRestored => {
                f.write_str("the browser hasn't restored the lost WebGL context yet")
            }
        }
    }
}

impl std::error::Error for WebDisplayCreationError {}

impl From<IncompatibleOpenGl> for WebDisplayCreationError {
    fn from(err: IncompatibleOpenGl) -> WebDisplayCreationError {
        WebDisplayCreationError::IncompatibleOpenGl(err)
    }
}

/// Which GPU the browser should use for a WebGL context (`powerPreference`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerPreference {
    /// The browser chooses.
    #[default]
    Default,
    /// A GPU that saves power, such as an integrated one.
    LowPower,
    /// The fastest GPU, such as a discrete one.
    HighPerformance,
}

/// Creates a `WebDisplay` with the attributes of its WebGL 2 context.
///
/// The attributes start with the defaults of WebGL: an alpha channel, antialiasing, a depth
/// buffer, no stencil buffer, premultiplied alpha, a drawing buffer that isn't kept after the
/// canvas is presented, no power preference, no failure on a major performance caveat, and a
/// canvas synchronized with the page.
///
/// A canvas has a single WebGL context: once it is created, by this builder or by other code,
/// the attributes of later requests are ignored.
pub struct WebDisplayBuilder {
    canvas: HtmlCanvasElement,
    attributes: WebGlContextAttributes,
    debug: debug::DebugCallbackBehavior,
}

impl WebDisplayBuilder {
    /// Whether the drawing buffer has an alpha channel, which composites the canvas with the
    /// page.
    pub fn alpha(self, alpha: bool) -> Self {
        self.attributes.set_alpha(alpha);
        self
    }

    /// Whether the drawing buffer is antialiased, with the browser's choice of samples.
    pub fn antialias(self, antialias: bool) -> Self {
        self.attributes.set_antialias(antialias);
        self
    }

    /// Whether the drawing buffer has a depth buffer of at least 16 bits.
    pub fn depth(self, depth: bool) -> Self {
        self.attributes.set_depth(depth);
        self
    }

    /// Whether the drawing buffer has a stencil buffer of 8 bits.
    pub fn stencil(self, stencil: bool) -> Self {
        self.attributes.set_stencil(stencil);
        self
    }

    /// Whether the page composites the colors of the canvas as premultiplied by their alpha.
    pub fn premultiplied_alpha(self, premultiplied: bool) -> Self {
        self.attributes.set_premultiplied_alpha(premultiplied);
        self
    }

    /// Whether the drawing buffer keeps its content after the canvas is presented, until it is
    /// cleared or drawn over. Keeping it can be slower.
    pub fn preserve_drawing_buffer(self, preserve: bool) -> Self {
        self.attributes.set_preserve_drawing_buffer(preserve);
        self
    }

    /// Which GPU the browser should use.
    pub fn power_preference(self, preference: PowerPreference) -> Self {
        self.attributes.set_power_preference(match preference {
            PowerPreference::Default => WebGlPowerPreference::Default,
            PowerPreference::LowPower => WebGlPowerPreference::LowPower,
            PowerPreference::HighPerformance => WebGlPowerPreference::HighPerformance,
        });
        self
    }

    /// Whether the creation fails when the context would be much slower than a native one, for
    /// example when the browser renders without a GPU: `build` then returns
    /// `WebDisplayCreationError::WebGl2NotSupported`.
    pub fn fail_if_major_performance_caveat(self, fail: bool) -> Self {
        self.attributes.set_fail_if_major_performance_caveat(fail);
        self
    }

    /// Whether the canvas is presented without waiting for the rest of the page, which lowers
    /// the latency, and can tear. Browsers without it ignore it.
    pub fn desynchronized(self, desynchronized: bool) -> Self {
        let _ = js_sys::Reflect::set(
            &self.attributes,
            &JsValue::from_str("desynchronized"),
            &JsValue::from_bool(desynchronized),
        );
        self
    }

    /// How the debug output is reported; see `DebugCallbackBehavior`.
    pub fn debug_callback_behavior(self, debug: debug::DebugCallbackBehavior) -> Self {
        WebDisplayBuilder { debug, ..self }
    }

    /// Creates the WebGL 2 context of the canvas and the display that draws to it.
    pub fn build(self) -> Result<WebDisplay, WebDisplayCreationError> {
        let gl = self
            .canvas
            .get_context_with_context_options("webgl2", &self.attributes)
            .ok()
            .flatten()
            .and_then(|context| context.dyn_into::<WebGl2RenderingContext>().ok())
            .ok_or(WebDisplayCreationError::WebGl2NotSupported)?;
        Ok(WebDisplay::from_context_with_debug(
            self.canvas,
            gl,
            self.debug,
        )?)
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
    /// Returns a builder of a display for `canvas`, which sets the attributes of its WebGL 2
    /// context.
    pub fn builder(canvas: HtmlCanvasElement) -> WebDisplayBuilder {
        WebDisplayBuilder {
            canvas,
            attributes: WebGlContextAttributes::new(),
            debug: Default::default(),
        }
    }

    /// Creates a WebGL 2 context for `canvas`, with the default attributes of WebGL (see
    /// `WebDisplayBuilder`), and a display that draws to it.
    pub fn from_canvas(canvas: HtmlCanvasElement) -> Result<WebDisplay, WebDisplayCreationError> {
        Self::from_canvas_with_debug(canvas, Default::default())
    }

    /// The same as `from_canvas`, but allows for specifying debug callback behaviour.
    pub fn from_canvas_with_debug(
        canvas: HtmlCanvasElement,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<WebDisplay, WebDisplayCreationError> {
        Self::builder(canvas).debug_callback_behavior(debug).build()
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

    /// Sizes the drawing buffer of a canvas shown at `css_size` CSS pixels on a screen of
    /// `device_pixel_ratio` device pixels per CSS pixel (`window.devicePixelRatio`), so that it
    /// has a pixel per device pixel, and returns its size. The browser may give a smaller
    /// drawing buffer than requested, when it is larger than the GPU allows.
    pub fn resize_to_display(&self, css_size: (f64, f64), device_pixel_ratio: f64) -> (u32, u32) {
        let pixels = |css: f64| (css * device_pixel_ratio).round().max(1.0) as u32;
        self.resize((pixels(css_size.0), pixels(css_size.1)));
        self.backend.get_framebuffer_dimensions()
    }

    /// Returns true if the browser restored the context after it was lost: `restore` then
    /// gives the display that draws with it.
    pub fn is_context_restored(&self) -> bool {
        self.backend.is_context_restored()
    }

    /// Returns the display of the context that the browser restored after it was lost, with
    /// the default debug behaviour, or `WebDisplayCreationError::ContextNotRestored` if the
    /// browser hasn't restored it yet.
    ///
    /// Every object of the lost context (buffers, textures, programs, ...) is gone: create them
    /// again with the new display. The objects of this display can only be dropped: they belong
    /// to the lost context, which the new display refuses with the `ContextMismatch` errors.
    pub fn restore(&self) -> Result<WebDisplay, WebDisplayCreationError> {
        self.restore_with_debug(Default::default())
    }

    /// The same as `restore`, but allows for specifying debug callback behaviour.
    pub fn restore_with_debug(
        &self,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<WebDisplay, WebDisplayCreationError> {
        if !self.backend.is_context_restored() {
            return Err(WebDisplayCreationError::ContextNotRestored);
        }
        // the objects of this display belong to the lost context for good
        self.context.mark_lost();
        let (canvas, gl) = (self.backend.canvas.clone(), self.backend.gl.clone());
        Ok(Self::from_context_with_debug(canvas, gl, debug)?)
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
