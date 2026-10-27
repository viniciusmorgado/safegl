use crate::gl;
use smallvec::SmallVec;

/// Represents the current OpenGL state.
///
/// The current state is passed to each function and can be freely updated.
pub struct GlState {
    /// Whether we have detected that the context has been lost.
    ///
    /// Even when this is `false`, the context may have been lost since the last query. So we have
    /// to check for lost context as long as this is false.
    pub lost_context: bool,

    /// The `glGetGraphicsResetStatus` value that told that the context was lost, or
    /// `GL_UNKNOWN_CONTEXT_RESET` when something else told it.
    pub reset_status: gl::types::GLenum,

    /// Whether GL_BLEND is enabled
    pub enabled_blend: bool,

    /// Whether GL_CULL_FACE is enabled
    pub enabled_cull_face: bool,

    /// Whether GL_DEBUG_OUTPUT is enabled. None means "unknown".
    pub enabled_debug_output: Option<bool>,

    /// Whether GL_DEBUG_OUTPUT_SYNCHRONOUS is enabled
    pub enabled_debug_output_synchronous: bool,

    /// Whether GL_DEPTH_TEST is enabled
    pub enabled_depth_test: bool,

    /// Whether DEPTH_CLAMP_NEAR is enabled.
    pub enabled_depth_clamp_near: bool,

    /// Whether DEPTH_CLAMP_FAR is enabled.
    pub enabled_depth_clamp_far: bool,

    /// Whether GL_DITHER is enabled
    pub enabled_dither: bool,

    /// Whether GL_FRAMEBUFFER_SRGB is enabled
    pub enabled_framebuffer_srgb: bool,

    /// Whether GL_MULTISAMPLE is enabled
    pub enabled_multisample: bool,

    /// Whether GL_COLOR_LOGIC_OP is enabled
    pub enabled_color_logic_op: bool,

    /// Whether GL_POLYGON_OFFSET_FILL is enabled
    pub enabled_polygon_offset_fill: bool,

    /// Whether GL_POLYGON_OFFSET_LINE is enabled
    pub enabled_polygon_offset_line: bool,

    /// Whether GL_POLYGON_OFFSET_POINT is enabled
    pub enabled_polygon_offset_point: bool,

    /// Whether GL_PRIMITIVE_RESTART_FIXED_INDEX is enabled
    ///
    /// Always true on WebGL 2, which can't disable it.
    pub enabled_primitive_fixed_restart: bool,

    /// Whether GL_PRIMITIVE_RESTART is enabled.
    pub enabled_primitive_restart: bool,

    /// The value of `glPrimitiveRestartIndex`.
    pub primitive_restart_index: u32,

    /// The value of `GL_PATCH_DEFAULT_OUTER_LEVEL`.
    pub patch_default_outer_level: [f32; 4],

    /// The value of `GL_PATCH_DEFAULT_INNER_LEVEL`.
    pub patch_default_inner_level: [f32; 2],

    /// The current values of the vertex attributes set by `glVertexAttrib*`, by location, or
    /// `None` for those that SafeGL never set.
    pub vertex_attrib_values: Vec<Option<crate::vertex::AttributeValue>>,

    /// Whether GL_RASTERIZER_DISCARD is enabled
    pub enabled_rasterizer_discard: bool,

    /// Whether GL_SAMPLE_ALPHA_TO_COVERAGE is enabled
    pub enabled_sample_alpha_to_coverage: bool,

    /// Whether GL_SAMPLE_COVERAGE is enabled
    pub enabled_sample_coverage: bool,

    /// The latest values passed to `glSampleCoverage`.
    pub sample_coverage: (gl::types::GLfloat, bool),

    /// Whether GL_SAMPLE_ALPHA_TO_ONE is enabled
    pub enabled_sample_alpha_to_one: bool,

    /// Whether GL_SAMPLE_MASK is enabled
    pub enabled_sample_mask: bool,

    /// Whether GL_SAMPLE_SHADING is enabled
    pub enabled_sample_shading: bool,

    /// Whether GL_SCISSOR_TEST is enabled
    pub enabled_scissor_test: bool,

