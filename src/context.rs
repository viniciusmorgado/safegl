//! Contains everything related to the interface between SafeGL and the OpenGL implementation.

use crate::gl;

use std::any::Any;
use std::borrow::Cow;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::ffi::CStr;
use std::hash::BuildHasherDefault;
use std::marker::PhantomData;
use std::mem;
use std::os::raw;
use std::panic;
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::str;

use fnv::FnvHasher;
use glow::HasContext;

use crate::backend::Backend;
use crate::version;
use crate::version::is_webgl;
use crate::version::Api;
use crate::version::Version;
use crate::CapabilitiesSource;
use crate::ContextExt;
use crate::IncompatibleOpenGl;
use crate::SwapBuffersError;

use crate::debug;
use crate::fbo;
use crate::ops;
use crate::sampler_object;
use crate::texture;
use crate::uniforms;
use crate::vertex_array_object;

#[cfg(any(safegl_gl45, safegl_gles32))]
pub use self::capabilities::GraphicsResetStatus;
pub use self::capabilities::{Capabilities, ReleaseBehavior, ResetNotificationStrategy};
pub use self::extensions::ExtensionsList;
#[cfg(safegl_gl43)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl43)))]
pub use self::format_info::{FormatInfo, FormatInfoNotSupportedError, FormatSupport, FormatTarget};
#[cfg(any(safegl_gl43, safegl_gles32))]
pub use self::debug_output::DebugGroup;
#[cfg(any(safegl_gl43, safegl_gles32))]
pub(crate) use self::debug_output::{object_label, set_object_label};
pub use self::limits::{Limits, ProvokingVertexConvention, QueryCounterBits, StageLimits};
pub use self::state::GlState;
pub(crate) use self::state::RecordingTransformFeedback;

mod capabilities;
mod extensions;
#[cfg(safegl_gl43)]
mod format_info;
mod limits;
#[cfg(any(safegl_gl43, safegl_gles32))]
mod debug_output;
mod restore;
mod state;

/// Stores the state and information required for SafeGL to execute commands. Most public SafeGL
/// functions require passing a `Rc<Context>`.
pub struct Context {
    /// The glow context used to issue OpenGL calls.
    glow: Arc<glow::Context>,

    /// Contains the pointers to OpenGL functions.
    extra: gl::Extra,

    /// The current state of the OpenGL state machine. Contains for example which buffer is bound
    /// to which bind point, whether depth testing is activated, etc.
    state: RefCell<GlState>,

    /// Version of OpenGL of the backend.
    version: Version,

    /// Tells whether or not the backend supports each extension.
    extensions: ExtensionsList,

    /// Constants defined by the backend and retrieved at initialization. For example, number
    /// of texture units, maximum size of the viewport, etc.
    capabilities: Capabilities,

    /// Glue between SafeGL and the code that handles windowing. Contains functions that allows
    /// you to swap buffers, retrieve the size of the framebuffer, etc.
    backend: RefCell<Box<dyn Backend>>,

    /// Whether or not SafeGL must check that the OpenGL context is the current one before each
    /// call.
    check_current_context: bool,

    /// The callback that is used by the debug output feature.
    debug_callback: RefCell<Option<debug::DebugCallback>>,

    /// Whether a `Frame` is being drawn: there can only be one at a time, since they all draw
    /// to the back buffer of the same window.
    frame_in_progress: Cell<bool>,

    /// The payload of a panic of the debug callback, which can't unwind through OpenGL: it is
    /// resumed when the command that triggered the message returns.
    debug_callback_panic: Cell<Option<Box<dyn Any + Send>>>,

    /// Whether the debug messages are synchronous, to set up the debug output again when the
    /// OpenGL context changes.
    debug_output_synchronous: bool,

    /// Whether the callback only reports messages of medium and high severity, so that the
    /// driver doesn't need to generate the others.
    debug_output_errors_only: bool,

    /// Whether the debug messages go to the message log of the context instead of a callback.
    debug_output_log: bool,

    /// The number of debug groups pushed by `push_debug_group` and not popped yet.
    debug_group_depth: Cell<u32>,

    /// Incremented when the OpenGL context changes, which loses the debug groups and the objects
    /// that contexts don't share, such as transform feedback objects.
    generation: Cell<u64>,

    /// Whether or not errors triggered by ARB_debug_output (and similar extensions) should be
    /// reported to the user when `DebugCallbackBehavior::DebugMessageOnError` is used. This must
    /// be set to `false` in some situations, like compiling/linking shaders.
    report_debug_output_errors: Cell<bool>,

    /// We maintain a cache of FBOs.
    /// The `Option` is here in order to destroy the container. It must be filled at all time
    /// is a normal situation.
    framebuffer_objects: Option<fbo::FramebuffersContainer>,

    /// We maintain a list of vertex array objects.
    vertex_array_objects: vertex_array_object::VertexAttributesSystem,

    /// We maintain a list of samplers for each possible behavior.
    samplers: RefCell<
        HashMap<
            uniforms::SamplerBehavior,
            sampler_object::SamplerObject,
            BuildHasherDefault<FnvHasher>,
        >,
    >,

    /// The answers of `format_info`.
    #[cfg(safegl_gl43)]
    format_infos: RefCell<
        HashMap<(FormatTarget, crate::image_format::TextureFormat), FormatInfo>,
    >,

    /// List of texture handles that are resident. We need to call `MakeTextureHandleResidentARB`
    /// when rebuilding the context.
    resident_texture_handles: RefCell<Vec<ResidentHandle>>,

    /// List of images handles that are resident. We need to call `MakeImageHandleResidentARB`
    /// when rebuilding the context.
    resident_image_handles: RefCell<Vec<(gl::types::GLuint64, gl::types::GLenum)>>,
}

/// This struct is a guard that is returned when you want to access the OpenGL backend.
/// A bindless texture handle that a `ResidentTexture` made resident, with what draw commands
/// check when it is the value of a sampler uniform.
pub struct ResidentHandle {
    /// The handle.
    pub handle: gl::types::GLuint64,
    /// The types of the samplers that can sample the texture.
    pub sampler_types: &'static [crate::uniforms::UniformType],
    /// Whether the sampler of the handle compares depths.
    pub compares: bool,
    /// The draw call that last wrote the texture through an image unit.
    pub latest_shader_write: Rc<Cell<u64>>,
}

