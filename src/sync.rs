use crate::context::CommandContext;
use crate::gl;
use crate::handles;
use crate::version::is_webgl;
use glow::HasContext;

use crate::backend::Facade;
use crate::context::Context;
use crate::ContextExt;
use std::rc::Rc;

use std::thread;
use std::time::Duration;

/// Provides a way to wait for a server-side operation to be finished.
///
/// Creating a `SyncFence` injects an element in the commands queue of the backend.
/// When this element is reached, the fence becomes signaled.
///
/// ## Example
///
/// ```no_run
/// # use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
/// # fn example<T>(display: safegl::Display<T>) where T: SurfaceTypeTrait + ResizeableSurface {
/// # fn do_something<T>(_: &T) {}
/// let fence = safegl::SyncFence::new(&display);
/// do_something(&display);
/// fence.wait();   // blocks until the previous operations have finished
/// # }
/// ```
pub struct SyncFence {
    context: Rc<Context>,
    id: Option<glow::Fence>,
}

impl SyncFence {
    /// Builds a new `SyncFence` that is injected in the server.
    #[inline]
    pub fn new<F: ?Sized>(facade: &F) -> SyncFence
    where
        F: Facade,
    {
        let mut ctxt = facade.get_context().make_current();
        let fence = unsafe { new_linear_sync_fence(&mut ctxt) };
        match fence.into_sync_fence(facade) {
            Ok(fence) => fence,
            Err(_) => unreachable!("the fence was created by the context of the facade"),
        }
    }

    /// Returns true if the operations before the fence have finished on the server, without
    /// blocking.
    ///
    /// This flushes the command queue so that the fence eventually gets signaled.
    ///
    /// On WebGL the status of a fence is only updated between two tasks of the browser's event
    /// loop, so polling this function in a loop inside a single frame never sees it change.
    pub fn is_signaled(&self) -> bool {
        let sync = match self.id {
            Some(sync) => sync,
            None => return true,
        };

        let ctxt = self.context.make_current();
        let result = unsafe { ctxt.glow.client_wait_sync(sync, gl::SYNC_FLUSH_COMMANDS_BIT, 0) };
        matches!(result, gl::ALREADY_SIGNALED | gl::CONDITION_SATISFIED)
    }

    /// Blocks until the operations before the fence have finished on the server, or until
    /// `timeout` has passed, and tells which happened. The fence stays usable.
    ///
    /// This flushes the command queue so that the fence eventually gets signaled. WebGL limits
    /// the timeout to `GL_MAX_CLIENT_WAIT_TIMEOUT_WEBGL`, which is often 0.
    pub fn wait_timeout(&self, timeout: Duration) -> SyncWaitResult {
        let sync = match self.id {
            Some(sync) => sync,
            None => return SyncWaitResult::Signaled,
        };

        let ctxt = self.context.make_current();
        let timeout = u64::try_from(timeout.as_nanos()).unwrap_or(u64::MAX);
        let result = unsafe {
            if is_webgl() {
                let max = ctxt
                    .glow
                    .get_parameter_i64(gl::MAX_CLIENT_WAIT_TIMEOUT_WEBGL)
                    .max(0) as u64;
                let timeout = timeout.min(max).min(i32::MAX as u64) as i32;
                ctxt.glow
                    .client_wait_sync(sync, gl::SYNC_FLUSH_COMMANDS_BIT, timeout)
            } else {
                ctxt.extra.ClientWaitSync(
                    handles::raw_sync(sync),
                    gl::SYNC_FLUSH_COMMANDS_BIT,
                    timeout,
                )
            }
        };

        match result {
            gl::ALREADY_SIGNALED | gl::CONDITION_SATISFIED => SyncWaitResult::Signaled,
            gl::TIMEOUT_EXPIRED => SyncWaitResult::TimedOut,
            _ => SyncWaitResult::Failed,
        }
    }

    /// Makes the server wait for the fence before it runs the commands that follow, without
    /// blocking the client (`glWaitSync`).
    ///
    /// The commands of a context already run in order, so this only orders them after the
    /// operations before the fence in the way the driver sees them, such as the end of a
    /// transfer the driver runs asynchronously.
    pub fn wait_on_gpu(&self) {
        let sync = match self.id {
            Some(sync) => sync,
            None => return,
        };

        let ctxt = self.context.make_current();
        unsafe { ctxt.glow.wait_sync(sync, 0, gl::TIMEOUT_IGNORED) };
    }

    /// Blocks until the operation has finished on the server.
    ///
    /// WebGL can't block on a fence: if the fence isn't signaled yet, this calls `glFinish`
    /// instead, which waits for every command submitted so far, and the fence is treated as
    /// signaled.
    pub fn wait(mut self) {
        let sync = self.id.take().unwrap();

        let mut ctxt = self.context.make_current();
        let result = unsafe { client_wait(&mut ctxt, sync) };
        unsafe { delete_fence(&mut ctxt, sync) };

        match result {
            gl::ALREADY_SIGNALED | gl::CONDITION_SATISFIED => (),
            _ => panic!("Could not wait for the fence"),
        };
    }
}