    /// Whether GL_STENCIL_TEST is enabled
    pub enabled_stencil_test: bool,

    /// Whether GL_LINE_SMOOTH is enabled
    pub enabled_line_smooth: bool,

    /// Whether GL_POLYGON_SMOOTH is enabled
    pub enabled_polygon_smooth: bool,

    /// Whether GL_PROGRAM_POINT_SIZE is enabled
    pub enabled_program_point_size: bool,

    /// A bitmask containing the currently enabled clip planes.
    pub enabled_clip_planes: gl::types::GLuint,

    /// The latest value passed to `glUseProgram`.
    pub program: Option<glow::Program>,

    /// The latest value passed to `glBindVertexArray`.
    pub vertex_array: Option<glow::VertexArray>,

    /// The latest value passed to `glClearColor`.
    pub clear_color: (
        gl::types::GLclampf,
        gl::types::GLclampf,
        gl::types::GLclampf,
        gl::types::GLclampf,
    ),

    /// The latest value passed to `glClearDepthf`.
    pub clear_depth: gl::types::GLclampf,

    /// The latest value passed to `glClearStencil`.
    pub clear_stencil: gl::types::GLint,

    /// The latest values passed to ``glColorMask`.
    pub color_mask: (
        gl::types::GLboolean,
        gl::types::GLboolean,
        gl::types::GLboolean,
        gl::types::GLboolean,
    ),

    /// Whether `glColorMaski` may have given some draw buffers a mask other than `color_mask`.
    pub indexed_color_mask: bool,

    /// Whether `glEnablei`, `glBlendEquationSeparatei` or `glBlendFuncSeparatei` may have given
    /// some draw buffers a blending state other than `enabled_blend`, `blend_equation` and
    /// `blend_func`.
    pub indexed_blend: bool,