pub struct CommandContext<'a> {
    /// The context that is current.
    pub context: &'a Context,

    /// The glow context used to issue OpenGL calls.
    pub glow: &'a glow::Context,

    /// Source of OpenGL function pointers.
    pub extra: &'a gl::Extra,

    /// Refers to the state of the OpenGL backend. Maintained between multiple calls.
    /// **Must** be synchronized with the real state of the backend.
    pub state: RefMut<'a, GlState>,

    /// Version of the backend.
    pub version: &'a Version,

    /// Extensions supported by the backend.
    pub extensions: &'a ExtensionsList,

    /// Capabilities of the backend.
    pub capabilities: &'a Capabilities,

    /// Whether or not errors triggered by ARB_debug_output (and similar extensions) should be
    /// reported to the user (by panicking).
    pub report_debug_output_errors: &'a Cell<bool>,

    /// The list of vertex array objects.
    pub vertex_array_objects: &'a vertex_array_object::VertexAttributesSystem,

    /// The list of framebuffer objects.
    pub framebuffer_objects: &'a fbo::FramebuffersContainer,

    /// The list of samplers.
    pub samplers: RefMut<
        'a,
        HashMap<
            uniforms::SamplerBehavior,
            sampler_object::SamplerObject,
            BuildHasherDefault<FnvHasher>,
        >,
    >,

    /// List of texture handles that need to be made resident.
    pub resident_texture_handles: RefMut<'a, Vec<ResidentHandle>>,

    /// List of image handles and their access that need to be made resident.
    pub resident_image_handles: RefMut<'a, Vec<(gl::types::GLuint64, gl::types::GLenum)>>,

    /// This marker is here to prevent `CommandContext` from implementing `Send`
    // TODO: use this when possible
    //impl<'a, 'b> !Send for CommandContext<'a, 'b> {}
    marker: PhantomData<*mut u8>,
}

/// Calls `glPatchParameteri(GL_PATCH_VERTICES, ...)`, under the name of the version or the
/// extension that has it. Tessellation must be supported.
pub(crate) unsafe fn set_patch_vertices(
    gl: (&glow::Context, &gl::Extra),
    version: &Version,
    extensions: &ExtensionsList,
    value: gl::types::GLint,
) {
    if version.0 == Api::Gl || version >= &Version(Api::GlEs, 3, 2) {
        gl.0.patch_parameter_i32(glow::PATCH_VERTICES, value);
    } else if extensions.gl_oes_tessellation_shader {
        gl.1.PatchParameteriOES(gl::PATCH_VERTICES, value);
    } else {
        gl.1.PatchParameteriEXT(gl::PATCH_VERTICES, value);
    }
}

impl Context {
    /// Returns the number of times that the OpenGL context changed (`rebuild`): the objects
    /// that contexts don't share, created in a previous generation, are gone.
    #[inline]
    pub(crate) fn generation(&self) -> u64 {
        self.generation.get()
    }

    /// Marks the start of a `Frame`. Returns false if another one is in progress.
    pub(crate) fn begin_frame(&self) -> bool {
        !self.frame_in_progress.replace(true)
    }

    /// Marks the end of the current `Frame`.
    pub(crate) fn end_frame(&self) {
        self.frame_in_progress.set(false);
    }
}

impl<'a> Drop for CommandContext<'a> {
    fn drop(&mut self) {
        // the debug callback panicked during the commands
        if let Some(payload) = self.context.debug_callback_panic.take() {
            if !thread::panicking() {
                panic::resume_unwind(payload);
            }
        }
    }
}

impl<'a> CommandContext<'a> {
    /// Returns true if `context` is the current context: OpenGL objects can only be used with
    /// the context that created them.
    #[inline]
    pub fn owns(&self, context: &Context) -> bool {
        std::ptr::eq(self.context, context)
    }
}

impl Context {
    /// Builds a new context.
    ///
    /// The `check_current_context` parameter tells the context whether it should check
    /// if the backend's OpenGL context is the current one before each OpenGL operation.
    ///
    /// If you pass `false`, you must ensure that no other OpenGL context is going to be made
    /// current in the same thread as this context. Passing `true` makes things safe but
    /// is slightly slower.
    ///
    /// The OpenGL context must be newly-created. If you make modifications to the context before
    /// passing it to this function, SafeGL's state cache may mismatch the actual one.
    ///
    pub unsafe fn new<B>(
        backend: B,
        check_current_context: bool,
        callback_behavior: DebugCallbackBehavior,
    ) -> Result<Rc<Context>, IncompatibleOpenGl>
    where
        B: Backend + 'static,
    {
        backend.make_current();

        let glow = Arc::new(backend.create_glow_context());
        #[cfg(not(all(target_family = "wasm", not(target_os = "emscripten"))))]
        let extra = gl::Extra::load(|symbol| backend.get_proc_address(symbol));
        #[cfg(all(target_family = "wasm", not(target_os = "emscripten")))]
        let extra = gl::Extra::unloaded();
        let gl_state: RefCell<GlState> = RefCell::new(Default::default());

        let version = version::get_gl_version(&glow);
        let extensions = extensions::get_extensions(&glow);
        check_gl_compatibility(&glow, &version)?;

        let capabilities = capabilities::get_capabilities(&glow, &extra, &version, &extensions);
        let report_debug_output_errors = Cell::new(true);

        let vertex_array_objects = vertex_array_object::VertexAttributesSystem::new();
        let framebuffer_objects = fbo::FramebuffersContainer::new();
        let samplers = RefCell::new({
            let mut map = HashMap::with_hasher(Default::default());
            map.reserve(16);
            map
        });
        let resident_texture_handles = RefCell::new(Vec::new());
        let resident_image_handles = RefCell::new(Vec::new());

        let errors_only = matches!(callback_behavior, DebugCallbackBehavior::DebugMessageOnError);
        #[cfg(any(safegl_gl43, safegl_gles32))]
        let log = matches!(callback_behavior, DebugCallbackBehavior::Log);
        #[cfg(not(any(safegl_gl43, safegl_gles32)))]
        let log = false;
        let (debug_callback, synchronous) = match callback_behavior {
            DebugCallbackBehavior::Ignore => (None, false),
            // logged in the order of the commands
            #[cfg(any(safegl_gl43, safegl_gles32))]
            DebugCallbackBehavior::Log => (None, true),
            DebugCallbackBehavior::DebugMessageOnError => (
                Some(Box::new(default_debug_callback) as debug::DebugCallback),
                true,
            ),
            DebugCallbackBehavior::PrintAll => (
                Some(Box::new(printall_debug_callback) as debug::DebugCallback),
                false,
            ),
            DebugCallbackBehavior::Custom {
                callback,
                synchronous,
            } => (Some(callback), synchronous),
        };

        let context = Rc::new(Context {
            glow,
            extra,
            state: gl_state,
            version,
            extensions,
            capabilities,
            debug_callback: RefCell::new(debug_callback),
            frame_in_progress: Cell::new(false),
            debug_callback_panic: Cell::new(None),
            debug_output_synchronous: synchronous,
            debug_output_errors_only: errors_only,
            debug_output_log: log,
            debug_group_depth: Cell::new(0),
            generation: Cell::new(0),
            report_debug_output_errors,
            backend: RefCell::new(Box::new(backend)),
            check_current_context,
            framebuffer_objects: Some(framebuffer_objects),
            vertex_array_objects,
            samplers,
            #[cfg(safegl_gl43)]
            format_infos: RefCell::new(HashMap::new()),
            resident_texture_handles,
            resident_image_handles,
        });

        if context.debug_callback.borrow().is_some() || context.debug_output_log {
            init_debug_callback(&context);
        }

        // making sure that an error wasn't triggered during initialization
        {
            let mut ctxt = context.make_current();
            if crate::get_gl_error(&mut ctxt).is_some() {
                eprintln!(
                    "SafeGL has triggered an OpenGL error during initialization. Please report \
                           this error: https://github.com/viniciusmorgado/safegl/issues"
                );
            }
            /*assert!(::get_gl_error(&mut ctxt).is_none(),
            "SafeGL has triggered an OpenGL error during initialization. Please report \
             this error: https://github.com/viniciusmorgado/safegl/issues");*/
            init_state(&ctxt);
        }

        Ok(context)
    }

