//! The implementation-dependent limits of the context.
//!
//! A limit exists at the levels whose versions have it, and is `None` when the backend is below
//! that version and lacks the extension that brings it.

use crate::context::ExtensionsList;
use crate::gl;
use crate::version::Api;
use crate::version::Version;

use glow::HasContext;

/// Limits of the implementation, as given by `glGet*`.
///
/// The limits that `Capabilities` already exposes (texture sizes, buffer binding points, ...)
/// are not repeated here.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Limits {
    /// Limits of the vertex shader stage.
    pub vertex: StageLimits,

    /// Limits of the tessellation control shader stage. `None` without tessellation.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub tess_control: Option<StageLimits>,

    /// Limits of the tessellation evaluation shader stage. `None` without tessellation.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub tess_evaluation: Option<StageLimits>,

    /// Limits of the geometry shader stage. `None` without geometry shaders.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub geometry: Option<StageLimits>,

    /// Limits of the fragment shader stage.
    pub fragment: StageLimits,

    /// Limits of the compute shader stage. `None` without compute shaders.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub compute: Option<StageLimits>,

    /// Uniform blocks of all the stages of a program (`GL_MAX_COMBINED_UNIFORM_BLOCKS`).
    pub max_combined_uniform_blocks: gl::types::GLint,

    /// Image uniforms of all the stages of a program (`GL_MAX_COMBINED_IMAGE_UNIFORMS`). `None`
    /// without image load and store.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_combined_image_uniforms: Option<gl::types::GLint>,

    /// Atomic counter buffers of all the stages of a program
    /// (`GL_MAX_COMBINED_ATOMIC_COUNTER_BUFFERS`). `None` without atomic counters.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_combined_atomic_counter_buffers: Option<gl::types::GLint>,

    /// Atomic counters of all the stages of a program (`GL_MAX_COMBINED_ATOMIC_COUNTERS`).
    /// `None` without atomic counters.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_combined_atomic_counters: Option<gl::types::GLint>,

    /// Shader storage blocks of all the stages of a program
    /// (`GL_MAX_COMBINED_SHADER_STORAGE_BLOCKS`). `None` without shader storage buffers.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub max_combined_shader_storage_blocks: Option<gl::types::GLint>,

    /// Image units, shader storage blocks and fragment outputs used together by a program
    /// (`GL_MAX_COMBINED_SHADER_OUTPUT_RESOURCES`). `None` without image load and store.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_combined_shader_output_resources: Option<gl::types::GLint>,

    /// Number of vertex attributes (`GL_MAX_VERTEX_ATTRIBS`).
    pub max_vertex_attribs: gl::types::GLint,

    /// Components of the varyings between two stages (`GL_MAX_VARYING_COMPONENTS`).
    pub max_varying_components: gl::types::GLint,

    /// Vectors of the varyings between two stages (`GL_MAX_VARYING_VECTORS`). `None` without
    /// OpenGL 4.1, OpenGL ES or `GL_ARB_ES2_compatibility`.
    #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))))]
    pub max_varying_vectors: Option<gl::types::GLint>,

    /// Vectors of the uniforms of the default block of the vertex shader
    /// (`GL_MAX_VERTEX_UNIFORM_VECTORS`). `None` without OpenGL 4.1, OpenGL ES or
    /// `GL_ARB_ES2_compatibility`.
    #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))))]
    pub max_vertex_uniform_vectors: Option<gl::types::GLint>,

    /// Vectors of the uniforms of the default block of the fragment shader
    /// (`GL_MAX_FRAGMENT_UNIFORM_VECTORS`). `None` without OpenGL 4.1, OpenGL ES or
    /// `GL_ARB_ES2_compatibility`.
    #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))))]
    pub max_fragment_uniform_vectors: Option<gl::types::GLint>,

    /// Largest index value of an indexed draw (`GL_MAX_ELEMENT_INDEX`). `None` without OpenGL
    /// 4.3, OpenGL ES or `GL_ARB_ES3_compatibility`.
    #[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))))]
    pub max_element_index: Option<gl::types::GLint64>,

    /// Recommended maximum number of indices of an indexed draw (`GL_MAX_ELEMENTS_INDICES`).
    pub max_elements_indices: gl::types::GLint,

    /// Recommended maximum number of vertices of an indexed draw (`GL_MAX_ELEMENTS_VERTICES`).
    pub max_elements_vertices: gl::types::GLint,

    /// Whether primitive restart applies to patches
    /// (`GL_PRIMITIVE_RESTART_FOR_PATCHES_SUPPORTED`). `None` without OpenGL 4.4 or OpenGL ES
    /// 3.2.
    #[cfg(any(safegl_gl44, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl44, safegl_gles32))))]
    pub primitive_restart_for_patches: Option<bool>,

    /// Largest tessellation level (`GL_MAX_TESS_GEN_LEVEL`). `None` without tessellation.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub max_tess_gen_level: Option<gl::types::GLint>,

    /// Components of the per-patch outputs of a tessellation control shader
    /// (`GL_MAX_TESS_PATCH_COMPONENTS`). `None` without tessellation.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub max_tess_patch_components: Option<gl::types::GLint>,

    /// Components of all the outputs of a tessellation control shader, per-vertex and per-patch
    /// (`GL_MAX_TESS_CONTROL_TOTAL_OUTPUT_COMPONENTS`). `None` without tessellation.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub max_tess_control_total_output_components: Option<gl::types::GLint>,

    /// Vertices emitted by a geometry shader invocation (`GL_MAX_GEOMETRY_OUTPUT_VERTICES`).
    /// `None` without geometry shaders.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub max_geometry_output_vertices: Option<gl::types::GLint>,

    /// Components of all the vertices emitted by a geometry shader invocation
    /// (`GL_MAX_GEOMETRY_TOTAL_OUTPUT_COMPONENTS`). `None` without geometry shaders.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub max_geometry_total_output_components: Option<gl::types::GLint>,

    /// Invocations of an instanced geometry shader (`GL_MAX_GEOMETRY_SHADER_INVOCATIONS`).
    /// `None` without geometry shaders.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub max_geometry_shader_invocations: Option<gl::types::GLint>,

    /// The vertex of a primitive that gives `gl_Layer` (`GL_LAYER_PROVOKING_VERTEX`). `None`
    /// without OpenGL 4.1, OpenGL ES 3.2, `GL_ARB_viewport_array` or geometry shaders on OpenGL
    /// ES.
    #[cfg(any(safegl_gl41, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles32))))]
    pub layer_provoking_vertex: Option<ProvokingVertexConvention>,

    /// Components captured in interleaved mode by transform feedback
    /// (`GL_MAX_TRANSFORM_FEEDBACK_INTERLEAVED_COMPONENTS`).
    pub max_transform_feedback_interleaved_components: gl::types::GLint,

    /// Varyings captured in separate mode by transform feedback
    /// (`GL_MAX_TRANSFORM_FEEDBACK_SEPARATE_ATTRIBS`).
    pub max_transform_feedback_separate_attribs: gl::types::GLint,

    /// Components of each varying captured in separate mode by transform feedback
    /// (`GL_MAX_TRANSFORM_FEEDBACK_SEPARATE_COMPONENTS`).
    pub max_transform_feedback_separate_components: gl::types::GLint,

    /// Vertex streams of geometry shaders (`GL_MAX_VERTEX_STREAMS`). `None` on OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub max_vertex_streams: Option<gl::types::GLint>,

    /// Number of viewports (`GL_MAX_VIEWPORTS`). `None` without OpenGL 4.1 or
    /// `GL_ARB_viewport_array`.
    #[cfg(safegl_gl41)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl41)))]
    pub max_viewports: Option<gl::types::GLint>,

    /// Bits of subpixel precision of viewport bounds (`GL_VIEWPORT_SUBPIXEL_BITS`). `None`
    /// without OpenGL 4.1 or `GL_ARB_viewport_array`.
    #[cfg(safegl_gl41)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl41)))]
    pub viewport_subpixel_bits: Option<gl::types::GLint>,

    /// Range of the corners of viewports (`GL_VIEWPORT_BOUNDS_RANGE`). `None` without OpenGL 4.1
    /// or `GL_ARB_viewport_array`.
    #[cfg(safegl_gl41)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl41)))]
    pub viewport_bounds_range: Option<(gl::types::GLfloat, gl::types::GLfloat)>,

    /// The vertex of a primitive that gives `gl_ViewportIndex`
    /// (`GL_VIEWPORT_INDEX_PROVOKING_VERTEX`). `None` without OpenGL 4.1 or
    /// `GL_ARB_viewport_array`.
    #[cfg(safegl_gl41)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl41)))]
    pub viewport_index_provoking_vertex: Option<ProvokingVertexConvention>,

    /// Bits of subpixel precision of the rasterizer (`GL_SUBPIXEL_BITS`).
    pub subpixel_bits: gl::types::GLint,

    /// Number of clip distances (`GL_MAX_CLIP_DISTANCES`). `None` on OpenGL ES without
    /// `GL_EXT_clip_cull_distance`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub max_clip_distances: Option<gl::types::GLint>,

    /// Number of cull distances (`GL_MAX_CULL_DISTANCES`). `None` without OpenGL 4.5,
    /// `GL_ARB_cull_distance` or `GL_EXT_clip_cull_distance`.
    #[cfg(safegl_gl45)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
    pub max_cull_distances: Option<gl::types::GLint>,

    /// Number of clip and cull distances used together
    /// (`GL_MAX_COMBINED_CLIP_AND_CULL_DISTANCES`). `None` without OpenGL 4.5,
    /// `GL_ARB_cull_distance` or `GL_EXT_clip_cull_distance`.
    #[cfg(safegl_gl45)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
    pub max_combined_clip_and_cull_distances: Option<gl::types::GLint>,

    /// Smallest and largest size of points (`GL_POINT_SIZE_RANGE` on desktop OpenGL,
    /// `GL_ALIASED_POINT_SIZE_RANGE` on OpenGL ES).
    pub point_size_range: (gl::types::GLfloat, gl::types::GLfloat),

    /// Granularity of the sizes of points (`GL_POINT_SIZE_GRANULARITY`). `None` on OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub point_size_granularity: Option<gl::types::GLfloat>,

    /// Smallest and largest width of aliased lines (`GL_ALIASED_LINE_WIDTH_RANGE`).
    pub aliased_line_width_range: (gl::types::GLfloat, gl::types::GLfloat),

    /// Smallest and largest width of antialiased lines (`GL_SMOOTH_LINE_WIDTH_RANGE`). `None` on
    /// OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub smooth_line_width_range: Option<(gl::types::GLfloat, gl::types::GLfloat)>,

    /// Granularity of the widths of antialiased lines (`GL_SMOOTH_LINE_WIDTH_GRANULARITY`).
    /// `None` on OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub smooth_line_width_granularity: Option<gl::types::GLfloat>,

    /// Samples of multisample renderbuffers (`GL_MAX_SAMPLES`).
    pub max_samples: gl::types::GLint,

    /// Samples of multisample color textures (`GL_MAX_COLOR_TEXTURE_SAMPLES`). `None` on OpenGL
    /// ES 3.0.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub max_color_texture_samples: Option<gl::types::GLint>,

    /// Samples of multisample depth and stencil textures (`GL_MAX_DEPTH_TEXTURE_SAMPLES`).
    /// `None` on OpenGL ES 3.0.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub max_depth_texture_samples: Option<gl::types::GLint>,

    /// Samples of multisample integer renderbuffers and textures (`GL_MAX_INTEGER_SAMPLES`).
    /// `None` on OpenGL ES 3.0.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub max_integer_samples: Option<gl::types::GLint>,

    /// Words of the sample mask (`GL_MAX_SAMPLE_MASK_WORDS`). `None` on OpenGL ES 3.0.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub max_sample_mask_words: Option<gl::types::GLint>,

    /// Samples of the textures bound to image units (`GL_MAX_IMAGE_SAMPLES`). `None` without
    /// OpenGL 4.2 or `GL_ARB_shader_image_load_store`.
    #[cfg(safegl_gl42)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl42)))]
    pub max_image_samples: Option<gl::types::GLint>,

    /// Draw buffers that dual-source blending can write (`GL_MAX_DUAL_SOURCE_DRAW_BUFFERS`).
    /// `None` without dual-source blending: on OpenGL ES without `GL_EXT_blend_func_extended`,
    /// and on WebGL.
    pub max_dual_source_draw_buffers: Option<gl::types::GLint>,

    /// Width and height of renderbuffers (`GL_MAX_RENDERBUFFER_SIZE`).
    pub max_renderbuffer_size: gl::types::GLint,

    /// Width and height of rectangle textures (`GL_MAX_RECTANGLE_TEXTURE_SIZE`). `None` on
    /// OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub max_rectangle_texture_size: Option<gl::types::GLint>,

    /// Absolute value of the level-of-detail bias (`GL_MAX_TEXTURE_LOD_BIAS`).
    pub max_texture_lod_bias: gl::types::GLfloat,

    /// Smallest texel offset of texture lookups (`GL_MIN_PROGRAM_TEXEL_OFFSET`).
    pub min_program_texel_offset: gl::types::GLint,

    /// Largest texel offset of texture lookups (`GL_MAX_PROGRAM_TEXEL_OFFSET`).
    pub max_program_texel_offset: gl::types::GLint,

    /// Smallest texel offset of texture gathers (`GL_MIN_PROGRAM_TEXTURE_GATHER_OFFSET`). `None`
    /// on OpenGL ES 3.0.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub min_program_texture_gather_offset: Option<gl::types::GLint>,

    /// Largest texel offset of texture gathers (`GL_MAX_PROGRAM_TEXTURE_GATHER_OFFSET`). `None`
    /// on OpenGL ES 3.0.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub max_program_texture_gather_offset: Option<gl::types::GLint>,

    /// Smallest offset of `interpolateAtOffset` (`GL_MIN_FRAGMENT_INTERPOLATION_OFFSET`). `None`
    /// without OpenGL ES 3.2 or `GL_OES_shader_multisample_interpolation` on OpenGL ES.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub min_fragment_interpolation_offset: Option<gl::types::GLfloat>,

    /// Largest offset of `interpolateAtOffset` (`GL_MAX_FRAGMENT_INTERPOLATION_OFFSET`). `None`
    /// without OpenGL ES 3.2 or `GL_OES_shader_multisample_interpolation` on OpenGL ES.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub max_fragment_interpolation_offset: Option<gl::types::GLfloat>,

    /// Bits of subpixel precision of `interpolateAtOffset`
    /// (`GL_FRAGMENT_INTERPOLATION_OFFSET_BITS`). `None` without OpenGL ES 3.2 or
    /// `GL_OES_shader_multisample_interpolation` on OpenGL ES.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub fragment_interpolation_offset_bits: Option<gl::types::GLint>,

    /// Number of image units (`GL_MAX_IMAGE_UNITS`). `None` without image load and store.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_image_units: Option<gl::types::GLint>,

    /// Size in bytes of an atomic counter buffer (`GL_MAX_ATOMIC_COUNTER_BUFFER_SIZE`). `None`
    /// without atomic counters.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_atomic_counter_buffer_size: Option<gl::types::GLint>,

    /// Size in bytes of a uniform block (`GL_MAX_UNIFORM_BLOCK_SIZE`).
    pub max_uniform_block_size: gl::types::GLint64,

    /// Size in bytes of a shader storage block (`GL_MAX_SHADER_STORAGE_BLOCK_SIZE`). `None`
    /// without shader storage buffers.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub max_shader_storage_block_size: Option<gl::types::GLint64>,

    /// Alignment in bytes of the offsets of buffer textures
    /// (`GL_TEXTURE_BUFFER_OFFSET_ALIGNMENT`). `None` without OpenGL 4.3, OpenGL ES 3.2,
    /// `GL_ARB_texture_buffer_range` or texture buffers on OpenGL ES.
    #[cfg(any(safegl_gl43, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles32))))]
    pub texture_buffer_offset_alignment: Option<gl::types::GLint>,

    /// Alignment in bytes of the pointers returned by buffer mappings
    /// (`GL_MIN_MAP_BUFFER_ALIGNMENT`). `None` without OpenGL 4.2 or
    /// `GL_ARB_map_buffer_alignment`.
    #[cfg(safegl_gl42)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl42)))]
    pub min_map_buffer_alignment: Option<gl::types::GLint>,

    /// Size of a compute work group in each dimension (`GL_MAX_COMPUTE_WORK_GROUP_SIZE`).
    /// `None` without compute shaders.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub max_compute_work_group_size: Option<(gl::types::GLint, gl::types::GLint, gl::types::GLint)>,

    /// Invocations of a compute work group (`GL_MAX_COMPUTE_WORK_GROUP_INVOCATIONS`). `None`
    /// without compute shaders.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub max_compute_work_group_invocations: Option<gl::types::GLint>,

    /// Size in bytes of the shared variables of a compute shader
    /// (`GL_MAX_COMPUTE_SHARED_MEMORY_SIZE`). `None` without compute shaders.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub max_compute_shared_memory_size: Option<gl::types::GLint>,

    /// Explicit uniform locations (`GL_MAX_UNIFORM_LOCATIONS`). `None` without OpenGL 4.3,
    /// OpenGL ES 3.1 or `GL_ARB_explicit_uniform_location`.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub max_uniform_locations: Option<gl::types::GLint>,

    /// Subroutines of a shader stage (`GL_MAX_SUBROUTINES`). `None` on OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub max_subroutines: Option<gl::types::GLint>,

    /// Subroutine uniform locations of a shader stage (`GL_MAX_SUBROUTINE_UNIFORM_LOCATIONS`).
    /// `None` on OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub max_subroutine_uniform_locations: Option<gl::types::GLint>,

    /// Number of program binary formats (`GL_NUM_PROGRAM_BINARY_FORMATS`). `None` on WebGL and
    /// without OpenGL 4.1 or `GL_ARB_get_program_binary`.
    #[cfg(any(safegl_gl41, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
    pub num_program_binary_formats: Option<gl::types::GLint>,

    /// Number of shader binary formats (`GL_NUM_SHADER_BINARY_FORMATS`). `None` on WebGL and
    /// without OpenGL 4.1 or `GL_ARB_ES2_compatibility`.
    #[cfg(any(safegl_gl41, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
    pub num_shader_binary_formats: Option<gl::types::GLint>,

    /// Length of a debug message, null terminator included (`GL_MAX_DEBUG_MESSAGE_LENGTH`).
    /// `None` without OpenGL 4.3, OpenGL ES 3.2 or `GL_KHR_debug`.
    #[cfg(any(safegl_gl43, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles32))))]
    pub max_debug_message_length: Option<gl::types::GLint>,

    /// Messages kept in the debug message log (`GL_MAX_DEBUG_LOGGED_MESSAGES`). `None` without
    /// OpenGL 4.3, OpenGL ES 3.2 or `GL_KHR_debug`.
    #[cfg(any(safegl_gl43, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles32))))]
    pub max_debug_logged_messages: Option<gl::types::GLint>,

    /// Depth of the debug group stack (`GL_MAX_DEBUG_GROUP_STACK_DEPTH`). `None` without OpenGL
    /// 4.3, OpenGL ES 3.2 or `GL_KHR_debug`.
    #[cfg(any(safegl_gl43, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles32))))]
    pub max_debug_group_stack_depth: Option<gl::types::GLint>,

    /// Length of an object label, null terminator included (`GL_MAX_LABEL_LENGTH`). `None`
    /// without OpenGL 4.3, OpenGL ES 3.2 or `GL_KHR_debug`.
    #[cfg(any(safegl_gl43, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles32))))]
    pub max_label_length: Option<gl::types::GLint>,

    /// Largest timeout in nanoseconds of a wait on the server (`GL_MAX_SERVER_WAIT_TIMEOUT`).
    pub max_server_wait_timeout: gl::types::GLuint64,

    /// Bits of the counters of queries (`GL_QUERY_COUNTER_BITS`). `None` on OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub query_counter_bits: Option<QueryCounterBits>,
}

