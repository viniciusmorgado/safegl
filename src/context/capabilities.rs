use crate::context::limits::{self, Limits};
use crate::context::ExtensionsList;
use crate::version::is_webgl;
use crate::version::Api;
use crate::version::Version;

use std::cmp;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use fnv::FnvHasher;
use glow::HasContext;

use crate::gl;
use crate::ToGlEnum;

use crate::image_format::TextureFormat;
use crate::CapabilitiesSource;

/// Represents the capabilities of the context.
///
/// Contrary to the state, these values never change.
#[derive(Debug)]
#[non_exhaustive]
pub struct Capabilities {
    /// List of versions of GLSL that are supported by the compiler.
    ///
    /// An empty list means that the backend doesn't have a compiler.
    pub supported_glsl_versions: Vec<Version>,

    /// Returns a version or release number. Vendor-specific information may follow the version
    /// number.
    pub version: String,

    /// The company responsible for this GL implementation.
    pub vendor: String,

    /// The name of the renderer. This name is typically specific to a particular
    /// configuration of a hardware platform.
    pub renderer: String,

    /// The context is in debug mode, which may have additional error and performance issue
    /// reporting functionality.
    pub debug: bool,

    /// The context is in "forward-compatible" mode, which means that no deprecated functionality
    /// will be supported.
    pub forward_compatible: bool,

    /// True if out-of-bound access on the GPU side can't result in crashes.
    pub robustness: bool,

    /// What the context does when the GPU is reset (`GL_RESET_NOTIFICATION_STRATEGY`):
    /// `LoseContextOnReset` means that the context can be lost.
    pub reset_notification: ResetNotificationStrategy,

    /// True if the context was created without error checking (`GL_CONTEXT_FLAG_NO_ERROR_BIT`,
    /// OpenGL 4.6 or `GL_KHR_no_error`): OpenGL errors are undefined behavior instead of being
    /// reported, and `glGetError` returns no error.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    pub no_error: bool,

    /// What happens when you change the current OpenGL context.
    pub release_behavior: ReleaseBehavior,

    /// Whether the context supports left and right buffers.
    pub stereo: bool,

    /// True if the default framebuffer is in sRGB.
    pub srgb: bool,

    /// Number of bits in the default framebuffer's depth buffer
    pub depth_bits: Option<u16>,

    /// Number of bits in the default framebuffer's stencil buffer
    pub stencil_bits: Option<u16>,

    /// Informations about formats when used to create textures.
    pub internal_formats_textures:
        HashMap<TextureFormat, FormatInfos, BuildHasherDefault<FnvHasher>>,

    /// Informations about formats when used to create renderbuffers.
    pub internal_formats_renderbuffers:
        HashMap<TextureFormat, FormatInfos, BuildHasherDefault<FnvHasher>>,

    /// Maximum number of textures that can be bound to a program.
    ///
    /// `glActiveTexture` must be between `GL_TEXTURE0` and `GL_TEXTURE0` + this value - 1.
    pub max_combined_texture_image_units: gl::types::GLint,

    /// Maximum value of the anisotropy of samplers (`GL_MAX_TEXTURE_MAX_ANISOTROPY`).
    ///
    /// `None` without OpenGL 4.6, `GL_ARB_texture_filter_anisotropic` or
    /// `GL_EXT_texture_filter_anisotropic`.
    pub max_texture_max_anisotropy: Option<gl::types::GLfloat>,

    /// Maximum size of a texture (i.e. GL_MAX_TEXTURE_SIZE)
    pub max_texture_size: gl::types::GLint,

    /// Maximum width, height and depth of a 3D texture (i.e. GL_MAX_3D_TEXTURE_SIZE)
    pub max_3d_texture_size: gl::types::GLint,

    /// Maximum number of layers of an array texture (i.e. GL_MAX_ARRAY_TEXTURE_LAYERS)
    pub max_array_texture_layers: gl::types::GLint,

    /// Maximum width and height of a cubemap face (i.e. GL_MAX_CUBE_MAP_TEXTURE_SIZE)
    pub max_cube_map_texture_size: gl::types::GLint,