    /// Calls `get_framebuffer_dimensions` on the backend object stored by this context.
    #[inline]
    pub fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        self.backend.borrow().get_framebuffer_dimensions()
    }

    /// Changes the OpenGL context associated with this context.
    ///
    /// The new context **must** have lists shared with the old one. SafeGL keeps using the
    /// version, extensions and capabilities of the first context, so this returns an error, and
    /// keeps the old context, if the new one has another version or other extensions.
    pub unsafe fn rebuild<B>(&self, new_backend: B) -> Result<(), IncompatibleOpenGl>
    where
        B: Backend + 'static,
    {
        self.rebuild_inner(move |context| {
            let old_backend = context.backend.replace(Box::new(new_backend));
            Box::new(move |context: &Context| {
                context.backend.replace(old_backend);
            })
        })
    }

    /// Changes the OpenGL context associated with this context. `switch` changes it in the
    /// backend, and returns a function that changes it back if the new one isn't compatible.
    pub(crate) unsafe fn rebuild_inner<'a, F>(&self, switch: F) -> Result<(), IncompatibleOpenGl>
    where
        F: FnOnce(&Context) -> Box<dyn FnOnce(&Context) + 'a>,
    {
        {
            let mut ctxt = self.make_current();

            // framebuffer objects and vertex array objects aren't shared,
            // so we have to destroy them
            fbo::FramebuffersContainer::purge_all(&mut ctxt);
            vertex_array_object::VertexAttributesSystem::purge_all(&mut ctxt);

            // the old context could outlive this one and send messages to a dangling pointer
            if ctxt.state.enabled_debug_output == Some(true) {
                ctxt.glow.disable(glow::DEBUG_OUTPUT);
                ctxt.state.enabled_debug_output = Some(false);
            }
        }

        let switch_back = switch(self);
        self.backend.borrow().make_current();

        // glow reads the version and the extensions once, when it is created
        let new_glow = self.backend.borrow().create_glow_context();
        let version = version::get_gl_version(&new_glow);
        let extensions = extensions::get_extensions(&new_glow);
        let compatible = check_gl_compatibility(&new_glow, &version).and_then(|()| {
            if version == self.version && extensions == self.extensions {
                Ok(())
            } else {
                Err(IncompatibleOpenGl(format!(
                    "the new context provides OpenGL {:?} {}.{} and its extensions, instead of \
                     {:?} {}.{} and the extensions of the first context",
                    version.0, version.1, version.2, self.version.0, self.version.1, self.version.2
                )))
            }
        });
        if let Err(error) = compatible {
            switch_back(self);
            self.backend.borrow().make_current();
            if self.debug_callback.borrow().is_some() || self.debug_output_log {
                init_debug_callback(self);
            }
            return Err(error);
        }

        *self.state.borrow_mut() = Default::default();
        self.debug_group_depth.set(0);
        self.generation.set(self.generation.get() + 1);

        // the debug output and the other state are set up again in the new context
        if self.debug_callback.borrow().is_some() || self.debug_output_log {
            init_debug_callback(self);
        }
        init_state(&self.make_current());

        // making textures resident
        let textures = self.resident_texture_handles.borrow();
        for texture in textures.iter() {
            self.extra.MakeTextureHandleResidentARB(texture.handle);
        }

        // making images resident
        let images = self.resident_image_handles.borrow();
        for &(image, access) in images.iter() {
            self.extra.MakeImageHandleResidentARB(image, access);
        }

        Ok(())
    }

    /// Swaps the buffers in the backend.
    pub fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        if self.state.borrow().lost_context {
            return Err(SwapBuffersError::ContextLost);
        }

        // Note: This is a work-around for the FRAPS software.
        //       The Fraps software calls `glClear` with scissoring and reads the image of the
        //       current framebuffer.
        //       Therefore we need to bind the default framebuffer before swapping.
        if self.state.borrow().draw_framebuffer.is_some()
            || self.state.borrow().read_framebuffer.is_some()
        {
            let mut ctxt = self.make_current();

            unsafe {
                ctxt.glow.bind_framebuffer(glow::FRAMEBUFFER, None);
            }
            ctxt.state.draw_framebuffer = None;
            ctxt.state.read_framebuffer = None;
        }

        let backend = self.backend.borrow();
        if self.check_current_context && !backend.is_current() {
            unsafe { backend.make_current() };
        }

        // swapping
        let err = backend.swap_buffers();
        if let Err(SwapBuffersError::ContextLost) = err {
            let mut state = self.state.borrow_mut();
            state.lost_context = true;
            state.reset_status = gl::UNKNOWN_CONTEXT_RESET;
        }
        err
    }

    /// Returns the OpenGL version
    #[inline]
    #[deprecated(note = "use `get_opengl_version` instead.")]
    pub fn get_version(&self) -> &Version {
        &self.version
    }

    /// Returns the OpenGL version detected by this context.
    #[inline]
    pub fn get_opengl_version(&self) -> &Version {
        &self.version
    }

    /// Returns the GLSL version guaranteed to be supported.
    #[inline]
    pub fn get_supported_glsl_version(&self) -> Version {
        version::get_supported_glsl_version(self.get_opengl_version())
    }

    /// Returns true if the given GLSL version is supported.
    #[inline]
    pub fn is_glsl_version_supported(&self, version: &Version) -> bool {
        self.capabilities()
            .supported_glsl_versions
            .iter()
            .any(|v| v == version)
    }

    /// Returns a string containing this GL version or release number used by this context.
    ///
    /// Vendor-specific information may follow the version number.
    #[inline]
    pub fn get_opengl_version_string(&self) -> &str {
        &self.capabilities().version
    }

    /// Returns a string containing the company responsible for this GL implementation.
    #[inline]
    pub fn get_opengl_vendor_string(&self) -> &str {
        &self.capabilities().vendor
    }

    /// Returns a string containing the name of the GL renderer used by this context.
    ///
    /// This name is typically specific to a particular configuration of a hardware platform.
    #[inline]
    pub fn get_opengl_renderer_string(&self) -> &str {
        &self.capabilities().renderer
    }

    /// Returns true if the context is in debug mode.
    ///
    /// Debug mode may provide additional error and performance issue reporting functionality.
    #[inline]
    pub fn is_debug(&self) -> bool {
        self.capabilities().debug
    }

    /// Returns true if the context has the "forward-compatible" flag, which removes the
    /// deprecated functionality of desktop OpenGL.
    ///
    /// SafeGL behaves as if every desktop context were forward-compatible, whatever this flag:
    /// it never uses deprecated functionality, and refuses lines wider than 1 pixel.
    #[inline]
    pub fn is_forward_compatible(&self) -> bool {
        self.capabilities().forward_compatible
    }

    /// Returns true if out-of-bound buffer access from the GPU side (inside a program) cannot
    /// result in a crash.
    ///
    /// You should take extra care if `is_robust` returns false.
    #[inline]
    pub fn is_robust(&self) -> bool {
        self.capabilities().robustness
    }

    /// Returns true if a context loss is possible: the context was created with the
    /// `LoseContextOnReset` reset notification strategy.
    #[inline]
    pub fn is_context_loss_possible(&self) -> bool {
        self.capabilities().reset_notification == ResetNotificationStrategy::LoseContextOnReset
    }

    /// Returns true if the context was created without error checking (OpenGL 4.6 or
    /// `GL_KHR_no_error`), in which OpenGL errors are undefined behavior. SafeGL validates its
    /// calls itself, but the debug callback receives no error messages from such a context.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    #[inline]
    pub fn is_no_error(&self) -> bool {
        self.capabilities().no_error
    }

    /// Returns true if the context has been lost and needs to be recreated.
    ///
    /// # Implementation
    ///
    /// If it has been determined that the context has been lost before, then the function
    /// immediately returns true. Otherwise, asks the backend (which is how a WebGL context loss
    /// is detected), then calls `glGetGraphicsResetStatus`. If this function is not available,
    /// returns false.
    pub fn is_context_lost(&self) -> bool {
        self.reset_status() != gl::NO_ERROR
    }

    /// Returns how the GPU reset that lost the context involved it, or `None` if the context
    /// isn't lost, or if the backend can't tell resets (OpenGL 4.5, OpenGL ES 3.2,
    /// `GL_KHR_robustness`, `GL_ARB_robustness` or `GL_EXT_robustness`).
    ///
    /// A context created with the `NoResetNotification` strategy (see
    /// `Capabilities::reset_notification`) isn't told about resets, so this is always `None`.
    /// The status stays once the context is known to be lost, like `is_context_lost`.
    #[cfg(any(safegl_gl45, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl45, safegl_gles32))))]
    pub fn graphics_reset_status(&self) -> Option<GraphicsResetStatus> {
        match self.reset_status() {
            gl::NO_ERROR => None,
            gl::GUILTY_CONTEXT_RESET => Some(GraphicsResetStatus::Guilty),
            gl::INNOCENT_CONTEXT_RESET => Some(GraphicsResetStatus::Innocent),
            _ => Some(GraphicsResetStatus::Unknown),
        }
    }

    /// Returns the reset status that made the context lost, or `GL_NO_ERROR` if it isn't lost.
    ///
    /// If the context has been determined lost before, returns the status of then. Otherwise,
    /// asks the backend (which is how a WebGL context loss is detected), then calls
    /// `glGetGraphicsResetStatus` if the context has it.
    fn reset_status(&self) -> gl::types::GLenum {
        {
            let state = self.state.borrow();
            if state.lost_context {
                return state.reset_status;
            }
        }

        let lost_by_backend = self.backend.borrow().is_context_lost();

        let mut ctxt = self.make_current();

        let status = if lost_by_backend {
            gl::UNKNOWN_CONTEXT_RESET
        } else {
            query_graphics_reset_status(&ctxt).unwrap_or(gl::NO_ERROR)
        };

        if status != gl::NO_ERROR {
            ctxt.state.lost_context = true;
            ctxt.state.reset_status = status;
        }
        status
    }

    /// Returns the behavior when the current OpenGL context is changed.
    ///
    /// The most common value is `Flush`. In order to get `None` you must explicitly request it
    /// during creation.
    #[inline]
    pub fn get_release_behavior(&self) -> ReleaseBehavior {
        self.capabilities().release_behavior
    }

    /// Returns the range and the precision of the numbers of a precision qualifier in the vertex
    /// or fragment shader, or `None` for another stage or without OpenGL 4.1, OpenGL ES or
    /// `GL_ARB_ES2_compatibility`.
    ///
    /// Desktop OpenGL has the same precision for every qualifier: the qualifiers only matter to
    /// OpenGL ES, where `mediump` and `lowp` can be less precise than `highp`, which the fragment
    /// shaders of OpenGL ES 3.0 always have.
    #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))))]
    pub fn shader_precision_format(
        &self,
        shader: crate::program::ShaderType,
        qualifier: crate::program::PrecisionQualifier,
    ) -> Option<crate::program::ShaderPrecision> {
        use crate::program::ShaderType;

        if !matches!(shader, ShaderType::Vertex | ShaderType::Fragment) {
            return None;
        }
        let ctxt = self.make_current();
        if ctxt.version.0 != Api::GlEs
            && ctxt.version < &Version(Api::Gl, 4, 1)
            && !ctxt.extensions.gl_arb_es2_compatibility
        {
            return None;
        }

        let format = unsafe {
            ctxt.glow
                .get_shader_precision_format(shader.to_opengl_type(), qualifier.to_glenum())
        }?;
        Some(crate::program::ShaderPrecision {
            range: (format.range_min, format.range_max),
            precision: format.precision,
        })
    }

    /// Returns the time of the GPU in nanoseconds, when it has received every previous command
    /// but not necessarily run them (`GL_TIMESTAMP`), or `None` on OpenGL ES without
    /// `GL_EXT_disjoint_timer_query`. `debug::TimestampQuery` gives the time when the commands
    /// have run, and the two use the same clock.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub fn gpu_timestamp(&self) -> Option<u64> {
        let ctxt = self.make_current();
        if ctxt.version.0 != Api::Gl && !ctxt.extensions.gl_ext_disjoint_timer_query {
            return None;
        }
        Some(unsafe { ctxt.glow.get_parameter_i64(gl::TIMESTAMP) } as u64)
    }

    /// Returns the maximum value that can be used for anisotropic filtering, or `None`
    /// if the hardware doesn't support it.
    #[inline]
    pub fn get_max_anisotropy_support(&self) -> Option<u16> {
        self.capabilities()
            .max_texture_max_anisotropy
            .map(|v| v as u16)
    }

    /// Returns the maximum dimensions of the viewport.
    ///
    /// SafeGL will panic if you request a larger viewport than this when drawing.
    #[inline]
    pub fn get_max_viewport_dimensions(&self) -> (u32, u32) {
        let d = self.capabilities().max_viewport_dims;
        (d.0 as u32, d.1 as u32)
    }

    /// Releases the shader compiler, indicating that no new programs will be created for a while.
    ///
    /// This method is a no-op if it's not available in the implementation.
    pub fn release_shader_compiler(&self) {
        unsafe {
            let ctxt = self.make_current();

            // `glReleaseShaderCompiler` is core in OpenGL 4.1 and OpenGL ES 2.0, but doesn't
            // exist on WebGL
            let available = !is_webgl()
                && (ctxt.version.0 == Api::GlEs
                    || ctxt.version >= &Version(Api::Gl, 4, 1)
                    || ctxt.extensions.gl_arb_es2_compatibility);

            if available && !ctxt.capabilities.supported_glsl_versions.is_empty() {
                ctxt.extra.ReleaseShaderCompiler();
            }
        }
    }

    /// Returns an estimate of the amount of video memory available in bytes.
    ///
    /// Returns `None` if no estimate is available.
    pub fn get_free_video_memory(&self) -> Option<usize> {
        unsafe {
            let ctxt = self.make_current();

            let mut value: [gl::types::GLint; 4] = [0; 4];

            if ctxt.extensions.gl_nvx_gpu_memory_info {
                ctxt.glow.get_parameter_i32_slice(
                    gl::GPU_MEMORY_INFO_CURRENT_AVAILABLE_VIDMEM_NVX,
                    &mut value,
                );
                Some(value[0] as usize * 1024)
            } else if ctxt.extensions.gl_ati_meminfo {
                ctxt.glow
                    .get_parameter_i32_slice(gl::TEXTURE_FREE_MEMORY_ATI, &mut value);
                Some(value[0] as usize * 1024)
            } else {
                None
            }
        }
    }

    /// Reads the content of the front buffer.
    ///
    /// You will only see the data that has finished being drawn.
    ///
    /// OpenGL ES and WebGL can't read the front buffer, so this reads the back buffer instead.
    /// On WebGL, the back buffer is cleared after it is presented unless the context was
    /// created with `preserveDrawingBuffer`.
    ///
    /// This function can return any type that implements `Texture2dDataSink<(u8, u8, u8, u8)>`.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
    /// # fn example<T>(display: safegl::Display<T>) where T: SurfaceTypeTrait + ResizeableSurface {
    /// let pixels: Result<Vec<Vec<(u8, u8, u8, u8)>>, _> = display.read_front_buffer();
    /// # }
    /// ```
    pub fn read_front_buffer<T>(&self) -> Result<T, ops::ReadError>
    where
        T: texture::Texture2dDataSink<(u8, u8, u8, u8)>,
    {
        let mut ctxt = self.make_current();
        let dimensions = self.get_framebuffer_dimensions();
        let rect = crate::Rect {
            left: 0,
            bottom: 0,
            width: dimensions.0,
            height: dimensions.1,
        };

        let mut data = Vec::with_capacity(0);
        ops::read(
            &mut ctxt,
            ops::Source::DefaultFramebuffer(gl::FRONT_LEFT),
            &rect,
            &mut data,
            false,
        )?;
        Ok(T::from_raw(Cow::Owned(data), dimensions.0, dimensions.1))
    }

    /// Execute an arbitrary closure with the OpenGL context active. Useful if another
    /// component needs to directly manipulate OpenGL state.
    ///
    /// **If `action` manipulates any OpenGL state, it must be restored before `action`
    /// completes, or [`Context::restore_state`] must be called afterwards.**
    ///
    /// The context is current when `action` starts, and `action` can use SafeGL. SafeGL's vertex
    /// array object is unbound, so that the code can't change it by binding buffers.
    #[inline]
    pub unsafe fn exec_in_context<'a, T, F>(&self, action: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T + 'a,
    {
        {
            let mut ctxt = self.make_current();
            if ctxt.state.vertex_array.is_some() {
                ctxt.glow.bind_vertex_array(None);
                ctxt.state.vertex_array = None;
            }
        }
        let result = action();

        // the debug callback panicked on a message of the closure
        if let Some(payload) = self.debug_callback_panic.take() {
            panic::resume_unwind(payload);
        }
        result
    }

    /// Makes the OpenGL state match the state that SafeGL expects, after code outside of SafeGL
    /// changed it.
    ///
    /// SafeGL caches the OpenGL state (bound objects, blending, depth test, viewport, ...) to
    /// avoid redundant calls. Code that shares the context with SafeGL (a GUI toolkit, a video
    /// player, another renderer, raw calls through [`Context::glow_context`]) changes the state
    /// behind that cache. Call this function after such code ran and before using SafeGL again:
    ///
    /// ```no_run
    /// # fn example(display: &safegl::Display<safegl::glutin::surface::WindowSurface>) {
    /// unsafe {
    ///     display.exec_in_context(|| {
    ///         // ... OpenGL calls that change the state ...
    ///     });
    /// }
    /// display.restore_state();
    /// # }
    /// ```
    ///
    /// The other code must not delete objects that SafeGL created, and must not stop the queries,
    /// transform feedback or conditional rendering that SafeGL started, which aren't restored.
    pub fn restore_state(&self) {
        let mut ctxt = self.make_current();
        unsafe { restore::restore_state(&mut ctxt) };
    }

    /// Returns the `glow` context that SafeGL uses to issue OpenGL calls. Useful for interop
    /// with libraries built on `glow`, such as `egui_glow`.
    ///
    /// The context isn't made current by this function: issue calls from within
    /// `exec_in_context`, or otherwise make sure that this context is the current one.
    ///
    /// # Safety
    ///
    /// Same contract as `exec_in_context`: **if the caller manipulates any OpenGL state through
    /// the returned context, it must be restored before SafeGL is used again**, as SafeGL caches
    /// the state it has set.
    #[inline]
    pub unsafe fn glow_context(&self) -> &Arc<glow::Context> {
        &self.glow
    }

    /// Asserts that there are no OpenGL errors pending.
    ///
    /// This function should be used in tests.
    pub fn assert_no_error(&self, user_msg: Option<&str>) {
        let mut ctxt = self.make_current();

        match (crate::get_gl_error(&mut ctxt), user_msg) {
            (Some(msg), None) => panic!("{}", msg),
            (Some(msg), Some(user_msg)) => panic!("{} : {}", user_msg, msg),
            (None, _) => (),
        };
    }

    /// DEPRECATED. Renamed `finish`.
    #[inline]
    pub fn synchronize(&self) {
        self.finish();
    }

    /// Calls `glFinish()`. This waits until all the previously issued commands have finished
    /// being executed.
    ///
    /// When you execute OpenGL functions, they are not executed immediately. Instead they are
    /// put in a queue. This function flushes this queue, then waits until all commands
    /// have finished being executed.
    ///
    /// You normally don't need to call this function manually, except for debugging purposes.
    #[inline]
    pub fn finish(&self) {
        let ctxt = self.make_current();
        unsafe {
            ctxt.glow.finish();
        }
    }

    /// Calls `glFlush()`. This starts executing the commands that you have issued if it is not
    /// yet the case.
    ///
    /// When you execute OpenGL functions, they are not executed immediately. Instead they are
    /// put in a queue. This function flushes this queue so that commands start being executed.
    ///
    /// You normally don't need to call this function manually. Swapping buffers automatically
    /// flushes the queue. This function can be useful if you want to benchmark the time it
    /// takes from your OpenGL driver to process commands.
    #[inline]
    pub fn flush(&self) {
        let ctxt = self.make_current();
        unsafe {
            ctxt.glow.flush();
        }
    }

    /// Inserts a debugging string in the commands queue. If you use an OpenGL debugger, you will
    /// be able to see that string.
    ///
    /// This is helpful to understand where you are when you have big applications.
    ///
    /// Returns `Err` if the backend doesn't support this functionality. You can choose whether
    /// to call `.unwrap()` if you want to make sure that it works, or `.ok()` if you don't care.
    pub fn insert_debug_marker(&self, marker: &str) -> Result<(), ()> {
        let ctxt = self.make_current();

        // `glDebugMessageInsert` is core in OpenGL 4.3 and OpenGL ES 3.2, and glow calls
        // `glDebugMessageInsertKHR` on OpenGL ES with `GL_KHR_debug`
        if !is_webgl()
            && (ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 2)
                || ctxt.extensions.gl_khr_debug)
        {
            unsafe {
                ctxt.glow.debug_message_insert(
                    glow::DEBUG_SOURCE_APPLICATION,
                    glow::DEBUG_TYPE_MARKER,
                    0,
                    glow::DEBUG_SEVERITY_NOTIFICATION,
                    marker,
                )
            };
            Ok(())
        } else if ctxt.extensions.gl_ext_debug_marker {
            let marker = marker.as_bytes();
            unsafe {
                ctxt.extra.InsertEventMarkerEXT(
                    marker.len() as gl::types::GLsizei,
                    marker.as_ptr() as *const _,
                )
            };
            Ok(())
        } else {
            Err(())
        }
    }

    /// Same as `insert_debug_marker`, except that if you don't compile with `debug_assertions`
    /// it is a no-op and returns `Ok`.
    #[inline]
    pub fn debug_insert_debug_marker(&self, marker: &str) -> Result<(), ()> {
        if cfg!(debug_assertions) {
            self.insert_debug_marker(marker)
        } else {
            Ok(())
        }
    }
}

