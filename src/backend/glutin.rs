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

/// The version of the selected desktop level, if any.
fn desktop_level() -> Option<(u8, u8)> {
    let levels = [
        cfg!(safegl_gl40),
        cfg!(safegl_gl41),
        cfg!(safegl_gl42),
        cfg!(safegl_gl43),
        cfg!(safegl_gl44),
        cfg!(safegl_gl45),
        cfg!(safegl_gl46),
    ];
    levels
        .iter()
        .rposition(|&level| level)
        .map(|minor| (4, minor as u8))
}

/// The version of the selected OpenGL ES level, if any.
fn embedded_level() -> Option<(u8, u8)> {
    let levels = [
        cfg!(safegl_gles30),
        cfg!(safegl_gles31),
        cfg!(safegl_gles32),
    ];
    levels
        .iter()
        .rposition(|&level| level)
        .map(|minor| (3, minor as u8))
}

/// Returns the context APIs to request from glutin for the selected levels, in order: the
/// desktop level, then the OpenGL ES level. Context creation code should try them in order and
/// keep the first that succeeds.
///
/// Each one requests the version of its level, since the level is the version that the
/// compiled API is written for. glutin would otherwise request OpenGL 3.3, below SafeGL's
/// minimum.
pub fn level_context_apis() -> Vec<glutin::context::ContextApi> {
    use glutin::context::{ContextApi, Version};

    let mut apis = Vec::with_capacity(2);
    if let Some((major, minor)) = desktop_level() {
        apis.push(ContextApi::OpenGl(Some(Version::new(major, minor))));
    }
    if let Some((major, minor)) = embedded_level() {
        apis.push(ContextApi::Gles(Some(Version::new(major, minor))));
    }
    apis
}

/// The error message when the driver can't create a context of the selected levels.
fn level_context_error(macos: bool) -> String {
    level_context_error_for(desktop_level(), embedded_level(), macos)
}

/// The error message when the driver can't create a context of the `desktop` and `embedded`
/// levels.
fn level_context_error_for(
    desktop: Option<(u8, u8)>,
    embedded: Option<(u8, u8)>,
    macos: bool,
) -> String {
    let mut levels = Vec::new();
    if let Some((major, minor)) = desktop {
        levels.push(format!(
            "`gl{}{}` (OpenGL {}.{} core)",
            major, minor, major, minor
        ));
    }
    if let Some((major, minor)) = embedded {
        levels.push(format!(
            "`gles{}{}` (OpenGL ES {}.{})",
            major, minor, major, minor
        ));
    }
    let mut message = format!(
        "the driver can't create a context of the selected level {}",
        levels.join(" or ")
    );
    // macOS stops at OpenGL 4.1
    if macos && desktop.is_some_and(|version| version > (4, 1)) {
        message += ": macOS provides OpenGL 4.1 at most, select the `gl41` level or a lower one";
    }
    message
}

/// Returns the context attributes for `api`, which request the core profile for desktop OpenGL,
/// the only desktop profile that SafeGL accepts.
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

#[cfg(safegl_egl)]
pub mod headless;

/// Resizes a surface of glutin.
type ResizeSurface<T> =
    fn(&glutin::surface::Surface<T>, &PossiblyCurrentContext, NonZeroU32, NonZeroU32);

/// Wraps a glutin context together with the corresponding Surface.
/// This is necessary so that we can swap buffers and determine the framebuffer size within SafeGL.
pub struct ContextSurfacePair<T: SurfaceTypeTrait> {
    context: PossiblyCurrentContext,
    // `None` while the display has no surface (`Display::remove_surface`)
    surface: Option<glutin::surface::Surface<T>>,
    // resizes the surface, `None` for the surfaces of a fixed size (pbuffers)
    resize: Option<ResizeSurface<T>>,
    // the dimensions of the last surface, while the display has none
    removed_dimensions: (u32, u32),
}

impl<T: SurfaceTypeTrait + ResizeableSurface> ContextSurfacePair<T> {
    fn new(context: PossiblyCurrentContext, surface: glutin::surface::Surface<T>) -> Self {
        Self {
            context,
            surface: Some(surface),
            resize: Some(|surface, context, width, height| surface.resize(context, width, height)),
            removed_dimensions: (0, 0),
        }
    }
}

impl<T: SurfaceTypeTrait> ContextSurfacePair<T> {
    /// A pair whose surface keeps its size.
    fn fixed(context: PossiblyCurrentContext, surface: glutin::surface::Surface<T>) -> Self {
        Self {
            context,
            surface: Some(surface),
            resize: None,
            removed_dimensions: (0, 0),
        }
    }