    /// Maximum size of a buffer texture. `None` if this is not supported.
    pub max_texture_buffer_size: Option<gl::types::GLint>,

    /// Maximum width and height of `glViewport`.
    pub max_viewport_dims: (gl::types::GLint, gl::types::GLint),

    /// Maximum number of elements that can be passed with `glDrawBuffers`.
    pub max_draw_buffers: gl::types::GLint,

    /// Maximum number of vertices per patch. `None` if tessellation is not supported.
    pub max_patch_vertices: Option<gl::types::GLint>,

    /// Number of available buffer bind points for `GL_ATOMIC_COUNTER_BUFFER`.
    pub max_indexed_atomic_counter_buffer: gl::types::GLint,

    /// Number of available buffer bind points for `GL_SHADER_STORAGE_BUFFER`.
    pub max_indexed_shader_storage_buffer: gl::types::GLint,

    /// Number of available buffer bind points for `GL_TRANSFORM_FEEDBACK_BUFFER`.
    pub max_indexed_transform_feedback_buffer: gl::types::GLint,

    /// Number of available buffer bind points for `GL_UNIFORM_BUFFER`.
    pub max_indexed_uniform_buffer: gl::types::GLint,

    /// Alignment in bytes of the offsets of buffer ranges bound to `GL_UNIFORM_BUFFER`.
    pub uniform_buffer_offset_alignment: usize,

    /// Alignment in bytes of the offsets of buffer ranges bound to `GL_SHADER_STORAGE_BUFFER`.
    pub shader_storage_buffer_offset_alignment: usize,

    /// Number of work groups for compute shaders.
    pub max_compute_work_group_count: (gl::types::GLint, gl::types::GLint, gl::types::GLint),

    /// Maximum number of color attachment bind points.
    pub max_color_attachments: gl::types::GLint,

    /// Maximum width of an empty framebuffer. `None` if not supported.
    pub max_framebuffer_width: Option<gl::types::GLint>,

    /// Maximum height of an empty framebuffer. `None` if not supported.
    pub max_framebuffer_height: Option<gl::types::GLint>,

    /// Maximum layers of an empty framebuffer. `None` if not supported.
    pub max_framebuffer_layers: Option<gl::types::GLint>,

    /// Maximum samples of an empty framebuffer. `None` if not supported.
    pub max_framebuffer_samples: Option<gl::types::GLint>,

    /// Number of vertex buffer binding points of `glBindVertexBuffer`. `None` if separate vertex
    /// formats and bindings (OpenGL 4.3, OpenGL ES 3.1 or `GL_ARB_vertex_attrib_binding`) are
    /// not supported.
    pub max_vertex_attrib_bindings: Option<gl::types::GLint>,

    /// Maximum offset of an attribute in a vertex given to `glVertexAttribFormat`. `None` if
    /// separate vertex formats and bindings are not supported.
    pub max_vertex_attrib_relative_offset: Option<gl::types::GLint>,

    /// Maximum stride of the vertices of a buffer. `None` if the backend doesn't give it, which
    /// it does from OpenGL 4.4 and OpenGL ES 3.1.
    pub max_vertex_attrib_stride: Option<gl::types::GLint>,

    /// The other limits of the implementation.
    pub limits: Limits,
}

/// Information about an internal format.
#[derive(Debug)]
pub struct FormatInfos {
    /// Possible values for multisampling. `None` if unknown.
    pub multisamples: Option<Vec<gl::types::GLint>>,
}

/// What a context does when the GPU is reset (`GL_RESET_NOTIFICATION_STRATEGY`), which a context
/// is created with.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResetNotificationStrategy {
    /// The context isn't told about resets and isn't lost by them (`GL_NO_RESET_NOTIFICATION`).
    /// This is the strategy of contexts created without robustness.
    NoResetNotification,

    /// The context is lost when the GPU is reset (`GL_LOSE_CONTEXT_ON_RESET`):
    /// `Context::is_context_lost` tells it, and the context must be created again.
    LoseContextOnReset,
}