impl ContextExt for Context {
    #[inline]
    fn set_report_debug_output_errors(&self, value: bool) {
        self.report_debug_output_errors.set(value);
    }

    fn make_current(&self) -> CommandContext<'_> {
        if self.check_current_context {
            let backend = self.backend.borrow();
            if !backend.is_current() {
                // `is_current` isn't checked again here: some drivers (macOS since 10.14) don't
                // report the context as current right after it was made current
                unsafe { backend.make_current() };
            }
        }

        // SafeGL can't issue commands while it is already issuing some: from a debug callback
        // or a destructor that runs in the middle of a command
        let state = self.state.try_borrow_mut().expect(
            "SafeGL was called while it was already issuing OpenGL commands, for example from a \
             debug callback",
        );

        CommandContext {
            context: self,
            glow: &self.glow,
            extra: &self.extra,
            state,
            version: &self.version,
            extensions: &self.extensions,
            capabilities: &self.capabilities,
            report_debug_output_errors: &self.report_debug_output_errors,
            vertex_array_objects: &self.vertex_array_objects,
            framebuffer_objects: self.framebuffer_objects.as_ref().unwrap(),
            samplers: self.samplers.borrow_mut(),
            resident_texture_handles: self.resident_texture_handles.borrow_mut(),
            resident_image_handles: self.resident_image_handles.borrow_mut(),
            marker: PhantomData,
        }
    }

    #[inline]
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
}