    /// The latest buffer bound to `GL_ARRAY_BUFFER`.
    pub array_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_PIXEL_PACK_BUFFER`.
    pub pixel_pack_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_PIXEL_UNPACK_BUFFER`.
    pub pixel_unpack_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_UNIFORM_BUFFER`.
    pub uniform_buffer_binding: Option<glow::Buffer>,

    /// List of buffers bound to the indexed `GL_UNIFORM_BUFFER`.
    pub indexed_uniform_buffer_bindings: SmallVec<[IndexedBufferState; 8]>,

    /// The latest buffer bound to `GL_COPY_READ_BUFFER`.
    pub copy_read_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_COPY_WRITE_BUFFER`.
    pub copy_write_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_DISPATCH_INDIRECT_BUFFER`.
    pub dispatch_indirect_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_DRAW_INDIRECT_BUFFER`.
    pub draw_indirect_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_QUERY_BUFFER`.
    pub query_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_PARAMETER_BUFFER`.
    pub parameter_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_TEXTURE_BUFFER`.
    pub texture_buffer_binding: Option<glow::Buffer>,

    /// The latest buffer bound to `GL_ATOMIC_COUNTER_BUFFER`.
    pub atomic_counter_buffer_binding: Option<glow::Buffer>,

    /// List of buffers bound to the indexed `GL_ATOMIC_COUNTER_BUFFER`.
    pub indexed_atomic_counter_buffer_bindings: SmallVec<[IndexedBufferState; 8]>,

    /// The latest buffer bound to `GL_SHADER_STORAGE_BUFFER`.
    pub shader_storage_buffer_binding: Option<glow::Buffer>,

    /// List of buffers bound to the indexed `GL_SHADER_STORAGE_BUFFER`.
    pub indexed_shader_storage_buffer_bindings: SmallVec<[IndexedBufferState; 8]>,

    /// The latest buffer bound to the generic `GL_TRANSFORM_FEEDBACK_BUFFER`.
    pub transform_feedback_buffer_binding: Option<glow::Buffer>,

    /// List of buffers bound to the indexed `GL_TRANSFORM_FEEDBACK_BUFFER`.
    pub indexed_transform_feedback_buffer_bindings: SmallVec<[IndexedBufferState; 4]>,

    /// The latest buffer bound to `GL_READ_FRAMEBUFFER`.
    pub read_framebuffer: Option<glow::Framebuffer>,

    /// The latest buffer bound to `GL_DRAW_FRAMEBUFFER`.
    pub draw_framebuffer: Option<glow::Framebuffer>,

    /// The latest values passed to `glReadBuffer` with the default framebuffer.
    /// `None` means "unknown".
    pub default_framebuffer_read: Option<gl::types::GLenum>,

    /// The latest render buffer bound with `glBindRenderbuffer`.
    pub renderbuffer: Option<glow::Renderbuffer>,

    /// The latest values passed to `glBlendEquation`.
    pub blend_equation: (gl::types::GLenum, gl::types::GLenum),

    /// The latest values passed to `glBlendFunc`.
    pub blend_func: (
        gl::types::GLenum,
        gl::types::GLenum,
        gl::types::GLenum,
        gl::types::GLenum,
    ),

    /// The latest value passed to `glBlendColor`.
    pub blend_color: (
        gl::types::GLclampf,
        gl::types::GLclampf,
        gl::types::GLclampf,
        gl::types::GLclampf,
    ),

    /// The latest value passed to `glDepthFunc`.
    pub depth_func: gl::types::GLenum,

    /// The latest value passed to `glDepthMask`.
    pub depth_mask: bool,

    /// The latest values passed to `glDepthRange`.
    pub depth_range: (f32, f32),

    /// The latest values passed to `glStencilFuncSeparate` with face `GL_FRONT`.
    pub stencil_func_front: (gl::types::GLenum, gl::types::GLint, gl::types::GLuint),

    /// The latest values passed to `glStencilFuncSeparate` with face `GL_BACK`.
    pub stencil_func_back: (gl::types::GLenum, gl::types::GLint, gl::types::GLuint),

    /// The latest value passed to `glStencilMaskSeparate` with face `GL_FRONT`.
    pub stencil_mask_front: gl::types::GLuint,

    /// The latest value passed to `glStencilMaskSeparate` with face `GL_BACK`.
    pub stencil_mask_back: gl::types::GLuint,

    /// The latest values passed to `glStencilOpSeparate` with face `GL_FRONT`.
    pub stencil_op_front: (gl::types::GLenum, gl::types::GLenum, gl::types::GLenum),

    /// The latest values passed to `glStencilOpSeparate` with face `GL_BACK`.
    pub stencil_op_back: (gl::types::GLenum, gl::types::GLenum, gl::types::GLenum),

    /// The latest values passed to `glViewport`. `None` means unknown.
    pub viewport: Option<(
        gl::types::GLint,
        gl::types::GLint,
        gl::types::GLsizei,
        gl::types::GLsizei,
    )>,

    /// The latest values passed to `glScissor`. `None` means unknown.
    pub scissor: Option<(
        gl::types::GLint,
        gl::types::GLint,
        gl::types::GLsizei,
        gl::types::GLsizei,
    )>,

    /// The latest value passed to `glLineWidth`.
    pub line_width: gl::types::GLfloat,

    /// The latest value passed to `glPointSize`.
    pub point_size: gl::types::GLfloat,

    /// The latest value of `GL_POINT_SPRITE_COORD_ORIGIN`.
    pub point_coord_origin: gl::types::GLenum,

    /// The latest value of `GL_POINT_FADE_THRESHOLD_SIZE`.
    pub point_fade_threshold: gl::types::GLfloat,

    /// The latest value passed to `glCullFace`.
    pub cull_face: gl::types::GLenum,

    /// The latest value passed to `glFrontFace`.
    pub front_face: gl::types::GLenum,

    /// The latest value passed to `glLogicOp`.
    pub logic_op: gl::types::GLenum,

    /// The latest value passed to `glPolygonMode`.
    pub polygon_mode: gl::types::GLenum,

    /// The latest values passed to `glPolygonOffset`.
    pub polygon_offset: (gl::types::GLfloat, gl::types::GLfloat),

    /// The latest clamp passed to `glPolygonOffsetClamp`.
    pub polygon_offset_clamp: gl::types::GLfloat,

    /// The latest value passed to `glMinSampleShading`.
    pub min_sample_shading: gl::types::GLfloat,

    /// The latest value passed to `glSampleMaski` for the first mask word.
    pub sample_mask: gl::types::GLbitfield,

    /// The latest value passed to `glHint` for smoothing.
    pub smooth: (gl::types::GLenum, gl::types::GLenum),

    /// The latest value passed to `glProvokingVertex`.
    pub provoking_vertex: gl::types::GLenum,

    /// The latest value passed to `glClipControl`.
    pub clip_control: (gl::types::GLenum, gl::types::GLenum),

    /// The latest value passed to `glPixelStore` with `GL_UNPACK_ALIGNMENT`.
    pub pixel_store_unpack_alignment: gl::types::GLint,

    /// The latest value passed to `glPixelStore` with `GL_PACK_ALIGNMENT`.
    pub pixel_store_pack_alignment: gl::types::GLint,

    /// The latest value passed to `glClampColor`.
    pub clamp_color: gl::types::GLenum,

    /// The latest value passed to `glPatchParameter` with `GL_PATCH_VERTICES`.
    pub patch_patch_vertices: gl::types::GLint,

    /// The id of the active texture unit.
    /// IMPORTANT: this is a raw number (0, 1, 2, ...), not an
    ///            enumeration (GL_TEXTURE0, GL_TEXTURE1, ...).
    pub active_texture: gl::types::GLenum,

    /// List of texture units.
    pub texture_units: SmallVec<[TextureUnitState; 32]>,

    /// Current query being used for GL_SAMPLES_PASSED​.
    pub samples_passed_query: Option<glow::Query>,

    /// Current query being used for GL_ANY_SAMPLES_PASSED​.
    pub any_samples_passed_query: Option<glow::Query>,

    /// Current query being used for GL_ANY_SAMPLES_PASSED​_CONSERVATIVE.
    pub any_samples_passed_conservative_query: Option<glow::Query>,

    /// Current query being used for GL_TIME_ELAPSED​.
    pub time_elapsed_query: Option<glow::Query>,

    /// The active queries of the other targets, the pipeline statistics and the transform
    /// feedback overflows, with their target and index.
    pub indexed_queries: SmallVec<[(gl::types::GLenum, gl::types::GLuint, glow::Query); 4]>,

    /// Latest value passed to `glBeginConditionalRender​`.
    pub conditional_render: Option<(glow::Query, gl::types::GLenum)>,

    /// The bound transform feedback object, `None` for the default one.
    pub transform_feedback: Option<glow::TransformFeedback>,

    /// The transform feedback objects whose capture began and didn't end, of which at most one
    /// isn't paused.
    pub transform_feedbacks: SmallVec<[RecordingTransformFeedback; 2]>,

    /// The latest value passed to `glPrimitiveBoundingBox`.
    pub primitive_bounding_box: (f32, f32, f32, f32, f32, f32, f32, f32),

    /// Current draw call ID.
    /// We maintain a counter that is incremented at each draw call.
    pub next_draw_call_id: u64,

    /// Incremented by `restore_state`: the programs forget the uniform values and block bindings
    /// that they cached before, since code outside of SafeGL may have changed them.
    pub uniform_values_generation: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_VERTEX_ATTRIB_ARRAY_BARRIER_BIT`.
    pub latest_memory_barrier_vertex_attrib_array: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_ELEMENT_ARRAY_BARRIER_BIT`.
    pub latest_memory_barrier_element_array: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_UNIFORM_BARRIER_BIT`.
    pub latest_memory_barrier_uniform: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_TEXTURE_FETCH_BARRIER_BIT`.
    pub latest_memory_barrier_texture_fetch: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_SHADER_IMAGE_ACCESS_BARRIER_BIT`.
    pub latest_memory_barrier_shader_image_access: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_COMMAND_BARRIER_BIT`.
    pub latest_memory_barrier_command: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_PIXEL_BUFFER_BARRIER_BIT`.
    pub latest_memory_barrier_pixel_buffer: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_TEXTURE_UPDATE_BARRIER_BIT`.
    pub latest_memory_barrier_texture_update: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_BUFFER_UPDATE_BARRIER_BIT`.
    pub latest_memory_barrier_buffer_update: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_FRAMEBUFFER_BARRIER_BIT`.
    pub latest_memory_barrier_framebuffer: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_TRANSFORM_FEEDBACK_BARRIER_BIT`.
    pub latest_memory_barrier_transform_feedback: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_ATOMIC_COUNTER_BARRIER_BIT`.
    pub latest_memory_barrier_atomic_counter: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_SHADER_STORAGE_BARRIER_BIT`.
    pub latest_memory_barrier_shader_storage: u64,

