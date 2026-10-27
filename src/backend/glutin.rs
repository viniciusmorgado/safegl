#![cfg(feature = "glutin")]
/*!

Backend implementation for the glutin library

# Features

Only available if the 'glutin' feature is enabled.

*/
pub use glutin;
use glutin::surface::Surface;
use glutin::surface::SwapInterval;

use crate::backend;
use crate::backend::Backend;
use crate::backend::Context;
use crate::context;
use crate::debug;
use crate::glutin::context::PossiblyCurrentContext;
use crate::glutin::display::GetGlDisplay;
use crate::glutin::prelude::*;
use crate::glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
use crate::SwapBuffersError;
use crate::{Frame, FrameError, IncompatibleOpenGl};
use std::cell::RefCell;
use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::os::raw::c_void;
use std::rc::Rc;

/// Returns the context APIs to request from glutin, in order of preference, for the OpenGL tier
/// SafeGL was compiled with.
///
/// glutin requests an OpenGL 3.3 core context when no version is given, which is below SafeGL's
/// minimum, so context creation code should try these in order and keep the first that succeeds.
pub fn tier_context_apis() -> Vec<glutin::context::ContextApi> {
    use glutin::context::{ContextApi, Version};

    let mut apis = Vec::with_capacity(3);
    if cfg!(feature = "gl46") {
        apis.push(ContextApi::OpenGl(Some(Version::new(4, 6))));
    }
    if cfg!(feature = "gl41") {
        apis.push(ContextApi::OpenGl(Some(Version::new(4, 1))));
    }
    apis.push(ContextApi::Gles(Some(Version::new(3, 0))));
    apis
}

/// Returns the context attributes for `api`, which request the core profile for desktop OpenGL:
/// SafeGL refuses compatibility profile contexts.
pub fn context_attributes_builder(
    api: glutin::context::ContextApi,
) -> glutin::context::ContextAttributesBuilder {
    use glutin::context::{ContextApi, ContextAttributesBuilder, GlProfile};

    let builder = ContextAttributesBuilder::new().with_context_api(api);
    match api {
        ContextApi::OpenGl(_) => builder.with_profile(GlProfile::Core),
        ContextApi::Gles(_) => builder,
    }
}

#[cfg(feature = "simple_window_builder")]
pub use self::simple_window_builder::SimpleWindowBuilder;

#[cfg(feature = "simple_window_builder")]
pub mod simple_window_builder;

/// Wraps a glutin context together with the corresponding Surface.
/// This is necessary so that we can swap buffers and determine the framebuffer size within SafeGL.
pub struct ContextSurfacePair<T: SurfaceTypeTrait + ResizeableSurface> {
    context: PossiblyCurrentContext,
    surface: glutin::surface::Surface<T>,
}

impl<T: SurfaceTypeTrait + ResizeableSurface> ContextSurfacePair<T> {
    fn new(context: PossiblyCurrentContext, surface: glutin::surface::Surface<T>) -> Self {
        Self { context, surface }
    }

    #[inline]
    /// Return the stored framebuffer dimensions
    pub fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        (
            self.surface.width().unwrap(),
            self.surface.height().unwrap(),
        )
    }

    #[inline]
    /// Swaps the underlying back buffers when the surface is not single buffered.
    pub fn swap_buffers(&self) -> Result<(), glutin::error::Error> {
        self.surface.swap_buffers(&self.context)
    }

    #[inline]
    /// Set swap interval for the surface.
    pub fn set_swap_interval(&self, interval: SwapInterval) -> Result<(), glutin::error::Error> {
        self.surface.set_swap_interval(&self.context, interval)
    }

    #[inline]
    /// Resize the associated surface
    pub fn resize(&self, new_size: (u32, u32)) {
        // Make sure that no dimension is zero, which happens when minimizing on Windows for example.
        let width = NonZeroU32::new(new_size.0).unwrap_or(NonZeroU32::new(1).unwrap());
        let height = NonZeroU32::new(new_size.1).unwrap_or(NonZeroU32::new(1).unwrap());
        self.surface.resize(&self.context, width, height);
    }
}

impl<T: SurfaceTypeTrait + ResizeableSurface> Deref for ContextSurfacePair<T> {
    type Target = PossiblyCurrentContext;
    #[inline]
    fn deref(&self) -> &PossiblyCurrentContext {
        &self.context
    }
}

/// A GL context combined with a facade for drawing upon.
///
/// The `Display` uses **glutin** for the **Window** and its associated GL **Context**.
///
/// These are stored alongside a SafeGL-specific context.
#[derive(Clone)]
pub struct Display<T: SurfaceTypeTrait + ResizeableSurface + 'static> {
    // contains everything related to the current SafeGL context and its state
    context: Rc<context::Context>,
    // The glutin Surface alongside its associated glutin Context.
    gl_context: Rc<RefCell<Option<ContextSurfacePair<T>>>>,
}