/// How a GPU reset that lost the context involved it, from `glGetGraphicsResetStatus`.
#[cfg(any(safegl_gl45, safegl_gles32))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl45, safegl_gles32))))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GraphicsResetStatus {
    /// The commands of this context caused the reset (`GL_GUILTY_CONTEXT_RESET`).
    Guilty,

    /// Another context caused the reset (`GL_INNOCENT_CONTEXT_RESET`).
    Innocent,

    /// The cause is unknown (`GL_UNKNOWN_CONTEXT_RESET`), or the backend told that the context
    /// was lost without a reset status, as WebGL and window systems do.
    Unknown,
}

/// Defines what happens when you change the current context.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReleaseBehavior {
    /// Nothing is done when using another context.
    None,

    /// The commands queue of the current context is flushed.
    Flush,
}

/// Loads the capabilities.
///
/// *Safety*: the OpenGL context corresponding to `gl` must be current in the thread, and `extra`
/// must be loaded from it.
///
/// ## Panic
///
/// Can panic if the version number or extensions list don't match the backend, leading to
/// unloaded functions being called.
///
pub unsafe fn get_capabilities(
    gl: &glow::Context,
    extra: &gl::Extra,
    version: &Version,
    extensions: &ExtensionsList,
) -> Capabilities {
    // OpenGL ES has `GL_CONTEXT_FLAGS` from version 3.2, without the forward-compatible flag
    let (debug, forward_compatible) =
        if version.0 == Api::Gl || version >= &Version(Api::GlEs, 3, 2) {
        let val = gl.get_parameter_i32(glow::CONTEXT_FLAGS) as gl::types::GLenum;
        (
            (val & glow::CONTEXT_FLAG_DEBUG_BIT) != 0,
            (val & glow::CONTEXT_FLAG_FORWARD_COMPATIBLE_BIT) != 0,
        )
    } else {
        (false, false)
    };

    // getting the value of `GL_RENDERER`
    let renderer = gl.get_parameter_string(glow::RENDERER);

    // separate vertex formats and bindings, which WebGL doesn't have
    let vertex_attrib_binding = !is_webgl()
        && (version >= &Version(Api::Gl, 4, 3)
            || version >= &Version(Api::GlEs, 3, 1)
            || extensions.gl_arb_vertex_attrib_binding);

    Capabilities {
        supported_glsl_versions: { get_supported_glsl(gl, version, extensions) },

        version: gl.get_parameter_string(glow::VERSION),

        vendor: gl.get_parameter_string(glow::VENDOR),

        debug,

        forward_compatible,

        robustness: if version >= &Version(Api::Gl, 4, 5)
            || version >= &Version(Api::GlEs, 3, 2)
            || (version.0 == Api::Gl && extensions.gl_arb_robustness)
        {
            let val = gl.get_parameter_i32(glow::CONTEXT_FLAGS) as gl::types::GLenum;
            (val & glow::CONTEXT_FLAG_ROBUST_ACCESS_BIT) != 0
        } else if extensions.gl_khr_robustness || extensions.gl_ext_robustness {
            gl.get_parameter_bool(gl::CONTEXT_ROBUST_ACCESS)
        } else {
            false
        },

        reset_notification: if version >= &Version(Api::Gl, 4, 5)
            || version >= &Version(Api::GlEs, 3, 2)
            || extensions.gl_khr_robustness
            || extensions.gl_arb_robustness
            || extensions.gl_ext_robustness
        {
            let val = gl.get_parameter_i32(glow::RESET_NOTIFICATION_STRATEGY);

            // AMD drivers return 0x31BE, which isn't an OpenGL value, and Adreno 430/506
            // drivers return `GL_NO_ERROR`: any value other than `GL_LOSE_CONTEXT_ON_RESET` means
            // that the context isn't lost
            match val as gl::types::GLenum {
                gl::LOSE_CONTEXT_ON_RESET => ResetNotificationStrategy::LoseContextOnReset,
                _ => ResetNotificationStrategy::NoResetNotification,
            }
        } else {
            ResetNotificationStrategy::NoResetNotification
        },

        #[cfg(safegl_gl46)]
        no_error: if version >= &Version(Api::Gl, 4, 6) || extensions.gl_khr_no_error {
            // OpenGL ES has `GL_CONTEXT_FLAGS` from version 3.2
            (version.0 == Api::Gl || version >= &Version(Api::GlEs, 3, 2))
                && (gl.get_parameter_i32(glow::CONTEXT_FLAGS) as gl::types::GLenum
                    & glow::CONTEXT_FLAG_NO_ERROR_BIT)
                    != 0
        } else {
            false
        },

        release_behavior: if extensions.gl_khr_context_flush_control {
            let val = gl.get_parameter_i32(glow::CONTEXT_RELEASE_BEHAVIOR);

            match val as gl::types::GLenum {
                gl::NONE => ReleaseBehavior::None,
                gl::CONTEXT_RELEASE_BEHAVIOR_FLUSH => ReleaseBehavior::Flush,
                _ => unreachable!(),
            }
        } else {
            ReleaseBehavior::Flush
        },

        // OpenGL ES has no stereo rendering
        stereo: version.0 == Api::Gl && gl.get_parameter_bool(glow::STEREO),

        srgb: {
            // the default framebuffer names its color buffers by side on desktop OpenGL
            let fb = gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING);
            let attachment = match (fb, version.0) {
                (0, Api::Gl) => gl::FRONT_LEFT,
                (0, Api::GlEs) => gl::BACK,
                _ => gl::COLOR_ATTACHMENT0,
            };
            let value = gl.get_framebuffer_attachment_parameter_i32(
                glow::FRAMEBUFFER,
                attachment,
                glow::FRAMEBUFFER_ATTACHMENT_COLOR_ENCODING,
            );
            value as gl::types::GLenum == gl::SRGB
        },

        depth_bits: {
            let value;

            // `GL_DEPTH_BITS` is removed from the core profile of desktop OpenGL, which queries
            // the attachment instead
            if version.0 == Api::Gl {
                let fb = gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING);
                let attachment = if fb == 0 {
                    gl::DEPTH
                } else {
                    gl::DEPTH_ATTACHMENT
                };
                let ty = gl.get_framebuffer_attachment_parameter_i32(
                    glow::FRAMEBUFFER,
                    attachment,
                    glow::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
                );

                if ty as gl::types::GLenum == gl::NONE {
                    value = 0;
                } else {
                    value = gl.get_framebuffer_attachment_parameter_i32(
                        glow::FRAMEBUFFER,
                        attachment,
                        glow::FRAMEBUFFER_ATTACHMENT_DEPTH_SIZE,
                    );
                }
            } else {
                value = gl.get_parameter_i32(glow::DEPTH_BITS);
            };

            match value {
                0 => None,
                v => Some(v as u16),
            }
        },

        stencil_bits: {
            let value;

            // `GL_STENCIL_BITS` is removed from the core profile of desktop OpenGL, which queries
            // the attachment instead
            if version.0 == Api::Gl {
                let fb = gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING);
                let attachment = if fb == 0 {
                    gl::STENCIL
                } else {
                    gl::STENCIL_ATTACHMENT
                };
                let ty = gl.get_framebuffer_attachment_parameter_i32(
                    glow::FRAMEBUFFER,
                    attachment,
                    glow::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
                );

                if ty as gl::types::GLenum == gl::NONE {
                    value = 0;
                } else {
                    value = gl.get_framebuffer_attachment_parameter_i32(
                        glow::FRAMEBUFFER,
                        attachment,
                        glow::FRAMEBUFFER_ATTACHMENT_STENCIL_SIZE,
                    );
                }
            } else {
                value = gl.get_parameter_i32(glow::STENCIL_BITS);
            };

            match value {
                0 => None,
                v => Some(v as u16),
            }
        },

        internal_formats_textures: get_internal_formats(gl, version, extensions, false),
        internal_formats_renderbuffers: get_internal_formats(gl, version, extensions, true),

        max_combined_texture_image_units: {
            let mut val = gl.get_parameter_i32(glow::MAX_COMBINED_TEXTURE_IMAGE_UNITS);

            // WORK-AROUND (issue #1181)
            // Some Radeon drivers crash if you use texture units 32 or more.
            if renderer.contains("Radeon") {
                val = cmp::min(val, 32);
            }

            val
        },

        // core in OpenGL 4.6, with the same values as the extensions
        max_texture_max_anisotropy: if version >= &Version(Api::Gl, 4, 6)
            || extensions.gl_arb_texture_filter_anisotropic
            || extensions.gl_ext_texture_filter_anisotropic
        {
            Some(gl.get_parameter_f32(glow::MAX_TEXTURE_MAX_ANISOTROPY_EXT))
        } else {
            None
        },

        max_texture_size: gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE),

        max_3d_texture_size: gl.get_parameter_i32(glow::MAX_3D_TEXTURE_SIZE),

        max_array_texture_layers: gl.get_parameter_i32(glow::MAX_ARRAY_TEXTURE_LAYERS),

        max_cube_map_texture_size: gl.get_parameter_i32(glow::MAX_CUBE_MAP_TEXTURE_SIZE),

        max_texture_buffer_size: {
            // core in OpenGL 3.1 and OpenGL ES 3.2
            if version.0 == Api::Gl
                || version >= &Version(Api::GlEs, 3, 2)
                || extensions.gl_oes_texture_buffer
                || extensions.gl_ext_texture_buffer
            {
                Some(gl.get_parameter_i32(glow::MAX_TEXTURE_BUFFER_SIZE))
            } else {
                None
            }
        },

        max_viewport_dims: {
            let mut val: [gl::types::GLint; 2] = [0, 0];
            gl.get_parameter_i32_slice(glow::MAX_VIEWPORT_DIMS, &mut val);
            (val[0], val[1])
        },

        max_draw_buffers: gl.get_parameter_i32(glow::MAX_DRAW_BUFFERS),

        // tessellation is core in OpenGL 4.0 and OpenGL ES 3.2
        max_patch_vertices: if version.0 == Api::Gl
            || version >= &Version(Api::GlEs, 3, 2)
            || extensions.gl_ext_tessellation_shader
            || extensions.gl_oes_tessellation_shader
        {
            Some(gl.get_parameter_i32(glow::MAX_PATCH_VERTICES))
        } else {
            None
        },

        max_indexed_atomic_counter_buffer: if version >= &Version(Api::Gl, 4, 2)
            || version >= &Version(Api::GlEs, 3, 1)
            || extensions.gl_arb_shader_atomic_counters
        {
            gl.get_parameter_i32(glow::MAX_ATOMIC_COUNTER_BUFFER_BINDINGS)
        } else {
            0
        },

        max_indexed_shader_storage_buffer: {
            if version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 1)
                || extensions.gl_arb_shader_storage_buffer_object
            {
                gl.get_parameter_i32(glow::MAX_SHADER_STORAGE_BUFFER_BINDINGS)
            } else {
                0
            }
        },

        max_indexed_transform_feedback_buffer: match version.0 {
            Api::Gl => gl.get_parameter_i32(glow::MAX_TRANSFORM_FEEDBACK_BUFFERS),
            // OpenGL ES binds one buffer per separate attribute
            Api::GlEs => gl.get_parameter_i32(glow::MAX_TRANSFORM_FEEDBACK_SEPARATE_ATTRIBS),
        },

        max_indexed_uniform_buffer: gl.get_parameter_i32(glow::MAX_UNIFORM_BUFFER_BINDINGS),

        uniform_buffer_offset_alignment: gl
            .get_parameter_i32(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT)
            .max(1) as usize,

        shader_storage_buffer_offset_alignment: {
            if version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 1)
                || extensions.gl_arb_shader_storage_buffer_object
            {
                gl.get_parameter_i32(glow::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT).max(1) as usize
            } else {
                1
            }
        },

        max_compute_work_group_count: if version >= &Version(Api::Gl, 4, 3)
            || version >= &Version(Api::GlEs, 3, 1)
            || extensions.gl_arb_compute_shader
        {
            let val1 = gl.get_parameter_indexed_i32(glow::MAX_COMPUTE_WORK_GROUP_COUNT, 0);
            let val2 = gl.get_parameter_indexed_i32(glow::MAX_COMPUTE_WORK_GROUP_COUNT, 1);
            let val3 = gl.get_parameter_indexed_i32(glow::MAX_COMPUTE_WORK_GROUP_COUNT, 2);
            (val1, val2, val3)
        } else {
            (0, 0, 0)
        },

        max_color_attachments: gl.get_parameter_i32(glow::MAX_COLOR_ATTACHMENTS),

        max_framebuffer_width: {
            if version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 1)
                || extensions.gl_arb_framebuffer_no_attachments
            {
                Some(gl.get_parameter_i32(glow::MAX_FRAMEBUFFER_WIDTH))
            } else {
                None
            }
        },

        max_framebuffer_height: {
            if version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 1)
                || extensions.gl_arb_framebuffer_no_attachments
            {
                Some(gl.get_parameter_i32(glow::MAX_FRAMEBUFFER_HEIGHT))
            } else {
                None
            }
        },

        max_framebuffer_layers: {
            if version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 2)
                || extensions.gl_arb_framebuffer_no_attachments
            {
                Some(gl.get_parameter_i32(glow::MAX_FRAMEBUFFER_LAYERS))
            } else {
                None
            }
        },

        max_framebuffer_samples: {
            if version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 1)
                || extensions.gl_arb_framebuffer_no_attachments
            {
                Some(gl.get_parameter_i32(glow::MAX_FRAMEBUFFER_SAMPLES))
            } else {
                None
            }
        },

        max_vertex_attrib_bindings: if vertex_attrib_binding {
            Some(gl.get_parameter_i32(glow::MAX_VERTEX_ATTRIB_BINDINGS))
        } else {
            None
        },

        max_vertex_attrib_relative_offset: if vertex_attrib_binding {
            Some(gl.get_parameter_i32(glow::MAX_VERTEX_ATTRIB_RELATIVE_OFFSET))
        } else {
            None
        },

        max_vertex_attrib_stride: if version >= &Version(Api::Gl, 4, 4)
            || version >= &Version(Api::GlEs, 3, 1)
        {
            Some(gl.get_parameter_i32(glow::MAX_VERTEX_ATTRIB_STRIDE))
        } else {
            None
        },

        limits: limits::get_limits(gl, extra, version, extensions),

        renderer,
    }
}