/// Limits of a shader stage, from the `GL_MAX_<STAGE>_*` values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StageLimits {
    /// Components of the uniforms of the default block (`GL_MAX_<STAGE>_UNIFORM_COMPONENTS`).
    pub max_uniform_components: gl::types::GLint,

    /// Components of all the uniforms, those of blocks included
    /// (`GL_MAX_COMBINED_<STAGE>_UNIFORM_COMPONENTS`).
    pub max_combined_uniform_components: gl::types::GLint64,

    /// Uniform blocks (`GL_MAX_<STAGE>_UNIFORM_BLOCKS`).
    pub max_uniform_blocks: gl::types::GLint,

    /// Texture image units (`GL_MAX_<STAGE>_TEXTURE_IMAGE_UNITS`, `GL_MAX_TEXTURE_IMAGE_UNITS`
    /// for the fragment shader).
    pub max_texture_image_units: gl::types::GLint,

    /// Components of the inputs (`GL_MAX_<STAGE>_INPUT_COMPONENTS`). `None` for the vertex
    /// shader, whose inputs are limited by `Limits::max_vertex_attribs`, and for compute shaders.
    pub max_input_components: Option<gl::types::GLint>,

    /// Components of the outputs (`GL_MAX_<STAGE>_OUTPUT_COMPONENTS`). `None` for the fragment
    /// shader, whose outputs are limited by `Capabilities::max_draw_buffers`, and for compute
    /// shaders.
    pub max_output_components: Option<gl::types::GLint>,

    /// Image uniforms (`GL_MAX_<STAGE>_IMAGE_UNIFORMS`). `None` without image load and store.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_image_uniforms: Option<gl::types::GLint>,

    /// Atomic counter buffers (`GL_MAX_<STAGE>_ATOMIC_COUNTER_BUFFERS`). `None` without atomic
    /// counters.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_atomic_counter_buffers: Option<gl::types::GLint>,

    /// Atomic counters (`GL_MAX_<STAGE>_ATOMIC_COUNTERS`). `None` without atomic counters.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    pub max_atomic_counters: Option<gl::types::GLint>,

    /// Shader storage blocks (`GL_MAX_<STAGE>_SHADER_STORAGE_BLOCKS`). `None` without shader
    /// storage buffers.
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    pub max_shader_storage_blocks: Option<gl::types::GLint>,
}