/// An implementation of the `Backend` trait for glutin.
#[derive(Clone)]
pub struct GlutinBackend<T: SurfaceTypeTrait + ResizeableSurface>(
    Rc<RefCell<Option<ContextSurfacePair<T>>>>,
);

/// Error that can happen while creating a SafeGL display.
#[derive(Debug)]
pub enum DisplayCreationError {
    /// An error has happened while creating the backend.
    GlutinError(glutin::error::Error),
    /// The OpenGL implementation is too old.
    IncompatibleOpenGl(IncompatibleOpenGl),
}

impl<T: SurfaceTypeTrait + ResizeableSurface> std::fmt::Debug for Display<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[safegl::backend::glutin::Display]")
    }
}

impl<T: SurfaceTypeTrait + ResizeableSurface> Display<T> {
    /// Create a new SafeGL `Display` from the given context and surface.
    ///
    /// Performs a compatibility check to make sure that all core elements of SafeGL are supported
    /// by the implementation.
    pub fn new(
        context: PossiblyCurrentContext,
        surface: Surface<T>,
    ) -> Result<Self, DisplayCreationError> {
        Self::from_context_surface(context, surface).map_err(From::from)
    }

    /// Create a new SafeGL `Display` from the given context and surface.
    ///
    /// Performs a compatibility check to make sure that all core elements of SafeGL are supported
    /// by the implementation.
    pub fn from_context_surface(
        context: PossiblyCurrentContext,
        surface: Surface<T>,
    ) -> Result<Self, IncompatibleOpenGl> {
        Self::with_debug(context, surface, Default::default())
    }

    /// Create a new SafeGL `Display` from the given context and surface.
    ///
    /// This function does the same as `from_context_surface`, except that the resulting context
    /// will assume that the current OpenGL context will never change.
    pub unsafe fn unchecked(
        context: PossiblyCurrentContext,
        surface: Surface<T>,
    ) -> Result<Self, IncompatibleOpenGl> {
        Self::unchecked_with_debug(context, surface, Default::default())
    }

    /// The same as the `new` constructor, but allows for specifying debug callback behaviour.
    pub fn with_debug(
        context: PossiblyCurrentContext,
        surface: Surface<T>,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<Self, IncompatibleOpenGl> {
        Self::new_inner(context, surface, debug, true)
    }

    /// The same as the `unchecked` constructor, but allows for specifying debug callback behaviour.
    pub unsafe fn unchecked_with_debug(
        context: PossiblyCurrentContext,
        surface: Surface<T>,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<Self, IncompatibleOpenGl> {
        Self::new_inner(context, surface, debug, false)
    }

    fn new_inner(
        context: PossiblyCurrentContext,
        surface: Surface<T>,
        debug: debug::DebugCallbackBehavior,
        checked: bool,
    ) -> Result<Self, IncompatibleOpenGl> {
        let context_surface_pair = ContextSurfacePair::new(context, surface);
        let gl_window = Rc::new(RefCell::new(Some(context_surface_pair)));
        let glutin_backend = GlutinBackend(gl_window.clone());
        let context = unsafe { context::Context::new(glutin_backend, checked, debug) }?;
        Ok(Display {
            gl_context: gl_window,
            context,
        })
    }

    /// Returns the glutin context, for example to create a context that shares its objects with
    /// `ContextAttributesBuilder::with_sharing` before calling `rebuild`.
    ///
    /// # Panic
    ///
    /// Panics if the returned reference is still alive when `rebuild` is called.
    #[inline]
    pub fn gl_context(&self) -> std::cell::Ref<'_, PossiblyCurrentContext> {
        std::cell::Ref::map(self.gl_context.borrow(), |pair| {
            &pair.as_ref().unwrap().context
        })
    }

    /// Replaces the glutin context and surface of the display, for example to move it to another
    /// window.
    ///
    /// The textures, buffers, programs and other objects of SafeGL keep working: the new context
    /// must share them with the current one, and provide the same OpenGL version and extensions.
    /// If it doesn't, this returns an error and the display keeps its current context and
    /// surface.
    ///
    /// # Safety
    ///
    /// `context` must share its objects with the current context of the display, for example by
    /// being created with `ContextAttributesBuilder::with_sharing(&*display.gl_context())`.
    pub unsafe fn rebuild(
        &self,
        context: PossiblyCurrentContext,
        surface: Surface<T>,
    ) -> Result<(), IncompatibleOpenGl> {
        let pair = ContextSurfacePair::new(context, surface);
        let gl_context = &self.gl_context;
        self.context.rebuild_inner(move |_| {
            let old_pair = gl_context.borrow_mut().replace(pair);
            Box::new(move |_: &context::Context| {
                *gl_context.borrow_mut() = old_pair;
            })
        })
    }

    /// Resize the underlying surface.
    #[inline]
    pub fn resize(&self, new_size: (u32, u32)) {
        self.gl_context.borrow().as_ref().unwrap().resize(new_size)
    }

    #[inline]
    /// Set swap interval for the surface.
    pub fn set_swap_interval(&self, interval: SwapInterval) -> Result<(), glutin::error::Error> {
        self.gl_context
            .borrow()
            .as_ref()
            .unwrap()
            .set_swap_interval(interval)
    }

    /// Start drawing on the backbuffer.
    ///
    /// This function returns a `Frame`, which can be used to draw on it. When the `Frame` is
    /// destroyed, the buffers are swapped.
    ///
    /// Note that destroying a `Frame` is immediate, even if vsync is enabled.
    ///
    /// # Panic
    ///
    /// Panics if the previous `Frame` hasn't been finished: there can only be one at a time.
    /// `try_draw` returns an error instead.
    #[inline]
    pub fn draw(&self) -> Frame {
        self.try_draw()
            .expect("the previous `Frame` must be finished before drawing a new one")
    }

    /// Same as `draw`, but returns `FrameError::AlreadyDrawing` if the previous `Frame` hasn't
    /// been finished.
    #[inline]
    pub fn try_draw(&self) -> Result<Frame, FrameError> {
        let dimensions = self.get_framebuffer_dimensions();
        Frame::new(self.context.clone(), dimensions)
    }
}

impl fmt::Display for DisplayCreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            DisplayCreationError::GlutinError(err) => write!(fmt, "{}", err),
            DisplayCreationError::IncompatibleOpenGl(err) => write!(fmt, "{}", err),
        }
    }
}