    #[inline]
    /// Return the stored framebuffer dimensions, the ones of the last surface while the display
    /// has none.
    pub fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        match &self.surface {
            Some(surface) => (surface.width().unwrap(), surface.height().unwrap()),
            None => self.removed_dimensions,
        }
    }

    #[inline]
    /// Returns true while the display has a surface.
    pub fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    #[inline]
    /// Swaps the underlying back buffers when the surface is not single buffered. Without a
    /// surface, does nothing.
    pub fn swap_buffers(&self) -> Result<(), glutin::error::Error> {
        match &self.surface {
            Some(surface) => surface.swap_buffers(&self.context),
            None => Ok(()),
        }
    }

    #[inline]
    /// Set swap interval for the surface. Without a surface, does nothing.
    pub fn set_swap_interval(&self, interval: SwapInterval) -> Result<(), glutin::error::Error> {
        match &self.surface {
            Some(surface) => surface.set_swap_interval(&self.context, interval),
            None => Ok(()),
        }
    }

    /// Makes the context current, with its surface if it has one.
    fn make_current(&self) -> Result<(), glutin::error::Error> {
        match &self.surface {
            Some(surface) => self.context.make_current(surface),
            #[cfg(safegl_egl)]
            None => match &self.context {
                PossiblyCurrentContext::Egl(context) => context.make_current_surfaceless(),
                #[allow(unreachable_patterns)]
                _ => unreachable!("only EGL contexts lose their surface"),
            },
            #[cfg(not(safegl_egl))]
            None => unreachable!("only EGL contexts lose their surface"),
        }
    }

    #[inline]
    /// Resize the associated surface. Surfaces of a fixed size, like pbuffers, keep their size.
    pub fn resize(&self, new_size: (u32, u32)) {
        // Make sure that no dimension is zero, which happens when minimizing on Windows for example.
        let width = NonZeroU32::new(new_size.0).unwrap_or(NonZeroU32::new(1).unwrap());
        let height = NonZeroU32::new(new_size.1).unwrap_or(NonZeroU32::new(1).unwrap());
        if let (Some(resize), Some(surface)) = (self.resize, &self.surface) {
            resize(surface, &self.context, width, height);
        }
    }
}

impl<T: SurfaceTypeTrait> Deref for ContextSurfacePair<T> {
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
///
/// The surface is usually the one of a window (`WindowSurface`); a `Display<PbufferSurface>`
/// draws to a pbuffer of a fixed size instead, like the displays without window of
/// `headless`.
#[derive(Clone)]
pub struct Display<T: SurfaceTypeTrait + 'static> {
    // contains everything related to the current SafeGL context and its state
    context: Rc<context::Context>,
    // The glutin Surface alongside its associated glutin Context.
    gl_context: Rc<RefCell<Option<ContextSurfacePair<T>>>>,
}

/// An implementation of the `Backend` trait for glutin.
#[derive(Clone)]
pub struct GlutinBackend<T: SurfaceTypeTrait>(
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

impl<T: SurfaceTypeTrait> std::fmt::Debug for Display<T> {
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
        Display::from_pair(ContextSurfacePair::new(context, surface), debug, checked)
    }

    /// Resize the underlying surface.
    #[inline]
    pub fn resize(&self, new_size: (u32, u32)) {
        self.gl_context.borrow().as_ref().unwrap().resize(new_size)
    }
}

impl Display<glutin::surface::PbufferSurface> {
    /// Creates a SafeGL `Display` from a context and a pbuffer, a surface of a fixed size
    /// without window, with the given debug callback behavior. `headless` creates both.
    ///
    /// Performs a compatibility check to make sure that all core elements of SafeGL are supported
    /// by the implementation.
    pub fn from_pbuffer(
        context: PossiblyCurrentContext,
        surface: Surface<glutin::surface::PbufferSurface>,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<Self, IncompatibleOpenGl> {
        Display::from_pair(ContextSurfacePair::fixed(context, surface), debug, true)
    }
}

impl<T: SurfaceTypeTrait> Display<T> {
    fn from_pair(
        context_surface_pair: ContextSurfacePair<T>,
        debug: debug::DebugCallbackBehavior,
        checked: bool,
    ) -> Result<Self, IncompatibleOpenGl> {
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
        // the new surface resizes like the current one
        let resize = self.gl_context.borrow().as_ref().and_then(|pair| pair.resize);
        let pair = ContextSurfacePair {
            context,
            surface: Some(surface),
            resize,
            removed_dimensions: (0, 0),
        };
        let gl_context = &self.gl_context;
        self.context.rebuild_inner(move |_| {
            let old_pair = gl_context.borrow_mut().replace(pair);
            Box::new(move |_: &context::Context| {
                *gl_context.borrow_mut() = old_pair;
            })
        })
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
        if !self.gl_context.borrow().as_ref().unwrap().has_surface() {
            return Err(FrameError::SurfaceLost);
        }
        let dimensions = self.get_framebuffer_dimensions();
        Frame::new(self.context.clone(), dimensions)
    }

    /// Returns true while the display has a surface to draw frames on, which it doesn't between
    /// `remove_surface` and `set_surface`.
    #[inline]
    pub fn has_surface(&self) -> bool {
        self.gl_context.borrow().as_ref().unwrap().has_surface()
    }