impl CapabilitiesSource for Context {
    #[inline]
    fn get_version(&self) -> &Version {
        &self.version
    }

    #[inline]
    fn get_extensions(&self) -> &ExtensionsList {
        &self.extensions
    }

    #[inline]
    fn get_capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            // this is the code of make_current duplicated here because we can't borrow
            // `self` twice
            if self.check_current_context {
                let backend = self.backend.borrow();
                if !backend.is_current() {
                    backend.make_current();
                }
            }

            let mut ctxt = CommandContext {
                context: self,
                glow: &self.glow,
                extra: &self.extra,
                state: self.state.borrow_mut(),
                version: &self.version,
                extensions: &self.extensions,
                capabilities: &self.capabilities,
                report_debug_output_errors: &self.report_debug_output_errors,
                vertex_array_objects: &self.vertex_array_objects,
                framebuffer_objects: self.framebuffer_objects.as_ref().unwrap(),
                samplers: self.samplers.borrow_mut(),
                resident_texture_handles: self.resident_texture_handles.borrow_mut(),
                resident_image_handles: self.resident_image_handles.borrow_mut(),
                marker: PhantomData,
            };

            fbo::FramebuffersContainer::cleanup(&mut ctxt);
            vertex_array_object::VertexAttributesSystem::cleanup(&mut ctxt);

