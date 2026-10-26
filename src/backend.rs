/*!

The `backend` module allows one to link between SafeGL and the OpenGL context..

There are three concepts in play:

 - The `Backend` trait describes the glue between SafeGL and the OpenGL context provider like
   glutin, SDL, GLFW, etc.
 - The `Context` struct is the main brick of SafeGL. It manages everything that SafeGL needs to
   execute OpenGL commands. Creating a `Context` requires a `Backend`.
 - The `Facade` trait. Calling functions like `VertexBuffer::new` requires passing an object
   that implements this trait. It is implemented on `Rc<Context>`.

*/
use std::num::NonZeroU32;
use std::ops::Deref;
#[cfg(not(all(target_family = "wasm", not(target_os = "emscripten"))))]
use std::os::raw::c_void;
use std::rc::Rc;

use crate::CapabilitiesSource;
use crate::SwapBuffersError;

use crate::context::Capabilities;
use crate::context::ExtensionsList;
use crate::version::Version;

pub use crate::context::Context;
pub use crate::context::ReleaseBehavior;

#[cfg(feature = "glutin")]
pub mod glutin;

#[cfg(all(feature = "webgl", target_family = "wasm", not(target_os = "emscripten")))]
pub mod web;

#[cfg(feature = "simple_window_builder")]
pub use winit;

/// How the backend should synchronize buffer swaps with the display's refresh rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapInterval {
    /// Swap buffers immediately, without waiting for the display (no vsync).
    DontWait,
    /// Wait for the given number of vertical blanks before swapping buffers.
    Wait(NonZeroU32),
}

/// Trait for types that can be used as a backend for a SafeGL context.
///
/// This trait is unsafe, as you can get undefined behaviors or crashes if you don't implement
/// the methods correctly.
pub unsafe trait Backend {
    /// Swaps buffers at the end of a frame.
    fn swap_buffers(&self) -> Result<(), SwapBuffersError>;

    /// Returns the address of an OpenGL function.
    ///
    /// Supposes that the context has been made current before this function is called.
    #[cfg(not(all(target_family = "wasm", not(target_os = "emscripten"))))]
    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void;

    /// Creates the `glow` context used to issue OpenGL calls.
    ///
    /// Called after the context has been made current, when SafeGL is created and when its
    /// context is rebuilt. The default implementation loads the functions with
    /// `get_proc_address`.
    #[cfg(not(all(target_family = "wasm", not(target_os = "emscripten"))))]
    unsafe fn create_glow_context(&self) -> glow::Context {
        glow::Context::from_loader_function(|symbol| self.get_proc_address(symbol))
    }

    /// Creates the `glow` context used to issue OpenGL calls.
    ///
    /// Called after the context has been made current, when SafeGL is created and when its
    /// context is rebuilt.
    #[cfg(all(target_family = "wasm", not(target_os = "emscripten")))]
    unsafe fn create_glow_context(&self) -> glow::Context;

    /// Returns the dimensions of the window, or screen, etc.
    fn get_framebuffer_dimensions(&self) -> (u32, u32);

    /// Resizes the underlying surface, should be called when the window's size has changed for example.
    fn resize(&self, new_size: (u32, u32));

    /// Set swap interval for the surface.
    fn set_swap_interval(&self, interval: SwapInterval);

    /// Returns true if the OpenGL context is the current one in the thread.
    fn is_current(&self) -> bool;

    /// Makes the OpenGL context the current context in the current thread.
    unsafe fn make_current(&self);

    /// Returns true if the backend knows that the OpenGL context has been lost, for example a
    /// browser's WebGL context. Backends that can't tell return `false`.
    fn is_context_lost(&self) -> bool {
        false
    }
}

unsafe impl<T> Backend for Rc<T>
where
    T: Backend,
{
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        self.deref().swap_buffers()
    }

    #[cfg(not(all(target_family = "wasm", not(target_os = "emscripten"))))]
    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        self.deref().get_proc_address(symbol)
    }

    unsafe fn create_glow_context(&self) -> glow::Context {
        self.deref().create_glow_context()
    }

    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        self.deref().get_framebuffer_dimensions()
    }

    fn resize(&self, new_size: (u32, u32)) {
        self.deref().resize(new_size);
    }

    fn set_swap_interval(&self, interval: SwapInterval) {
        self.deref().set_swap_interval(interval);
    }

    fn is_current(&self) -> bool {
        self.deref().is_current()
    }

    unsafe fn make_current(&self) {
        self.deref().make_current();
    }

    fn is_context_lost(&self) -> bool {
        self.deref().is_context_lost()
    }
}

/// Trait for types that provide a safe access for SafeGL functions.
pub trait Facade {
    /// Returns an opaque type that contains the OpenGL state, extensions, version, etc.
    fn get_context(&self) -> &Rc<Context>;
}

impl<T: ?Sized> CapabilitiesSource for T
where
    T: Facade,
{
    fn get_version(&self) -> &Version {
        self.get_context().deref().get_opengl_version()
    }

    fn get_extensions(&self) -> &ExtensionsList {
        self.get_context().deref().get_extensions()
    }

    fn get_capabilities(&self) -> &Capabilities {
        self.get_context().deref().get_capabilities()
    }
}

impl Facade for Rc<Context> {
    #[inline]
    fn get_context(&self) -> &Rc<Context> {
        self
    }
}