/// The vertex of a primitive whose value is used for a per-primitive output, such as `gl_Layer`
/// or `gl_ViewportIndex`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ProvokingVertexConvention {
    /// The first vertex (`GL_FIRST_VERTEX_CONVENTION`).
    First,
    /// The last vertex (`GL_LAST_VERTEX_CONVENTION`).
    Last,
    /// The vertex set by `glProvokingVertex` (`GL_PROVOKING_VERTEX`).
    Provoking,
    /// Any vertex: the value must be the same for all vertices (`GL_UNDEFINED_VERTEX`).
    Undefined,
}

/// Bits of the counters of the queries of each kind (`GL_QUERY_COUNTER_BITS`). A query whose
/// counter has 0 bits has no meaningful result.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct QueryCounterBits {
    /// Samples-passed queries.
    pub samples_passed: gl::types::GLint,
    /// Primitives-generated queries.
    pub primitives_generated: gl::types::GLint,
    /// Transform-feedback-primitives-written queries.
    pub transform_feedback_primitives_written: gl::types::GLint,
    /// Time-elapsed queries.
    pub time_elapsed: gl::types::GLint,
    /// Timestamp queries.
    pub timestamp: gl::types::GLint,
}

/// The features whose presence decides which limits exist at runtime.
///
/// Some fields are only read at some levels.
#[allow(dead_code)]
struct Features {
    desktop: bool,
    tessellation: bool,
    geometry: bool,
    compute: bool,
    images: bool,
    atomic_counters: bool,
    shader_storage: bool,
}