    /// The draw call ID of the latest call to `glMemoryBarrier` with
    /// `GL_QUERY_BUFFER_BARRIER_BIT`.
    pub latest_memory_barrier_query_buffer: u64,
}

/// State of a texture unit (the one designated by `glActiveTexture`).
#[derive(Copy, Clone, Debug)]
pub struct TextureUnitState {
    /// Id of the texture.
    pub texture: Option<glow::Texture>,

    /// Id of the sampler.
    pub sampler: Option<glow::Sampler>,
}

/// A transform feedback object whose capture began and didn't end.
#[derive(Clone, Debug)]
pub struct RecordingTransformFeedback {
    /// The object.
    pub id: glow::TransformFeedback,
    /// The type of primitives that it captures (`glBeginTransformFeedback`).
    pub primitives: gl::types::GLenum,
    /// True if the capture is paused (`glPauseTransformFeedback`).
    pub paused: bool,
    /// The buffers that it captures into.
    pub buffers: SmallVec<[glow::Buffer; 4]>,
}

/// State of an indexed buffer target (`glBindBufferRange`/`glBindBufferBase`).
#[derive(Copy, Clone, Debug)]
pub struct IndexedBufferState {
    /// Id of the buffer.
    pub buffer: Option<glow::Buffer>,

    /// Starting offset in bytes.
    pub offset: gl::types::GLintptr,