/// What `SyncFence::wait_timeout` saw.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SyncWaitResult {
    /// The operations before the fence have finished.
    Signaled,
    /// The timeout passed before they finished.
    TimedOut,
    /// The wait failed, which happens when the context is lost.
    Failed,
}

#[cfg(any(safegl_gl43, safegl_gles32))]
impl crate::debug::sealed::Labeled for SyncFence {
    fn labeled_object(&self) -> (&Context, crate::debug::sealed::LabeledObject) {
        let sync = self.id.expect("the fence is only taken when it is consumed");
        (&self.context, crate::debug::sealed::LabeledObject::Sync(sync))
    }
}

impl Drop for SyncFence {
    #[inline]
    fn drop(&mut self) {
        let sync = match self.id {
            None => return, // fence has already been deleted
            Some(s) => s,
        };

        let mut ctxt = self.context.make_current();
        unsafe { delete_fence(&mut ctxt, sync) };
    }
}

/// Prototype for a `SyncFence`.
///
/// The fence must be consumed with either `into_sync_fence`, otherwise
/// the destructor will panic.
#[must_use]
pub struct LinearSyncFence {
    id: Option<glow::Fence>,
    // the address of the context that created the fence, which identifies it without making the
    // fence not `Send`
    context: usize,
}

unsafe impl Send for LinearSyncFence {}

impl LinearSyncFence {
    /// Turns the prototype into a real fence.
    ///
    /// Returns the prototype if `facade` isn't the context that created it: OpenGL objects can
    /// only be used with the context that created them.
    #[inline]
    pub fn into_sync_fence<F: ?Sized>(mut self, facade: &F) -> Result<SyncFence, LinearSyncFence>
    where
        F: Facade,
    {
        let context = facade.get_context();
        if &**context as *const Context as usize != self.context {
            return Err(self);
        }

        Ok(SyncFence {
            context: context.clone(),
            id: self.id.take(),
        })
    }
}

impl Drop for LinearSyncFence {
    #[inline]
    fn drop(&mut self) {
        if !thread::panicking() {
            assert!(self.id.is_none());
        }
    }
}

pub unsafe fn new_linear_sync_fence(ctxt: &mut CommandContext<'_>) -> LinearSyncFence {
    // sync objects are core in OpenGL 3.2 and OpenGL ES 3.0
    LinearSyncFence {
        context: ctxt.context as *const Context as usize,
        id: Some(
            ctxt.glow
                .fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0)
                .expect("glFenceSync failed"),
        ),
    }
}

/// Waits for this fence and destroys it, from within the commands context.
#[inline]
pub unsafe fn wait_linear_sync_fence_and_drop(
    mut fence: LinearSyncFence,
    ctxt: &mut CommandContext<'_>,
) {
    let fence = fence.id.take().unwrap();
    client_wait(ctxt, fence);
    delete_fence(ctxt, fence);
}

/// Destroys a fence, from within the commands context.
#[inline]
pub unsafe fn destroy_linear_sync_fence(ctxt: &mut CommandContext<'_>, mut fence: LinearSyncFence) {
    let fence = fence.id.take().unwrap();
    delete_fence(ctxt, fence);
}

/// Calls `glClientWaitSync` and returns the result.
///
/// Tries without flushing first, then with flushing.
///
/// WebGL only accepts a timeout of 0, so there it calls `glFinish` if the fence isn't signaled
/// yet and reports it as signaled.
///
/// # Unsafety
///
/// The fence object must exist.
///
unsafe fn client_wait(ctxt: &mut CommandContext<'_>, fence: glow::Fence) -> gl::types::GLenum {
    // trying without flushing first
    let result = ctxt.glow.client_wait_sync(fence, 0, 0);

    match result {
        val @ gl::ALREADY_SIGNALED | val @ gl::CONDITION_SATISFIED => return val,
        gl::TIMEOUT_EXPIRED => (),
        gl::WAIT_FAILED => (),
        _ => unreachable!(),
    };

    if is_webgl() {
        // `glFinish` waits for every command, including the ones before the fence
        ctxt.glow.finish();
        return gl::ALREADY_SIGNALED;
    }

    // waiting with a deadline of one year
    // the reason why the deadline is so long is because if you attach a GL debugger,
    // the wait can be blocked during a breaking point of the debugger
    ctxt.extra.ClientWaitSync(
        handles::raw_sync(fence),
        gl::SYNC_FLUSH_COMMANDS_BIT,
        365 * 24 * 3600 * 1000 * 1000 * 1000,
    )
}

/// Deletes a fence.
///
/// # Unsafety
///
/// The fence object must exist.
///
#[inline]
unsafe fn delete_fence(ctxt: &mut CommandContext<'_>, fence: glow::Fence) {
    ctxt.glow.delete_sync(fence);
}