/// The names of the limits of a stage.
struct StageNames {
    uniform_components: gl::types::GLenum,
    combined_uniform_components: gl::types::GLenum,
    uniform_blocks: gl::types::GLenum,
    texture_image_units: gl::types::GLenum,
    input_components: Option<gl::types::GLenum>,
    output_components: Option<gl::types::GLenum>,
    #[allow(dead_code)]
    image_uniforms: gl::types::GLenum,
    #[allow(dead_code)]
    atomic_counter_buffers: gl::types::GLenum,
    #[allow(dead_code)]
    atomic_counters: gl::types::GLenum,
    #[allow(dead_code)]
    shader_storage_blocks: gl::types::GLenum,
}

/// Reads an integer limit if it exists.
unsafe fn optional(
    gl: &glow::Context,
    present: bool,
    name: gl::types::GLenum,
) -> Option<gl::types::GLint> {
    if present {
        Some(gl.get_parameter_i32(name))
    } else {
        None
    }
}

// the stage limits of the features above the level aren't read
#[allow(unused_variables)]
unsafe fn get_stage(gl: &glow::Context, names: StageNames, features: &Features) -> StageLimits {
    StageLimits {
        max_uniform_components: gl.get_parameter_i32(names.uniform_components),
        max_combined_uniform_components: gl.get_parameter_i64(names.combined_uniform_components),
        max_uniform_blocks: gl.get_parameter_i32(names.uniform_blocks),
        max_texture_image_units: gl.get_parameter_i32(names.texture_image_units),
        max_input_components: names
            .input_components
            .map(|name| gl.get_parameter_i32(name)),
        max_output_components: names
            .output_components
            .map(|name| gl.get_parameter_i32(name)),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_image_uniforms: optional(gl, features.images, names.image_uniforms),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_atomic_counter_buffers: optional(
            gl,
            features.atomic_counters,
            names.atomic_counter_buffers,
        ),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_atomic_counters: optional(gl, features.atomic_counters, names.atomic_counters),
        #[cfg(any(safegl_gl43, safegl_gles31))]
        max_shader_storage_blocks: optional(
            gl,
            features.shader_storage,
            names.shader_storage_blocks,
        ),
    }
}