            for (_, s) in mem::replace(
                &mut *ctxt.samplers,
                HashMap::with_hasher(Default::default()),
            ) {
                s.destroy(&mut ctxt);
            }

            // disabling callback
            if ctxt.state.enabled_debug_output != Some(false) {
                if ctxt.version >= &Version(Api::Gl, 4, 3) || ctxt.extensions.gl_khr_debug {
                    ctxt.glow.disable(glow::DEBUG_OUTPUT);
                } else if ctxt.extensions.gl_arb_debug_output {
                    ctxt.extra.DebugMessageCallbackARB(None, ptr::null());
                }

                ctxt.state.enabled_debug_output = Some(false);
                ctxt.glow.finish();
            }
        }
    }
}

impl<'a> CapabilitiesSource for CommandContext<'a> {
    #[inline]
    fn get_version(&self) -> &Version {
        self.version
    }

    #[inline]
    fn get_extensions(&self) -> &ExtensionsList {
        self.extensions
    }

    #[inline]
    fn get_capabilities(&self) -> &Capabilities {
        self.capabilities
    }
}

/// Calls the `glGetGraphicsResetStatus` of the context, or returns `None` if it has none.
///
/// The function is core in OpenGL 4.5 and OpenGL ES 3.2, and desktop `GL_KHR_robustness` uses the
/// same name; OpenGL ES names it with a `KHR` suffix, and `GL_EXT_robustness` and
/// `GL_ARB_robustness` with their suffixes. WebGL has none: the backend is its only source.
fn query_graphics_reset_status(ctxt: &CommandContext<'_>) -> Option<gl::types::GLenum> {
    let version = ctxt.version;
    let extensions = ctxt.extensions;
    unsafe {
        if is_webgl() {
            None
        } else if version >= &Version(Api::Gl, 4, 5)
            || version >= &Version(Api::GlEs, 3, 2)
            || (version.0 == Api::Gl && extensions.gl_khr_robustness)
        {
            Some(ctxt.extra.GetGraphicsResetStatus())
        } else if extensions.gl_khr_robustness {
            Some(ctxt.extra.GetGraphicsResetStatusKHR())
        } else if extensions.gl_ext_robustness {
            Some(ctxt.extra.GetGraphicsResetStatusEXT())
        } else if extensions.gl_arb_robustness {
            Some(ctxt.extra.GetGraphicsResetStatusARB())
        } else {
            None
        }
    }
}