/// Gets the list of GLSL versions supported by the backend.
///
/// *Safety*: the OpenGL context corresponding to `gl` must be current in the thread.
///
/// ## Panic
///
/// Can panic if the version number or extensions list don't match the backend, leading to
/// unloaded functions being called.
///
pub unsafe fn get_supported_glsl(
    gl: &glow::Context,
    version: &Version,
    extensions: &ExtensionsList,
) -> Vec<Version> {
    // checking if the implementation has a shader compiler
    // a compiler is optional in OpenGL ES, but always present in WebGL, which can't be queried
    if version.0 == Api::GlEs && !is_webgl() && !gl.get_parameter_bool(glow::SHADER_COMPILER) {
        return vec![];
    }

    // some recent versions have an API to determine the list of supported versions
    if version >= &Version(Api::Gl, 4, 3) {
        // FIXME: implement this and return the result directly
    }

    let mut result = Vec::with_capacity(8);

    // GLSL ES 1.00 is accepted by OpenGL 4.1 through `GL_ARB_ES2_compatibility`
    if version.0 == Api::GlEs
        || version >= &Version(Api::Gl, 4, 1)
        || extensions.gl_arb_es2_compatibility
    {
        result.push(Version(Api::GlEs, 1, 0));
    }

    if version >= &Version(Api::GlEs, 3, 0)
        || version >= &Version(Api::Gl, 4, 3)
        || extensions.gl_arb_es3_compatibility
    {
        result.push(Version(Api::GlEs, 3, 0));
    }

    if version >= &Version(Api::GlEs, 3, 1)
        || version >= &Version(Api::Gl, 4, 5)
        || extensions.gl_arb_es3_1_compatibility
    {
        result.push(Version(Api::GlEs, 3, 1));
    }

    if version >= &Version(Api::GlEs, 3, 2) || extensions.gl_arb_es3_2_compatibility {
        result.push(Version(Api::GlEs, 3, 2));
    }

    // the core profile accepts GLSL 1.40 and above, but not GLSL 1.10 to 1.30
    for &(major, minor) in &[
        (1, 4),
        (1, 5),
        (3, 3),
        (4, 0),
        (4, 1),
        (4, 2),
        (4, 3),
        (4, 4),
        (4, 5),
        (4, 6),
    ] {
        if version >= &Version(Api::Gl, major, minor) {
            result.push(Version(Api::Gl, major, minor));
        }
    }

    result
}