    /// Removes the surface of the display and returns it, for the window systems that destroy
    /// the window of an application that keeps running, like Android when the application is
    /// paused (`Event::Suspended` of winit): the surface must be dropped before its window.
    ///
    /// The context and the objects of SafeGL live on, the context being current without
    /// surface (`EGL_KHR_surfaceless_context`): textures, buffers and programs can be created,
    /// drawn to as framebuffers and dropped, but frames can't be drawn
    /// (`FrameError::SurfaceLost`) until `set_surface` gives a new surface, of the new window.
    ///
    /// Returns `SurfaceRemovalError::NotSupported` for the contexts that aren't EGL, and for the
    /// EGL displays without `EGL_KHR_surfaceless_context`, and
    /// `SurfaceRemovalError::NoSurface` if the surface was removed already.
    pub fn remove_surface(&self) -> Result<Surface<T>, SurfaceRemovalError> {
        // the frame being drawn and the commands so far go to the surface before it is removed
        if self.has_surface() {
            self.context.flush();
        }
        let mut pair = self.gl_context.borrow_mut();
        let pair = pair.as_mut().unwrap();
        if pair.surface.is_none() {
            return Err(SurfaceRemovalError::NoSurface);
        }
        if !supports_surfaceless(&pair.context) {
            return Err(SurfaceRemovalError::NotSupported);
        }
        let dimensions = pair.get_framebuffer_dimensions();
        let surface = pair.surface.take().unwrap();
        pair.removed_dimensions = dimensions;
        if pair.make_current().is_err() {
            pair.surface = Some(surface);
            return Err(SurfaceRemovalError::NotSupported);
        }
        Ok(surface)
    }

    /// Gives the display a surface, after `remove_surface`, or instead of its surface, which is
    /// returned: it must be a surface of the configuration of the context, usually of a new
    /// window. The context is made current with it.
    pub fn set_surface(
        &self,
        surface: Surface<T>,
    ) -> Result<Option<Surface<T>>, glutin::error::Error> {
        let mut pair = self.gl_context.borrow_mut();
        let pair = pair.as_mut().unwrap();
        pair.context.make_current(&surface)?;
        Ok(pair.surface.replace(surface))
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

impl<T: SurfaceTypeTrait> Deref for Display<T> {
    type Target = Context;
    #[inline]
    fn deref(&self) -> &Context {
        &self.context
    }
}

impl<T: SurfaceTypeTrait> backend::Facade for Display<T> {
    #[inline]
    fn get_context(&self) -> &Rc<Context> {
        &self.context
    }
}

impl<T: SurfaceTypeTrait> Deref for GlutinBackend<T> {
    type Target = Rc<RefCell<Option<ContextSurfacePair<T>>>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl<T: SurfaceTypeTrait> Backend for GlutinBackend<T> {
    #[inline]
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        let pair = self.borrow();
        let pair = pair.as_ref().unwrap();
        if !pair.has_surface() {
            return Err(SwapBuffersError::SurfaceLost);
        }
        match pair.swap_buffers() {
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
        self.borrow().as_ref().unwrap().make_current().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::level_context_error_for;

    #[test]
    fn macos_above_its_version_is_explained() {
        assert_eq!(
            level_context_error_for(Some((4, 6)), None, true),
            "the driver can't create a context of the selected level `gl46` (OpenGL 4.6 core): \
             macOS provides OpenGL 4.1 at most, select the `gl41` level or a lower one"
        );
        // macOS has OpenGL 4.1, and the other systems any version
        assert!(!level_context_error_for(Some((4, 1)), None, true).contains("macOS"));
        assert!(!level_context_error_for(Some((4, 6)), None, false).contains("macOS"));
    }

    #[test]
    fn every_selected_level_is_named() {
        assert_eq!(
            level_context_error_for(Some((4, 3)), Some((3, 1)), false),
            "the driver can't create a context of the selected level `gl43` (OpenGL 4.3 core) or \
             `gles31` (OpenGL ES 3.1)"
        );
        assert_eq!(
            level_context_error_for(None, Some((3, 0)), false),
            "the driver can't create a context of the selected level `gles30` (OpenGL ES 3.0)"
        );
    }
}

/// Returns true if `context` can be made current without surface: an EGL context of a display
/// with `EGL_KHR_surfaceless_context`.
fn supports_surfaceless(context: &PossiblyCurrentContext) -> bool {
    // EGL is the only API of some platforms, like Android
    #[cfg(safegl_egl)]
    #[allow(irrefutable_let_patterns)]
    if let PossiblyCurrentContext::Egl(context) = context {
        use glutin::display::GetDisplayExtensions;
        return context
            .display()
            .extensions()
            .contains("EGL_KHR_surfaceless_context");
    }
    let _ = context;
    false
}

/// Error that can happen while removing the surface of a display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SurfaceRemovalError {
    /// The context can't be current without surface: it isn't an EGL context, or its display
    /// doesn't have `EGL_KHR_surfaceless_context`.
    NotSupported,
    /// The display has no surface already.
    NoSurface,
}

impl fmt::Display for SurfaceRemovalError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(match self {
            SurfaceRemovalError::NotSupported => "The context can't be current without surface",
            SurfaceRemovalError::NoSurface => "The display has no surface",
        })
    }
}

impl Error for SurfaceRemovalError {}