/// Reads a pair of floats such as a range.
unsafe fn get_pair(gl: &glow::Context, name: gl::types::GLenum) -> (f32, f32) {
    let mut value = [0.0; 2];
    gl.get_parameter_f32_slice(name, &mut value);
    (value[0], value[1])
}

/// Reads a provoking vertex convention.
#[allow(dead_code)]
unsafe fn get_convention(gl: &glow::Context, name: gl::types::GLenum) -> ProvokingVertexConvention {
    match gl.get_parameter_i32(name) as gl::types::GLenum {
        gl::FIRST_VERTEX_CONVENTION => ProvokingVertexConvention::First,
        gl::LAST_VERTEX_CONVENTION => ProvokingVertexConvention::Last,
        gl::PROVOKING_VERTEX => ProvokingVertexConvention::Provoking,
        _ => ProvokingVertexConvention::Undefined,
    }
}

/// Loads the limits.
///
/// *Safety*: the OpenGL context corresponding to `gl` must be current in the thread, and `extra`
/// must be loaded from it.
// `extra` is only used at the desktop levels
#[allow(unused_variables)]
pub unsafe fn get_limits(
    gl: &glow::Context,
    extra: &gl::Extra,
    version: &Version,
    extensions: &ExtensionsList,
) -> Limits {
    let desktop = version.0 == Api::Gl;
    let features = Features {
        desktop,
        // core in OpenGL 4.0 and OpenGL ES 3.2
        tessellation: desktop
            || version >= &Version(Api::GlEs, 3, 2)
            || extensions.gl_ext_tessellation_shader
            || extensions.gl_oes_tessellation_shader,
        geometry: desktop
            || version >= &Version(Api::GlEs, 3, 2)
            || extensions.gl_ext_geometry_shader
            || extensions.gl_oes_geometry_shader,
        compute: version >= &Version(Api::Gl, 4, 3)
            || version >= &Version(Api::GlEs, 3, 1)
            || extensions.gl_arb_compute_shader,
        images: version >= &Version(Api::Gl, 4, 2)
            || version >= &Version(Api::GlEs, 3, 1)
            || extensions.gl_arb_shader_image_load_store,
        atomic_counters: version >= &Version(Api::Gl, 4, 2)
            || version >= &Version(Api::GlEs, 3, 1)
            || extensions.gl_arb_shader_atomic_counters,
        shader_storage: version >= &Version(Api::Gl, 4, 3)
            || version >= &Version(Api::GlEs, 3, 1)
            || extensions.gl_arb_shader_storage_buffer_object,
    };
    let optional = |present, name| optional(gl, present, name);

    #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
    let es2_compatibility = version.0 == Api::GlEs
        || version >= &Version(Api::Gl, 4, 1)
        || extensions.gl_arb_es2_compatibility;
    #[cfg(safegl_gl41)]
    let viewport_array = version >= &Version(Api::Gl, 4, 1) || extensions.gl_arb_viewport_array;
    #[cfg(any(safegl_desktop, safegl_gles32))]
    let interpolation = desktop
        || version >= &Version(Api::GlEs, 3, 2)
        || extensions.gl_oes_shader_multisample_interpolation;
    #[cfg(any(safegl_gl43, safegl_gles32))]
    let debug = version >= &Version(Api::Gl, 4, 3)
        || version >= &Version(Api::GlEs, 3, 2)
        || extensions.gl_khr_debug;
    #[cfg(safegl_gl45)]
    let cull_distance = version >= &Version(Api::Gl, 4, 5)
        || extensions.gl_arb_cull_distance
        || extensions.gl_ext_clip_cull_distance;

    Limits {
        vertex: get_stage(
            gl,
            StageNames {
                uniform_components: gl::MAX_VERTEX_UNIFORM_COMPONENTS,
                combined_uniform_components: gl::MAX_COMBINED_VERTEX_UNIFORM_COMPONENTS,
                uniform_blocks: gl::MAX_VERTEX_UNIFORM_BLOCKS,
                texture_image_units: gl::MAX_VERTEX_TEXTURE_IMAGE_UNITS,
                input_components: None,
                output_components: Some(gl::MAX_VERTEX_OUTPUT_COMPONENTS),
                image_uniforms: gl::MAX_VERTEX_IMAGE_UNIFORMS,
                atomic_counter_buffers: gl::MAX_VERTEX_ATOMIC_COUNTER_BUFFERS,
                atomic_counters: gl::MAX_VERTEX_ATOMIC_COUNTERS,
                shader_storage_blocks: gl::MAX_VERTEX_SHADER_STORAGE_BLOCKS,
            },
            &features,
        ),

        #[cfg(any(safegl_desktop, safegl_gles32))]
        tess_control: if features.tessellation {
            Some(get_stage(
                gl,
                StageNames {
                    uniform_components: gl::MAX_TESS_CONTROL_UNIFORM_COMPONENTS,
                    combined_uniform_components: gl::MAX_COMBINED_TESS_CONTROL_UNIFORM_COMPONENTS,
                    uniform_blocks: gl::MAX_TESS_CONTROL_UNIFORM_BLOCKS,
                    texture_image_units: gl::MAX_TESS_CONTROL_TEXTURE_IMAGE_UNITS,
                    input_components: Some(gl::MAX_TESS_CONTROL_INPUT_COMPONENTS),
                    output_components: Some(gl::MAX_TESS_CONTROL_OUTPUT_COMPONENTS),
                    image_uniforms: gl::MAX_TESS_CONTROL_IMAGE_UNIFORMS,
                    atomic_counter_buffers: gl::MAX_TESS_CONTROL_ATOMIC_COUNTER_BUFFERS,
                    atomic_counters: gl::MAX_TESS_CONTROL_ATOMIC_COUNTERS,
                    shader_storage_blocks: gl::MAX_TESS_CONTROL_SHADER_STORAGE_BLOCKS,
                },
                &features,
            ))
        } else {
            None
        },

        #[cfg(any(safegl_desktop, safegl_gles32))]
        tess_evaluation: if features.tessellation {
            Some(get_stage(
                gl,
                StageNames {
                    uniform_components: gl::MAX_TESS_EVALUATION_UNIFORM_COMPONENTS,
                    combined_uniform_components:
                        gl::MAX_COMBINED_TESS_EVALUATION_UNIFORM_COMPONENTS,
                    uniform_blocks: gl::MAX_TESS_EVALUATION_UNIFORM_BLOCKS,
                    texture_image_units: gl::MAX_TESS_EVALUATION_TEXTURE_IMAGE_UNITS,
                    input_components: Some(gl::MAX_TESS_EVALUATION_INPUT_COMPONENTS),
                    output_components: Some(gl::MAX_TESS_EVALUATION_OUTPUT_COMPONENTS),
                    image_uniforms: gl::MAX_TESS_EVALUATION_IMAGE_UNIFORMS,
                    atomic_counter_buffers: gl::MAX_TESS_EVALUATION_ATOMIC_COUNTER_BUFFERS,
                    atomic_counters: gl::MAX_TESS_EVALUATION_ATOMIC_COUNTERS,
                    shader_storage_blocks: gl::MAX_TESS_EVALUATION_SHADER_STORAGE_BLOCKS,
                },
                &features,
            ))
        } else {
            None
        },

        #[cfg(any(safegl_desktop, safegl_gles32))]
        geometry: if features.geometry {
            Some(get_stage(
                gl,
                StageNames {
                    uniform_components: gl::MAX_GEOMETRY_UNIFORM_COMPONENTS,
                    combined_uniform_components: gl::MAX_COMBINED_GEOMETRY_UNIFORM_COMPONENTS,
                    uniform_blocks: gl::MAX_GEOMETRY_UNIFORM_BLOCKS,
                    texture_image_units: gl::MAX_GEOMETRY_TEXTURE_IMAGE_UNITS,
                    input_components: Some(gl::MAX_GEOMETRY_INPUT_COMPONENTS),
                    output_components: Some(gl::MAX_GEOMETRY_OUTPUT_COMPONENTS),
                    image_uniforms: gl::MAX_GEOMETRY_IMAGE_UNIFORMS,
                    atomic_counter_buffers: gl::MAX_GEOMETRY_ATOMIC_COUNTER_BUFFERS,
                    atomic_counters: gl::MAX_GEOMETRY_ATOMIC_COUNTERS,
                    shader_storage_blocks: gl::MAX_GEOMETRY_SHADER_STORAGE_BLOCKS,
                },
                &features,
            ))
        } else {
            None
        },

        fragment: get_stage(
            gl,
            StageNames {
                uniform_components: gl::MAX_FRAGMENT_UNIFORM_COMPONENTS,
                combined_uniform_components: gl::MAX_COMBINED_FRAGMENT_UNIFORM_COMPONENTS,
                uniform_blocks: gl::MAX_FRAGMENT_UNIFORM_BLOCKS,
                texture_image_units: gl::MAX_TEXTURE_IMAGE_UNITS,
                input_components: Some(gl::MAX_FRAGMENT_INPUT_COMPONENTS),
                output_components: None,
                image_uniforms: gl::MAX_FRAGMENT_IMAGE_UNIFORMS,
                atomic_counter_buffers: gl::MAX_FRAGMENT_ATOMIC_COUNTER_BUFFERS,
                atomic_counters: gl::MAX_FRAGMENT_ATOMIC_COUNTERS,
                shader_storage_blocks: gl::MAX_FRAGMENT_SHADER_STORAGE_BLOCKS,
            },
            &features,
        ),

        #[cfg(any(safegl_gl43, safegl_gles31))]
        compute: if features.compute {
            Some(get_stage(
                gl,
                StageNames {
                    uniform_components: gl::MAX_COMPUTE_UNIFORM_COMPONENTS,
                    combined_uniform_components: gl::MAX_COMBINED_COMPUTE_UNIFORM_COMPONENTS,
                    uniform_blocks: gl::MAX_COMPUTE_UNIFORM_BLOCKS,
                    texture_image_units: gl::MAX_COMPUTE_TEXTURE_IMAGE_UNITS,
                    input_components: None,
                    output_components: None,
                    image_uniforms: gl::MAX_COMPUTE_IMAGE_UNIFORMS,
                    atomic_counter_buffers: gl::MAX_COMPUTE_ATOMIC_COUNTER_BUFFERS,
                    atomic_counters: gl::MAX_COMPUTE_ATOMIC_COUNTERS,
                    shader_storage_blocks: gl::MAX_COMPUTE_SHADER_STORAGE_BLOCKS,
                },
                &features,
            ))
        } else {
            None
        },

        max_combined_uniform_blocks: gl.get_parameter_i32(gl::MAX_COMBINED_UNIFORM_BLOCKS),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_combined_image_uniforms: optional(features.images, gl::MAX_COMBINED_IMAGE_UNIFORMS),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_combined_atomic_counter_buffers: optional(
            features.atomic_counters,
            gl::MAX_COMBINED_ATOMIC_COUNTER_BUFFERS,
        ),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_combined_atomic_counters: optional(
            features.atomic_counters,
            gl::MAX_COMBINED_ATOMIC_COUNTERS,
        ),
        #[cfg(any(safegl_gl43, safegl_gles31))]
        max_combined_shader_storage_blocks: optional(
            features.shader_storage,
            gl::MAX_COMBINED_SHADER_STORAGE_BLOCKS,
        ),
        // the same value as `GL_MAX_COMBINED_IMAGE_UNITS_AND_FRAGMENT_OUTPUTS` of OpenGL 4.2
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_combined_shader_output_resources: optional(
            features.images,
            gl::MAX_COMBINED_SHADER_OUTPUT_RESOURCES,
        ),

        max_vertex_attribs: gl.get_parameter_i32(gl::MAX_VERTEX_ATTRIBS),
        max_varying_components: gl.get_parameter_i32(gl::MAX_VARYING_COMPONENTS),
        #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
        max_varying_vectors: optional(es2_compatibility, gl::MAX_VARYING_VECTORS),
        #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
        max_vertex_uniform_vectors: optional(es2_compatibility, gl::MAX_VERTEX_UNIFORM_VECTORS),
        #[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
        max_fragment_uniform_vectors: optional(es2_compatibility, gl::MAX_FRAGMENT_UNIFORM_VECTORS),
        #[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
        max_element_index: if version.0 == Api::GlEs
            || version >= &Version(Api::Gl, 4, 3)
            || extensions.gl_arb_es3_compatibility
        {
            Some(gl.get_parameter_i64(gl::MAX_ELEMENT_INDEX))
        } else {
            None
        },
        max_elements_indices: gl.get_parameter_i32(gl::MAX_ELEMENTS_INDICES),
        max_elements_vertices: gl.get_parameter_i32(gl::MAX_ELEMENTS_VERTICES),
        #[cfg(any(safegl_gl44, safegl_gles32))]
        primitive_restart_for_patches: if version >= &Version(Api::Gl, 4, 4)
            || version >= &Version(Api::GlEs, 3, 2)
        {
            Some(gl.get_parameter_bool(gl::PRIMITIVE_RESTART_FOR_PATCHES_SUPPORTED))
        } else {
            None
        },

        #[cfg(any(safegl_desktop, safegl_gles32))]
        max_tess_gen_level: optional(features.tessellation, gl::MAX_TESS_GEN_LEVEL),
        #[cfg(any(safegl_desktop, safegl_gles32))]
        max_tess_patch_components: optional(features.tessellation, gl::MAX_TESS_PATCH_COMPONENTS),
        #[cfg(any(safegl_desktop, safegl_gles32))]
        max_tess_control_total_output_components: optional(
            features.tessellation,
            gl::MAX_TESS_CONTROL_TOTAL_OUTPUT_COMPONENTS,
        ),

        #[cfg(any(safegl_desktop, safegl_gles32))]
        max_geometry_output_vertices: optional(features.geometry, gl::MAX_GEOMETRY_OUTPUT_VERTICES),
        #[cfg(any(safegl_desktop, safegl_gles32))]
        max_geometry_total_output_components: optional(
            features.geometry,
            gl::MAX_GEOMETRY_TOTAL_OUTPUT_COMPONENTS,
        ),
        #[cfg(any(safegl_desktop, safegl_gles32))]
        max_geometry_shader_invocations: optional(
            features.geometry,
            gl::MAX_GEOMETRY_SHADER_INVOCATIONS,
        ),
        // from `GL_ARB_viewport_array` on desktop OpenGL, and from geometry shaders on OpenGL ES
        #[cfg(any(safegl_gl41, safegl_gles32))]
        layer_provoking_vertex: if version >= &Version(Api::Gl, 4, 1)
            || extensions.gl_arb_viewport_array
            || (version.0 == Api::GlEs && features.geometry)
        {
            Some(get_convention(gl, gl::LAYER_PROVOKING_VERTEX))
        } else {
            None
        },

        max_transform_feedback_interleaved_components: gl
            .get_parameter_i32(gl::MAX_TRANSFORM_FEEDBACK_INTERLEAVED_COMPONENTS),
        max_transform_feedback_separate_attribs: gl
            .get_parameter_i32(gl::MAX_TRANSFORM_FEEDBACK_SEPARATE_ATTRIBS),
        max_transform_feedback_separate_components: gl
            .get_parameter_i32(gl::MAX_TRANSFORM_FEEDBACK_SEPARATE_COMPONENTS),
        #[cfg(safegl_desktop)]
        max_vertex_streams: optional(desktop, gl::MAX_VERTEX_STREAMS),

        #[cfg(safegl_gl41)]
        max_viewports: optional(viewport_array, gl::MAX_VIEWPORTS),
        #[cfg(safegl_gl41)]
        viewport_subpixel_bits: optional(viewport_array, gl::VIEWPORT_SUBPIXEL_BITS),
        #[cfg(safegl_gl41)]
        viewport_bounds_range: if viewport_array {
            Some(get_pair(gl, gl::VIEWPORT_BOUNDS_RANGE))
        } else {
            None
        },
        #[cfg(safegl_gl41)]
        viewport_index_provoking_vertex: if viewport_array {
            Some(get_convention(gl, gl::VIEWPORT_INDEX_PROVOKING_VERTEX))
        } else {
            None
        },
        subpixel_bits: gl.get_parameter_i32(gl::SUBPIXEL_BITS),

        #[cfg(safegl_desktop)]
        max_clip_distances: optional(
            desktop || extensions.gl_ext_clip_cull_distance,
            gl::MAX_CLIP_DISTANCES,
        ),
        #[cfg(safegl_gl45)]
        max_cull_distances: optional(cull_distance, gl::MAX_CULL_DISTANCES),
        #[cfg(safegl_gl45)]
        max_combined_clip_and_cull_distances: optional(
            cull_distance,
            gl::MAX_COMBINED_CLIP_AND_CULL_DISTANCES,
        ),

        // the aliased range is the only one of OpenGL ES, and isn't in the core profile
        point_size_range: get_pair(
            gl,
            if desktop {
                gl::POINT_SIZE_RANGE
            } else {
                gl::ALIASED_POINT_SIZE_RANGE
            },
        ),
        #[cfg(safegl_desktop)]
        point_size_granularity: if desktop {
            Some(gl.get_parameter_f32(gl::POINT_SIZE_GRANULARITY))
        } else {
            None
        },
        aliased_line_width_range: get_pair(gl, gl::ALIASED_LINE_WIDTH_RANGE),
        #[cfg(safegl_desktop)]
        smooth_line_width_range: if desktop {
            Some(get_pair(gl, gl::SMOOTH_LINE_WIDTH_RANGE))
        } else {
            None
        },
        #[cfg(safegl_desktop)]
        smooth_line_width_granularity: if desktop {
            Some(gl.get_parameter_f32(gl::SMOOTH_LINE_WIDTH_GRANULARITY))
        } else {
            None
        },

        max_samples: gl.get_parameter_i32(gl::MAX_SAMPLES),
        #[cfg(any(safegl_desktop, safegl_gles31))]
        max_color_texture_samples: optional(
            desktop || version >= &Version(Api::GlEs, 3, 1),
            gl::MAX_COLOR_TEXTURE_SAMPLES,
        ),
        #[cfg(any(safegl_desktop, safegl_gles31))]
        max_depth_texture_samples: optional(
            desktop || version >= &Version(Api::GlEs, 3, 1),
            gl::MAX_DEPTH_TEXTURE_SAMPLES,
        ),
        #[cfg(any(safegl_desktop, safegl_gles31))]
        max_integer_samples: optional(
            desktop || version >= &Version(Api::GlEs, 3, 1),
            gl::MAX_INTEGER_SAMPLES,
        ),
        #[cfg(any(safegl_desktop, safegl_gles31))]
        max_sample_mask_words: optional(
            desktop || version >= &Version(Api::GlEs, 3, 1),
            gl::MAX_SAMPLE_MASK_WORDS,
        ),
        #[cfg(safegl_gl42)]
        max_image_samples: optional(desktop && features.images, gl::MAX_IMAGE_SAMPLES),
        max_dual_source_draw_buffers: optional(
            desktop || extensions.gl_ext_blend_func_extended,
            gl::MAX_DUAL_SOURCE_DRAW_BUFFERS,
        ),

        max_renderbuffer_size: gl.get_parameter_i32(gl::MAX_RENDERBUFFER_SIZE),
        #[cfg(safegl_desktop)]
        max_rectangle_texture_size: optional(desktop, gl::MAX_RECTANGLE_TEXTURE_SIZE),
        max_texture_lod_bias: gl.get_parameter_f32(gl::MAX_TEXTURE_LOD_BIAS),
        min_program_texel_offset: gl.get_parameter_i32(gl::MIN_PROGRAM_TEXEL_OFFSET),
        max_program_texel_offset: gl.get_parameter_i32(gl::MAX_PROGRAM_TEXEL_OFFSET),
        #[cfg(any(safegl_desktop, safegl_gles31))]
        min_program_texture_gather_offset: optional(
            desktop || version >= &Version(Api::GlEs, 3, 1),
            gl::MIN_PROGRAM_TEXTURE_GATHER_OFFSET,
        ),
        #[cfg(any(safegl_desktop, safegl_gles31))]
        max_program_texture_gather_offset: optional(
            desktop || version >= &Version(Api::GlEs, 3, 1),
            gl::MAX_PROGRAM_TEXTURE_GATHER_OFFSET,
        ),
        #[cfg(any(safegl_desktop, safegl_gles32))]
        min_fragment_interpolation_offset: if interpolation {
            Some(gl.get_parameter_f32(gl::MIN_FRAGMENT_INTERPOLATION_OFFSET))
        } else {
            None
        },
        #[cfg(any(safegl_desktop, safegl_gles32))]
        max_fragment_interpolation_offset: if interpolation {
            Some(gl.get_parameter_f32(gl::MAX_FRAGMENT_INTERPOLATION_OFFSET))
        } else {
            None
        },
        #[cfg(any(safegl_desktop, safegl_gles32))]
        fragment_interpolation_offset_bits: optional(
            interpolation,
            gl::FRAGMENT_INTERPOLATION_OFFSET_BITS,
        ),

        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_image_units: optional(features.images, gl::MAX_IMAGE_UNITS),
        #[cfg(any(safegl_gl42, safegl_gles31))]
        max_atomic_counter_buffer_size: optional(
            features.atomic_counters,
            gl::MAX_ATOMIC_COUNTER_BUFFER_SIZE,
        ),
        max_uniform_block_size: gl.get_parameter_i64(gl::MAX_UNIFORM_BLOCK_SIZE),
        #[cfg(any(safegl_gl43, safegl_gles31))]
        max_shader_storage_block_size: if features.shader_storage {
            Some(gl.get_parameter_i64(gl::MAX_SHADER_STORAGE_BLOCK_SIZE))
        } else {
            None
        },
        #[cfg(any(safegl_gl43, safegl_gles32))]
        texture_buffer_offset_alignment: optional(
            version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 2)
                || extensions.gl_arb_texture_buffer_range
                || extensions.gl_oes_texture_buffer
                || extensions.gl_ext_texture_buffer,
            gl::TEXTURE_BUFFER_OFFSET_ALIGNMENT,
        ),
        #[cfg(safegl_gl42)]
        min_map_buffer_alignment: optional(
            version >= &Version(Api::Gl, 4, 2) || extensions.gl_arb_map_buffer_alignment,
            gl::MIN_MAP_BUFFER_ALIGNMENT,
        ),

        #[cfg(any(safegl_gl43, safegl_gles31))]
        max_compute_work_group_size: if features.compute {
            Some((
                gl.get_parameter_indexed_i32(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 0),
                gl.get_parameter_indexed_i32(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 1),
                gl.get_parameter_indexed_i32(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 2),
            ))
        } else {
            None
        },
        #[cfg(any(safegl_gl43, safegl_gles31))]
        max_compute_work_group_invocations: optional(
            features.compute,
            gl::MAX_COMPUTE_WORK_GROUP_INVOCATIONS,
        ),
        #[cfg(any(safegl_gl43, safegl_gles31))]
        max_compute_shared_memory_size: optional(
            features.compute,
            gl::MAX_COMPUTE_SHARED_MEMORY_SIZE,
        ),

        #[cfg(any(safegl_gl43, safegl_gles31))]
        max_uniform_locations: optional(
            version >= &Version(Api::Gl, 4, 3)
                || version >= &Version(Api::GlEs, 3, 1)
                || extensions.gl_arb_explicit_uniform_location,
            gl::MAX_UNIFORM_LOCATIONS,
        ),
        #[cfg(safegl_desktop)]
        max_subroutines: optional(desktop, gl::MAX_SUBROUTINES),
        #[cfg(safegl_desktop)]
        max_subroutine_uniform_locations: optional(desktop, gl::MAX_SUBROUTINE_UNIFORM_LOCATIONS),
        #[cfg(any(safegl_gl41, safegl_gles30))]
        num_program_binary_formats: optional(
            !crate::version::is_webgl()
                && (version.0 == Api::GlEs
                    || version >= &Version(Api::Gl, 4, 1)
                    || extensions.gl_arb_get_program_binary),
            gl::NUM_PROGRAM_BINARY_FORMATS,
        ),
        #[cfg(any(safegl_gl41, safegl_gles30))]
        num_shader_binary_formats: optional(
            !crate::version::is_webgl() && es2_compatibility,
            gl::NUM_SHADER_BINARY_FORMATS,
        ),

        #[cfg(any(safegl_gl43, safegl_gles32))]
        max_debug_message_length: optional(debug, gl::MAX_DEBUG_MESSAGE_LENGTH),
        #[cfg(any(safegl_gl43, safegl_gles32))]
        max_debug_logged_messages: optional(debug, gl::MAX_DEBUG_LOGGED_MESSAGES),
        #[cfg(any(safegl_gl43, safegl_gles32))]
        max_debug_group_stack_depth: optional(debug, gl::MAX_DEBUG_GROUP_STACK_DEPTH),
        #[cfg(any(safegl_gl43, safegl_gles32))]
        max_label_length: optional(debug, gl::MAX_LABEL_LENGTH),

        // an unsigned value, which drivers can set to all ones
        max_server_wait_timeout: gl.get_parameter_i64(gl::MAX_SERVER_WAIT_TIMEOUT) as u64,

        #[cfg(safegl_desktop)]
        query_counter_bits: if desktop {
            let bits = |target| {
                let mut value = 0;
                extra.GetQueryiv(target, gl::QUERY_COUNTER_BITS, &mut value);
                value
            };
            Some(QueryCounterBits {
                samples_passed: bits(gl::SAMPLES_PASSED),
                primitives_generated: bits(gl::PRIMITIVES_GENERATED),
                transform_feedback_primitives_written: bits(
                    gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN,
                ),
                time_elapsed: bits(gl::TIME_ELAPSED),
                timestamp: bits(gl::TIMESTAMP),
            })
        } else {
            None
        },
    }
}