/// Returns all informations about all supported internal formats.
pub fn get_internal_formats(
    gl: &glow::Context,
    version: &Version,
    extensions: &ExtensionsList,
    renderbuffer: bool,
) -> HashMap<TextureFormat, FormatInfos, BuildHasherDefault<FnvHasher>> {
    // We create a dummy object to implement the `CapabilitiesSource` trait.
    let dummy = {
        struct DummyCaps<'a>(&'a Version, &'a ExtensionsList);
        impl<'a> CapabilitiesSource for DummyCaps<'a> {
            fn get_version(&self) -> &Version {
                self.0
            }
            fn get_extensions(&self) -> &ExtensionsList {
                self.1
            }
            fn get_capabilities(&self) -> &Capabilities {
                unreachable!()
            }
        }
        DummyCaps(version, extensions)
    };

    TextureFormat::get_formats_list()
        .into_iter()
        .filter_map(|format| {
            if renderbuffer {
                if !format.is_supported_for_renderbuffers(&dummy) {
                    return None;
                }
            } else if !format.is_supported_for_textures(&dummy) {
                return None;
            }

            let infos = get_internal_format(gl, version, extensions, format, renderbuffer);
            Some((format, infos))
        })
        .collect()
}

/// Returns informations about a precise internal format.
pub fn get_internal_format(
    gl: &glow::Context,
    version: &Version,
    extensions: &ExtensionsList,
    format: TextureFormat,
    renderbuffer: bool,
) -> FormatInfos {
    // We create a dummy object to implement the `CapabilitiesSource` trait.
    let dummy = {
        struct DummyCaps<'a>(&'a Version, &'a ExtensionsList);
        impl<'a> CapabilitiesSource for DummyCaps<'a> {
            fn get_version(&self) -> &Version {
                self.0
            }
            fn get_extensions(&self) -> &ExtensionsList {
                self.1
            }
            fn get_capabilities(&self) -> &Capabilities {
                unreachable!()
            }
        }
        DummyCaps(version, extensions)
    };

    unsafe {
        let target = if renderbuffer {
            glow::RENDERBUFFER
        } else {
            glow::TEXTURE_2D_MULTISAMPLE
        };

        // WebGL only accepts `SAMPLES`, not `NUM_SAMPLE_COUNTS`, so the list can't be sized
        let samples = if format.is_renderable(&dummy)
            && !is_webgl()
            && ((version >= &Version(Api::GlEs, 3, 0) && renderbuffer)
                || version >= &Version(Api::Gl, 4, 2)
                || extensions.gl_arb_internalformat_query)
        {
            let mut num = [0];
            gl.get_internal_format_i32_slice(
                target,
                format.to_glenum(),
                glow::NUM_SAMPLE_COUNTS,
                &mut num,
            );
            let num = num[0];

            if num >= 1 {
                let mut formats = vec![0; num as usize];
                gl.get_internal_format_i32_slice(
                    target,
                    format.to_glenum(),
                    glow::SAMPLES,
                    &mut formats,
                );
                Some(formats)
            } else {
                Some(Vec::new())
            }
        } else {
            None
        };

        FormatInfos {
            multisamples: samples,
        }
    }
}