impl Error for DisplayCreationError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            DisplayCreationError::GlutinError(ref err) => Some(err),
            DisplayCreationError::IncompatibleOpenGl(ref err) => Some(err),
        }
    }
}

impl From<glutin::error::Error> for DisplayCreationError {
    #[inline]
    fn from(err: glutin::error::Error) -> DisplayCreationError {
        DisplayCreationError::GlutinError(err)
    }
}

impl From<IncompatibleOpenGl> for DisplayCreationError {
    #[inline]
    fn from(err: IncompatibleOpenGl) -> DisplayCreationError {
        DisplayCreationError::IncompatibleOpenGl(err)
    }
}

impl<T: SurfaceTypeTrait + ResizeableSurface> Deref for Display<T> {
    type Target = Context;
    #[inline]
    fn deref(&self) -> &Context {
        &self.context
    }
}

impl<T: SurfaceTypeTrait + ResizeableSurface> backend::Facade for Display<T> {
    #[inline]
    fn get_context(&self) -> &Rc<Context> {
        &self.context
    }
}

impl<T: SurfaceTypeTrait + ResizeableSurface> Deref for GlutinBackend<T> {
    type Target = Rc<RefCell<Option<ContextSurfacePair<T>>>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl<T: SurfaceTypeTrait + ResizeableSurface> Backend for GlutinBackend<T> {
    #[inline]
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        match self.borrow().as_ref().unwrap().swap_buffers() {
            Ok(()) => Ok(()),
            _ => Err(SwapBuffersError::ContextLost),
        }
    }

    #[inline]
    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        let symbol = CString::new(symbol).unwrap();
        self.borrow()
            .as_ref()
            .unwrap()
            .display()
            .get_proc_address(&symbol) as *const _
    }

    #[inline]
    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        self.0
            .borrow()
            .as_ref()
            .unwrap()
            .get_framebuffer_dimensions()
    }

    #[inline]
    fn resize(&self, new_size: (u32, u32)) {
        self.borrow().as_ref().unwrap().resize(new_size)
    }

    #[inline]
    fn set_swap_interval(&self, interval: backend::SwapInterval) {
        let interval = match interval {
            backend::SwapInterval::DontWait => SwapInterval::DontWait,
            backend::SwapInterval::Wait(frames) => SwapInterval::Wait(frames),
        };
        self.borrow()
            .as_ref()
            .unwrap()
            .set_swap_interval(interval)
            .unwrap()
    }

    #[inline]
    fn is_current(&self) -> bool {
        self.borrow().as_ref().unwrap().is_current()
    }

    #[inline]
    unsafe fn make_current(&self) {
        let pair = self.borrow();
        pair.as_ref()
            .unwrap()
            .context
            .make_current(&pair.as_ref().unwrap().surface)
            .unwrap();
    }
}