/// Minimum desktop OpenGL version supported by SafeGL, whatever the level.
const MIN_GL_VERSION: Version = Version(Api::Gl, 4, 0);
/// Minimum OpenGL ES version supported by SafeGL, whatever the level (also WebGL 2).
const MIN_GLES_VERSION: Version = Version(Api::GlEs, 3, 0);

/// Checks whether the backend supports SafeGL. Returns an `Err` if it doesn't.
fn check_gl_compatibility(gl: &glow::Context, version: &Version) -> Result<(), IncompatibleOpenGl> {
    let minimum = match version.0 {
        Api::Gl => MIN_GL_VERSION,
        Api::GlEs => MIN_GLES_VERSION,
    };

    if version < &minimum {
        return Err(IncompatibleOpenGl(format!(
            "SafeGL requires {:?} {}.{} or newer, but the driver provides {}.{}",
            minimum.0, minimum.1, minimum.2, version.1, version.2
        )));
    }

    // only the core profile of desktop OpenGL is supported
    if version.0 == Api::Gl {
        let mask = unsafe { gl.get_parameter_i32(glow::CONTEXT_PROFILE_MASK) } as u32;
        if mask & glow::CONTEXT_COMPATIBILITY_PROFILE_BIT != 0 {
            return Err(IncompatibleOpenGl(
                "SafeGL requires a core profile context, but the driver provides a \
                 compatibility profile context: request the core profile, for example with \
                 `ContextAttributesBuilder::with_profile(GlProfile::Core)`"
                    .to_owned(),
            ));
        }
    }

    Ok(())
}

/// Describes the behavior that the debug output should have.
///
/// The synchronous behaviors (`DebugMessageOnError`, the default in debug builds, and `Custom`
/// with `synchronous: true`) make the driver report each message during the call that caused
/// it, which disables the threaded optimizations of some drivers and costs some speed. Without
/// them, the NVIDIA driver runs OpenGL commands on a worker thread, which waits for the vertical
/// synchronization by spinning: a program that does little else shows a CPU core busy. Set the
/// `__GL_THREADED_OPTIMIZATIONS=0` environment variable, or `__GL_YIELD=USLEEP`, to avoid it.
///
/// The messages come from OpenGL 4.3, OpenGL ES 3.2 or `GL_KHR_debug`, which SafeGL turns on in
/// every context. Drivers below them that only have `GL_ARB_debug_output` may keep the messages
/// of contexts that aren't debug contexts, and nothing can turn them on there.
pub enum DebugCallbackBehavior {
    /// Don't do anything. This is the default behavior in release.
    Ignore,

    /// Print a message on stdout on error, except in some circumstances like when compiling
    /// shaders. This is the default behavior in debug mode.
    ///
    /// Only the messages of medium and high severity are printed, so the driver is told not to
    /// generate the others: some drivers format a notification for every buffer they create.
    DebugMessageOnError,

    /// Print every single output received by the driver.
    PrintAll,

    /// Don't call a callback, but keep the messages in the message log of the context, which
    /// `Context::debug_message_log` reads (OpenGL 4.3, OpenGL ES 3.2 or `GL_KHR_debug`). Every
    /// message is logged, synchronously.
    #[cfg(any(safegl_gl43, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles32))))]
    Log,

    /// Use a custom callback.
    Custom {
        /// The function to be called.
        callback: debug::DebugCallback,
        /// Whether or not it should be called immediately (true) or asynchronously (false).
        synchronous: bool,
    },
}

impl Default for DebugCallbackBehavior {
    #[inline]
    fn default() -> DebugCallbackBehavior {
        if cfg!(debug_assertions) {
            DebugCallbackBehavior::DebugMessageOnError
        } else {
            DebugCallbackBehavior::Ignore
        }
    }
}

/// The callback corresponding to `DebugMessageOnError`.
fn default_debug_callback(
    _: debug::Source,
    ty: debug::MessageType,
    severity: debug::Severity,
    _: u32,
    report_debug_output_errors: bool,
    message: &str,
) {
    match severity {
        debug::Severity::Medium => (),
        debug::Severity::High => (),
        _ => return,
    };

    match ty {
        debug::MessageType::Error => (),
        debug::MessageType::DeprecatedBehavior => (),
        debug::MessageType::UndefinedBehavior => (),
        debug::MessageType::Portability => (),
        _ => return,
    };

    if report_debug_output_errors {
        eprint!(
            "Debug message with high or medium severity: `{}`.\n\
                 Please report this error: https://github.com/viniciusmorgado/safegl/issues",
            message
        );

        #[cfg(feature = "backtrace")]
        print_backtrace();

        eprintln!("\n");
    }
}

/// Prints the current call stack to the standard error, after a debug message.
#[cfg(feature = "backtrace")]
fn print_backtrace() {
    eprint!("\nBacktrace:");

    let mut frame_id = 1;
    backtrace::trace(|frame| {
        let ip = frame.ip();
        eprint!("\n{:>#4} - {:p}", frame_id, ip);

        backtrace::resolve(ip, |symbol| {
            let name = symbol
                .name()
                .map(|n| n.as_str().unwrap_or("<not-utf8>"))
                .unwrap_or("<unknown>");
            let filename = symbol
                .filename()
                .map(|p| p.to_str().unwrap_or("<not-utf8>"))
                .unwrap_or("<unknown>");
            let line = symbol
                .lineno()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "??".to_owned());

            eprint!("\n         {} at {}:{}", name, filename, line);
        });

        frame_id += 1;
        true
    });
}