    /// Size in bytes.
    pub size: gl::types::GLsizeiptr,
}

/// Builds the `GlState` corresponding to a newly-created OpenGL context.
impl Default for GlState {
    fn default() -> GlState {
        fn small_vec_one<T>() -> SmallVec<T>
        where
            T: ::smallvec::Array,
            T::Item: Default,
        {
            let mut v = SmallVec::new();
            v.push(Default::default());
            v
        }

        GlState {
            lost_context: false,
            reset_status: gl::NO_ERROR,

            enabled_blend: false,
            enabled_cull_face: false,
            enabled_debug_output: None,
            enabled_debug_output_synchronous: false,
            enabled_depth_test: false,
            enabled_depth_clamp_near: false,
            enabled_depth_clamp_far: false,
            enabled_dither: false,
            enabled_framebuffer_srgb: false,
            enabled_multisample: true,
            enabled_color_logic_op: false,
            enabled_polygon_offset_fill: false,
            enabled_polygon_offset_line: false,
            enabled_polygon_offset_point: false,
            enabled_rasterizer_discard: false,
            enabled_sample_alpha_to_coverage: false,
            enabled_sample_coverage: false,
            sample_coverage: (1.0, false),
            enabled_sample_alpha_to_one: false,
            enabled_sample_mask: false,
            enabled_sample_shading: false,
            enabled_scissor_test: false,
            enabled_stencil_test: false,
            enabled_line_smooth: false,
            enabled_polygon_smooth: false,
            enabled_primitive_fixed_restart: crate::version::is_webgl(),
            enabled_primitive_restart: false,
            primitive_restart_index: 0,
            patch_default_outer_level: [1.0; 4],
            patch_default_inner_level: [1.0; 2],
            vertex_attrib_values: Vec::new(),
            enabled_program_point_size: false,
            enabled_clip_planes: 0,

            program: None,
            vertex_array: None,
            clear_color: (0.0, 0.0, 0.0, 0.0),
            clear_depth: 1.0,
            clear_stencil: 0,
            color_mask: (1, 1, 1, 1),
            indexed_color_mask: false,
            indexed_blend: false,
            array_buffer_binding: None,
            pixel_pack_buffer_binding: None,
            pixel_unpack_buffer_binding: None,
            uniform_buffer_binding: None,
            indexed_uniform_buffer_bindings: small_vec_one(),
            copy_read_buffer_binding: None,
            copy_write_buffer_binding: None,
            dispatch_indirect_buffer_binding: None,
            draw_indirect_buffer_binding: None,
            query_buffer_binding: None,
            parameter_buffer_binding: None,
            texture_buffer_binding: None,
            atomic_counter_buffer_binding: None,
            indexed_atomic_counter_buffer_bindings: small_vec_one(),
            shader_storage_buffer_binding: None,
            indexed_shader_storage_buffer_bindings: small_vec_one(),
            transform_feedback_buffer_binding: None,
            indexed_transform_feedback_buffer_bindings: small_vec_one(),
            read_framebuffer: None,
            draw_framebuffer: None,
            default_framebuffer_read: None,
            renderbuffer: None,
            depth_func: gl::LESS,
            depth_mask: true,
            depth_range: (0.0, 1.0),
            stencil_func_front: (gl::ALWAYS, 0, 0xffffffff),
            stencil_func_back: (gl::ALWAYS, 0, 0xffffffff),
            stencil_mask_front: 0xffffffff,
            stencil_mask_back: 0xffffffff,
            stencil_op_front: (gl::KEEP, gl::KEEP, gl::KEEP),
            stencil_op_back: (gl::KEEP, gl::KEEP, gl::KEEP),
            blend_equation: (gl::FUNC_ADD, gl::FUNC_ADD),
            blend_func: (gl::ONE, gl::ZERO, gl::ONE, gl::ZERO),
            blend_color: (0.0, 0.0, 0.0, 0.0),
            viewport: None,
            scissor: None,
            line_width: 1.0,
            point_size: 1.0,
            point_coord_origin: gl::UPPER_LEFT,
            point_fade_threshold: 1.0,
            cull_face: gl::BACK,
            front_face: gl::CCW,
            logic_op: gl::COPY,
            polygon_mode: gl::FILL,
            smooth: (gl::DONT_CARE, gl::DONT_CARE),
            provoking_vertex: gl::LAST_VERTEX_CONVENTION,
            pixel_store_unpack_alignment: 4,
            pixel_store_pack_alignment: 4,
            clamp_color: gl::FIXED_ONLY,
            patch_patch_vertices: 3,
            active_texture: 0,
            texture_units: small_vec_one(),
            samples_passed_query: None,
            any_samples_passed_query: None,
            any_samples_passed_conservative_query: None,
            time_elapsed_query: None,
            indexed_queries: SmallVec::new(),
            conditional_render: None,
            transform_feedback: None,
            transform_feedbacks: SmallVec::new(),
            primitive_bounding_box: (-1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0),
            polygon_offset: (0.0, 0.0),
            polygon_offset_clamp: 0.0,
            min_sample_shading: 0.0,
            sample_mask: !0,
            clip_control: (gl::LOWER_LEFT, gl::NEGATIVE_ONE_TO_ONE),

            next_draw_call_id: 1,
            uniform_values_generation: 0,
            latest_memory_barrier_vertex_attrib_array: 1,
            latest_memory_barrier_element_array: 1,
            latest_memory_barrier_uniform: 1,
            latest_memory_barrier_texture_fetch: 1,
            latest_memory_barrier_shader_image_access: 1,
            latest_memory_barrier_command: 1,
            latest_memory_barrier_pixel_buffer: 1,
            latest_memory_barrier_texture_update: 1,
            latest_memory_barrier_buffer_update: 1,
            latest_memory_barrier_framebuffer: 1,
            latest_memory_barrier_transform_feedback: 1,
            latest_memory_barrier_atomic_counter: 1,
            latest_memory_barrier_shader_storage: 1,
            latest_memory_barrier_query_buffer: 1,
        }
    }
}

impl Default for TextureUnitState {
    #[inline]
    fn default() -> TextureUnitState {
        TextureUnitState {
            texture: None,
            sampler: None,
        }
    }
}

impl Default for IndexedBufferState {
    #[inline]
    fn default() -> IndexedBufferState {
        IndexedBufferState {
            buffer: None,
            offset: 0,
            size: 0,
        }
    }
}