/// The callback corresponding to `DebugMessageOnError`.
fn printall_debug_callback(
    source: debug::Source,
    ty: debug::MessageType,
    severity: debug::Severity,
    id: u32,
    _: bool,
    message: &str,
) {
    eprintln!(
        "Source: {src:?}\t\tSeverity: {sev:?}\t\tType: {ty:?}\t\tId: {id}\n{msg}",
        src = source,
        sev = severity,
        ty = ty,
        id = id,
        msg = message
    );
}

/// Initializes `GL_KHR_debug`, `GL_ARB_debug`, or a similar extension so that the debug output
/// is reported.
/// Sets up the state that SafeGL expects in a new OpenGL context.
unsafe fn init_state(ctxt: &CommandContext<'_>) {
    // core in OpenGL 3.2, and always enabled on OpenGL ES
    if ctxt.version.0 == Api::Gl {
        ctxt.glow.enable(glow::TEXTURE_CUBE_MAP_SEAMLESS);
    }
}

/// Disables the debug messages of low severity and notifications, which the default callback
/// ignores: some drivers spend a lot of time formatting them, for every buffer for example.
unsafe fn disable_minor_debug_messages(
    control: impl Fn(gl::types::GLenum, gl::types::GLenum, gl::types::GLenum, gl::types::GLboolean),
    has_notifications: bool,
) {
    control(gl::DONT_CARE, gl::DONT_CARE, gl::DEBUG_SEVERITY_LOW, gl::FALSE);
    if has_notifications {
        control(gl::DONT_CARE, gl::DONT_CARE, gl::DEBUG_SEVERITY_NOTIFICATION, gl::FALSE);
    }
}

fn init_debug_callback(context: &Context) {
    let synchronous = context.debug_output_synchronous;
    let errors_only = context.debug_output_errors_only;

    // this is the C callback
    extern "system" fn callback_wrapper(
        source: gl::types::GLenum,
        ty: gl::types::GLenum,
        id: gl::types::GLuint,
        severity: gl::types::GLenum,
        length: gl::types::GLsizei,
        message: *const gl::types::GLchar,
        user_param: *mut raw::c_void,
    ) {
        // the user param is the context, which disables debug output and flushes in its
        // destructor, so that it outlives the messages
        let context: &Context = unsafe { &*(user_param as *const Context) };

        // the message has `length` bytes: drivers pass the messages of debug groups and of
        // inserted messages as they were given, without a null terminator
        let message = unsafe {
            let bytes = if length >= 0 {
                std::slice::from_raw_parts(message as *const u8, length as usize)
            } else {
                CStr::from_ptr(message).to_bytes()
            };
            String::from_utf8_lossy(bytes).into_owned()
        };

        // a message with a value outside of the specification is dropped
        let (severity, source, ty) = match (
            debug::Severity::from_glenum(severity),
            debug::Source::from_glenum(source),
            debug::MessageType::from_glenum(ty),
        ) {
            (Some(severity), Some(source), Some(ty)) => (severity, source, ty),
            _ => return,
        };

        // a callback that triggers messages itself isn't called again for them
        let mut callback = match context.debug_callback.try_borrow_mut() {
            Ok(callback) => callback,
            Err(_) => return,
        };
        if let Some(callback) = callback.as_mut() {
            // a panic can't unwind through OpenGL: it is resumed when the command returns
            let report_errors = context.report_debug_output_errors.get();
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                callback(source, ty, severity, id, report_errors, &message)
            }));
            if let Err(payload) = result {
                context.debug_callback_panic.set(Some(payload));
            }
        }
    }

    struct ContextRawPtr(*const Context);
    unsafe impl Send for ContextRawPtr {}
    let context_raw_ptr = ContextRawPtr(context);

    // without a callback, the messages go to the message log
    let callback: gl::types::GLDEBUGPROC = if context.debug_output_log {
        None
    } else {
        Some(callback_wrapper)
    };

    unsafe {
        let mut ctxt = context.make_current();

        if ctxt.version >= &Version(Api::Gl, 4, 3)
            || ctxt.version >= &Version(Api::GlEs, 3, 2)
            || ctxt.extensions.gl_khr_debug
            || ctxt.extensions.gl_arb_debug_output
        {
            if synchronous && !ctxt.state.enabled_debug_output_synchronous {
                ctxt.glow.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS);
                ctxt.state.enabled_debug_output_synchronous = true;
            }

            if ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 2)
                || (ctxt.version.0 == Api::Gl && ctxt.extensions.gl_khr_debug)
            {
                ctxt.extra
                    .DebugMessageCallback(callback, context_raw_ptr.0 as *const _);
                ctxt.extra.DebugMessageControl(
                    gl::DONT_CARE,
                    gl::DONT_CARE,
                    gl::DONT_CARE,
                    0,
                    ptr::null(),
                    gl::TRUE,
                );
                if errors_only {
                    disable_minor_debug_messages(
                        |source, ty, severity, enabled| {
                            ctxt.extra
                                .DebugMessageControl(source, ty, severity, 0, ptr::null(), enabled)
                        },
                        true,
                    );
                }

                if ctxt.state.enabled_debug_output != Some(true) {
                    ctxt.glow.enable(glow::DEBUG_OUTPUT);
                    ctxt.state.enabled_debug_output = Some(true);
                }
            } else if ctxt.extensions.gl_khr_debug {
                ctxt.extra
                    .DebugMessageCallbackKHR(callback, context_raw_ptr.0 as *const _);
                ctxt.extra.DebugMessageControlKHR(
                    gl::DONT_CARE,
                    gl::DONT_CARE,
                    gl::DONT_CARE,
                    0,
                    ptr::null(),
                    gl::TRUE,
                );
                if errors_only {
                    disable_minor_debug_messages(
                        |source, ty, severity, enabled| {
                            ctxt.extra
                                .DebugMessageControlKHR(source, ty, severity, 0, ptr::null(), enabled)
                        },
                        true,
                    );
                }

                if ctxt.state.enabled_debug_output != Some(true) {
                    ctxt.glow.enable(glow::DEBUG_OUTPUT);
                    ctxt.state.enabled_debug_output = Some(true);
                }
            } else {
                ctxt.extra
                    .DebugMessageCallbackARB(callback, context_raw_ptr.0 as *const _);
                ctxt.extra.DebugMessageControlARB(
                    gl::DONT_CARE,
                    gl::DONT_CARE,
                    gl::DONT_CARE,
                    0,
                    ptr::null(),
                    gl::TRUE,
                );
                if errors_only {
                    disable_minor_debug_messages(
                        |source, ty, severity, enabled| {
                            ctxt.extra
                                .DebugMessageControlARB(source, ty, severity, 0, ptr::null(), enabled)
                        },
                        false,
                    );
                }

                ctxt.state.enabled_debug_output = Some(true);
            }
        }
    }
}
