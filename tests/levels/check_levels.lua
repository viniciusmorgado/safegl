-- Checks the levels of SafeGL at compile time. Each case is a small crate that depends on SafeGL
-- with some levels, for some target, and uses some API: it must compile, or fail with an error
-- that contains a given text.
--
-- Usage, from the root of the repository: lua tests/levels/check_levels.lua [case...]
-- Without arguments, every case runs. The cases of a target that rustup hasn't installed are
-- skipped and listed. The crates are written to `target/levels`.

local root = io.popen("pwd"):read("l")
assert(io.open(root .. "/build/levels.rs"), "run this script from the root of the repository")

local WASM = "wasm32-unknown-unknown"
local MACOS = "aarch64-apple-darwin"

-- `code` is the body of the crate; `fail` is a text of the expected error, or nil to compile
local cases = {
  -- every level compiles the API of its version
  { "gl40_has_subroutines", { "gl40" }, nil, [[
    pub fn f(p: &safegl::Program) { let _ = p.get_subroutine_uniforms(); }
  ]] },
  { "gl41_has_program_binaries", { "gl41" }, nil, [[
    pub fn f(p: &safegl::Program) { let _ = p.get_binary(); }
  ]] },
  { "gl40_has_indirect_draws", { "gl40" }, nil, [[
    pub fn f(b: &safegl::index::DrawCommandsNoIndicesBuffer) -> Option<safegl::index::IndicesSource<'_>> {
        b.command(0, safegl::index::PrimitiveType::Points)
    }
  ]] },
  { "gl40_has_primitive_restart_index", { "gl40" }, nil, [[
    pub const R: safegl::draw_parameters::PrimitiveRestart = safegl::draw_parameters::PrimitiveRestart::Index(7);
  ]] },
  { "gles31_has_sample_positions", { "gles31" }, nil, [[
    pub fn f(f: &safegl::Frame) -> Result<Vec<[f32; 2]>, safegl::framebuffer::SamplePositionsNotSupportedError> {
        f.sample_positions()
    }
  ]] },
  { "gles31_has_indirect_draws", { "gles31" }, nil, [[
    pub use safegl::index::{DrawCommandLayout, DrawCommandsIndicesBuffer};
  ]] },
  { "gl40_has_draw_ranges", { "gl40" }, nil, [[
    pub fn f(r: &[std::ops::Range<usize>]) -> safegl::index::IndicesSource<'_> {
        safegl::index::NoIndicesRanges(safegl::index::PrimitiveType::Points, r).into()
    }
  ]] },
  { "gl40_has_captured_draws", { "gl40" }, nil, [[
    pub fn f(c: &safegl::vertex::CapturedVertices) -> safegl::index::IndicesSource<'_> {
        c.stream_indices(1, safegl::index::PrimitiveType::Points)
    }
  ]] },
  { "gles30_has_transform_feedback_objects", { "gles30" }, nil, [[
    pub use safegl::vertex::{CapturedVertices, TransformFeedback};
  ]] },
  { "gl40_has_layered_framebuffers", { "gl40" }, nil, [[
    pub fn f<'a>(d: &impl safegl::backend::Facade, t: &'a safegl::texture::Cubemap)
        -> Result<safegl::framebuffer::LayeredFrameBuffer<'a>, safegl::framebuffer::ValidationError> {
        safegl::framebuffer::LayeredFrameBuffer::new(d, t)
    }
  ]] },
  { "gles32_has_layered_framebuffers", { "gles32" }, nil, [[
    pub use safegl::framebuffer::{LayeredFrameBuffer, ToLayeredDepthAttachment};
  ]] },
  { "gl40_has_rectangle_textures", { "gl40" }, nil, [[
    pub use safegl::texture::{DepthTextureRectangle, TextureRectangle, UnsignedTextureRectangle};
  ]] },
  { "gl41_has_viewport_arrays", { "gl41" }, nil, [[
    pub fn f(v: &[safegl::ViewportState]) -> safegl::DrawParameters<'_> {
        safegl::DrawParameters { viewports: v, ..Default::default() }
    }
  ]] },
  { "gl40_has_lod_bias", { "gl40" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2d) -> safegl::uniforms::Sampler<'_, safegl::texture::Texture2d> {
        t.sampled().lod_range(0.0, 2.0).lod_bias(1.0)
    }
  ]] },
  { "gles32_has_no_lod_bias", { "gles32" }, "lod_bias", [[
    pub fn f(t: &safegl::texture::Texture2d) -> safegl::uniforms::Sampler<'_, safegl::texture::Texture2d> {
        t.sampled().lod_bias(1.0)
    }
  ]] },
  { "gl42_has_image_units", { "gl42" }, nil, [[
    pub use safegl::uniforms::ImageUnitFormat;
  ]] },
  { "gl42_has_base_instances", { "gl42" }, nil, [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { base_instance: 1, ..Default::default() }
    }
  ]] },
  { "gl43_has_buffer_clears", { "gl43" }, nil, [[
    pub fn f(b: &safegl::buffer::Buffer<[u32]>) { let _ = b.clear(0); }
  ]] },
  { "gl43_has_compute_shaders", { "gl43" }, nil, [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gl44_has_texture_clears", { "gl44" }, nil, [[
    pub use safegl::texture::ClearTextureError;
  ]] },
  { "gl45_has_texture_barriers", { "gl45" }, nil, [[
    pub use safegl::texture::texture_barrier;
  ]] },
  { "gl46_has_draw_counts", { "gl46" }, nil, [[
    pub use safegl::index::DrawCount;
  ]] },
  { "gl46_has_pipeline_statistics", { "gl46" }, nil, [[
    pub use safegl::draw_parameters::{PipelineStatisticsQuery, TransformFeedbackOverflowQuery};
  ]] },
  { "gl46_has_spirv", { "gl46" }, nil, [[
    pub use safegl::program::SpirvProgram;
  ]] },
  { "gl41_has_viewport_limit", { "gl41" }, nil, [[
    pub fn f(l: &safegl::Limits) -> Option<i32> { l.max_viewports }
  ]] },
  { "gl45_has_cull_limit", { "gl45" }, nil, [[
    pub fn f(l: &safegl::Limits) -> Option<i32> { l.max_cull_distances }
  ]] },
  { "gles30_has_cull_limit", { "gles30" }, nil, [[
    pub fn f(l: &safegl::Limits) -> Option<i32> { l.max_cull_distances }
  ]] },
  { "gles31_has_compute_limits", { "gles31" }, nil, [[
    pub fn f(l: &safegl::Limits) -> Option<i32> {
        let _ = l.max_compute_work_group_size;
        l.compute.as_ref().map(|stage| stage.max_uniform_blocks)
    }
  ]] },
  { "gl41_has_shader_precision", { "gl41" }, nil, [[
    pub use safegl::program::{PrecisionQualifier, ShaderPrecision};
  ]] },
  { "gl43_has_debug_groups", { "gl43" }, nil, [[
    pub fn f(c: &safegl::backend::Context) { let _group = c.push_debug_group(0, "group"); }
  ]] },
  { "gl45_has_reset_status", { "gl45" }, nil, [[
    pub fn f(c: &safegl::backend::Context) -> Option<safegl::GraphicsResetStatus> { c.graphics_reset_status() }
  ]] },
  { "gl46_has_no_error_contexts", { "gl46" }, nil, [[
    pub fn f(c: &safegl::backend::Context) -> bool { c.is_no_error() }
  ]] },
  { "gles30_has_shader_precision", { "gles30" }, nil, [[
    pub use safegl::program::PrecisionQualifier;
  ]] },
  { "gles32_has_object_labels", { "gles32" }, nil, [[
    pub use safegl::debug::{DebugMessage, ObjectLabel};
  ]] },
  { "gl40_has_gpu_timestamps", { "gl40" }, nil, [[
    pub fn f(c: &safegl::backend::Context) -> Option<u64> { c.gpu_timestamp() }
  ]] },
  { "gl44_has_query_buffer_modes", { "gl44" }, nil, [[
    pub use safegl::draw_parameters::{QueryBufferMode, QueryBufferValue};
  ]] },
  { "gl45_has_inverted_conditions", { "gl45" }, nil, [[
    pub fn f(q: &safegl::draw_parameters::AnySamplesPassedQuery) -> safegl::draw_parameters::ConditionalRendering<'_> {
        safegl::draw_parameters::ConditionalRendering { query: q.into(), wait: true, per_region: false, inverted: true }
    }
  ]] },
  { "gles30_has_fence_timeouts", { "gles30" }, nil, [[
    pub fn f(s: &safegl::SyncFence) -> safegl::SyncWaitResult { s.wait_timeout(std::time::Duration::ZERO) }
  ]] },
  { "gles30_has_framebuffer_invalidation", { "gles30" }, nil, [[
    pub use safegl::framebuffer::InvalidateAttachments;
  ]] },
  { "gles31_has_compute_shaders", { "gles31" }, nil, [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gles32_has_geometry_shaders", { "gles32" }, nil, [[
    pub const P: safegl::index::PrimitiveType = safegl::index::PrimitiveType::TrianglesListAdjacency;
  ]] },
  { "gles32_has_copy_image", { "gles32" }, nil, [[
    pub use safegl::texture::copy_image;
  ]] },
  { "webgl_has_framebuffer_invalidation", { "webgl" }, nil, [[
    pub use safegl::framebuffer::InvalidateAttachments;
  ]], WASM },
  { "webgl_has_front_faces_and_sample_coverage", { "webgl" }, nil, [[
    use safegl::draw_parameters::{MultisampleState, SampleCoverage};
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters {
            front_face: safegl::FrontFace::Clockwise,
            multisample: MultisampleState {
                sample_coverage: Some(SampleCoverage { value: 0.5, invert: false }),
                ..Default::default()
            },
            ..Default::default()
        }
    }
  ]], WASM },
  { "gl40_has_logic_ops_points_and_alpha_to_one", { "gl40" }, nil, [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters {
            logic_op: Some(safegl::LogicOp::Xor),
            point_coord_origin: safegl::PointCoordOrigin::LowerLeft,
            point_fade_threshold: 2.0,
            multisample: safegl::draw_parameters::MultisampleState {
                alpha_to_one: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }
  ]] },
  { "both_chains_have_both_apis", { "gl46", "gles32" }, nil, [[
    pub use safegl::draw_parameters::PolygonMode;
    pub use safegl::texture::Texture1d;
    pub const P: safegl::index::PrimitiveType = safegl::index::PrimitiveType::Patches { vertices_per_patch: 3 };
  ]] },
  { "no_level_is_gl40", {}, nil, [[
    pub use safegl::draw_parameters::PolygonMode;
  ]] },

  -- a level doesn't compile the API of the versions above it
  { "no_level_has_no_program_binaries", {}, "no method named `get_binary`", [[
    pub fn f(p: &safegl::Program) { let _ = p.get_binary(); }
  ]] },
  { "gl41_has_no_image_units", { "gl41" }, "ImageUnitFormat", [[
    pub use safegl::uniforms::ImageUnitFormat;
  ]] },
  { "gl41_has_no_base_instances", { "gl41" }, "no field named `base_instance`", [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { base_instance: 1, ..Default::default() }
    }
  ]] },
  { "gl42_has_no_buffer_clears", { "gl42" }, "no method named `clear`", [[
    pub fn f(b: &safegl::buffer::Buffer<[u32]>) { let _ = b.clear(0); }
  ]] },
  { "gl41_has_no_compute_shaders", { "gl41" }, "ComputeShader", [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gl43_has_no_query_buffers", { "gl43" }, "ToBufferError", [[
    pub use safegl::draw_parameters::ToBufferError;
  ]] },
  { "gl44_has_no_texture_barriers", { "gl44" }, "texture_barrier", [[
    pub use safegl::texture::texture_barrier;
  ]] },
  { "gl45_has_no_draw_counts", { "gl45" }, "DrawCount", [[
    pub use safegl::index::DrawCount;
  ]] },
  { "gl45_has_no_pipeline_statistics", { "gl45" }, "PipelineStatisticsQuery", [[
    pub use safegl::draw_parameters::PipelineStatisticsQuery;
  ]] },
  { "gl45_has_no_spirv", { "gl45" }, "SpirvProgram", [[
    pub use safegl::program::SpirvProgram;
  ]] },
  { "gl40_has_no_viewport_limit", { "gl40" }, "no field `max_viewports`", [[
    pub fn f(l: &safegl::Limits) -> Option<i32> { l.max_viewports }
  ]] },
  { "gl40_has_no_viewport_arrays", { "gl40" }, "ViewportState", [[
    pub use safegl::ViewportState;
  ]] },
  { "gl44_has_no_cull_limit", { "gl44" }, "no field `max_cull_distances`", [[
    pub fn f(l: &safegl::Limits) -> Option<i32> { l.max_cull_distances }
  ]] },
  { "gles30_has_no_compute_limits", { "gles30" }, "no field `compute`", [[
    pub fn f(l: &safegl::Limits) -> bool { l.compute.is_some() }
  ]] },
  { "gles32_has_no_vertex_streams_limit", { "gles32" }, "no field `max_vertex_streams`", [[
    pub fn f(l: &safegl::Limits) -> Option<i32> { l.max_vertex_streams }
  ]] },
  { "gl40_has_no_shader_precision", { "gl40" }, "PrecisionQualifier", [[
    pub use safegl::program::PrecisionQualifier;
  ]] },
  { "gl42_has_no_debug_groups", { "gl42" }, "no method named `push_debug_group`", [[
    pub fn f(c: &safegl::backend::Context) { let _group = c.push_debug_group(0, "group"); }
  ]] },
  { "gl44_has_no_reset_status", { "gl44" }, "no method named `graphics_reset_status`", [[
    pub fn f(c: &safegl::backend::Context) { let _ = c.graphics_reset_status(); }
  ]] },
  { "gl45_has_no_error_contexts", { "gl45" }, "no method named `is_no_error`", [[
    pub fn f(c: &safegl::backend::Context) -> bool { c.is_no_error() }
  ]] },
  { "gles31_has_no_object_labels", { "gles31" }, "ObjectLabel", [[
    pub use safegl::debug::ObjectLabel;
  ]] },
  { "gles31_has_no_debug_log", { "gles31" }, "no variant", [[
    pub const B: fn() -> safegl::debug::DebugCallbackBehavior = || safegl::debug::DebugCallbackBehavior::Log;
  ]] },
  { "gl43_has_no_query_buffer_modes", { "gl43" }, "QueryBufferMode", [[
    pub use safegl::draw_parameters::QueryBufferMode;
  ]] },
  { "gl44_has_no_inverted_conditions", { "gl44" }, "has no field named `inverted`", [[
    pub fn f(q: &safegl::draw_parameters::AnySamplesPassedQuery) -> safegl::draw_parameters::ConditionalRendering<'_> {
        safegl::draw_parameters::ConditionalRendering { query: q.into(), wait: true, per_region: false, inverted: true }
    }
  ]] },
  { "gles32_has_no_gpu_timestamps", { "gles32" }, "no method named `gpu_timestamp`", [[
    pub fn f(c: &safegl::backend::Context) -> Option<u64> { c.gpu_timestamp() }
  ]] },
  { "gl42_has_no_multidraw", { "gl42" }, "no method named `with_primitive_type`", [[
    pub fn f(b: &safegl::index::DrawCommandsNoIndicesBuffer) -> safegl::index::IndicesSource<'_> {
        b.with_primitive_type(safegl::index::PrimitiveType::Points)
    }
  ]] },
  { "gles30_has_no_indirect_draws", { "gles30" }, "DrawCommandsNoIndicesBuffer", [[
    pub use safegl::index::DrawCommandsNoIndicesBuffer;
  ]] },
  { "gles32_has_no_primitive_restart_index", { "gles32" }, "no variant", [[
    pub const R: safegl::draw_parameters::PrimitiveRestart = safegl::draw_parameters::PrimitiveRestart::Index(7);
  ]] },
  { "gles32_has_no_draw_ranges", { "gles32" }, "NoIndicesRanges", [[
    pub use safegl::index::NoIndicesRanges;
  ]] },
  { "gles32_has_no_captured_draws", { "gles32" }, "no method named `indices`", [[
    pub fn f(c: &safegl::vertex::CapturedVertices) { let _ = c.indices(safegl::index::PrimitiveType::Points); }
  ]] },
  { "gles32_has_no_stream_queries", { "gles32" }, "named `with_stream` found", [[
    pub fn f<F: safegl::backend::Facade>(f: &F) { let _ = safegl::draw_parameters::PrimitivesGeneratedQuery::with_stream(f, 1); }
  ]] },
  { "gles32_has_no_bgra_attributes", { "gles32" }, "Bgra8", [[
    pub use safegl::vertex::Bgra8;
  ]] },
  { "gles30_has_no_compute_shaders", { "gles30" }, "ComputeShader", [[
    pub use safegl::program::ComputeShader;
  ]] },
  { "gles31_has_no_geometry_shaders", { "gles31" }, "TrianglesListAdjacency", [[
    pub const P: safegl::index::PrimitiveType = safegl::index::PrimitiveType::TrianglesListAdjacency;
  ]] },
  { "gles31_has_no_geometry_shader_sources", { "gles31" }, "expected `NotInLevel`, found `&str`", [[
    pub fn f<F: safegl::backend::Facade>(display: &F) {
        let _ = safegl::Program::from_source(display, "", "", Some(""));
    }
  ]] },
  -- the type exists at every level for multiview framebuffers, without layered rendering
  { "gles31_has_no_layered_framebuffers", { "gles31" }, "named `new` found for struct `LayeredFrameBuffer", [[
    pub fn f<'a>(d: &impl safegl::backend::Facade, t: &'a safegl::texture::Texture2dArray) {
        let _ = safegl::framebuffer::LayeredFrameBuffer::new(d, t);
    }
  ]] },
  { "gles30_has_no_sample_positions", { "gles30" }, "sample_positions", [[
    pub fn f(f: &safegl::Frame) { let _ = f.sample_positions(); }
  ]] },
  { "gles32_has_integer_border_colors", { "gles32" }, nil, [[
    pub fn f(t: &safegl::texture::UnsignedTexture2d) -> safegl::uniforms::Sampler<'_, safegl::texture::UnsignedTexture2d> {
        t.sampled().border_color_uint([1; 4])
    }
  ]] },
  { "gles31_has_no_integer_border_colors", { "gles31" }, "border_color_uint", [[
    pub fn f(t: &safegl::texture::UnsignedTexture2d) -> safegl::uniforms::Sampler<'_, safegl::texture::UnsignedTexture2d> {
        t.sampled().border_color_uint([1; 4])
    }
  ]] },
  { "gles30_has_etc2", { "gles30" }, nil, [[
    pub const F: safegl::texture::CompressedFormat = safegl::texture::CompressedFormat::Etc2Rgba8Eac;
  ]] },
  { "gl42_has_multisample_images", { "gl42" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2dMultisample) -> bool {
        t.image_unit(safegl::uniforms::ImageUnitFormat::RGBA8).is_ok()
    }
  ]] },
  { "gles32_has_no_multisample_images", { "gles32" }, "image_unit", [[
    pub fn f(t: &safegl::texture::Texture2dMultisample) -> bool {
        t.image_unit(safegl::uniforms::ImageUnitFormat::RGBA8).is_ok()
    }
  ]] },
  { "gles32_has_buffer_images", { "gles32" }, nil, [[
    pub use safegl::texture::buffer_texture::BufferImageUnit;
  ]] },
  { "gles31_has_no_buffer_images", { "gles31" }, "BufferImageUnit", [[
    pub use safegl::texture::buffer_texture::BufferImageUnit;
  ]] },
  { "gles32_has_buffer_texture_ranges", { "gles32" }, nil, [[
    pub use safegl::texture::buffer_texture::TextureCreationError::RangeOutOfBounds;
  ]] },
  { "gles31_has_no_buffer_texture_ranges", { "gles31" }, "RangeOutOfBounds", [[
    pub use safegl::texture::buffer_texture::TextureCreationError::RangeOutOfBounds;
  ]] },
  { "gl45_has_box_reads", { "gl45" }, nil, [[
    pub fn f(t: &safegl::texture::Texture3d) -> Result<Vec<f32>, safegl::ReadError> {
        t.main_level().read_box((0, 0, 0), (1, 1, 1))
    }
  ]] },
  { "gl44_has_no_box_reads", { "gl44" }, "read_box", [[
    pub fn f(t: &safegl::texture::Texture3d) -> Result<Vec<f32>, safegl::ReadError> {
        t.main_level().read_box((0, 0, 0), (1, 1, 1))
    }
  ]] },
  { "gl43_has_format_info", { "gl43" }, nil, [[
    pub use safegl::{FormatInfo, FormatTarget};
  ]] },
  { "gl42_has_no_format_info", { "gl42" }, "FormatInfo", [[
    pub use safegl::FormatInfo;
  ]] },
  { "gl43_has_texture_views", { "gl43" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2dArray) -> Result<safegl::texture::Texture2d, safegl::texture::TextureViewError> {
        safegl::texture::Texture2d::view(t, safegl::texture::UncompressedFloatFormat::U8U8U8U8, 0..1, 0..1)
    }
  ]] },
  { "gl42_has_no_texture_views", { "gl42" }, "TextureViewError", [[
    pub use safegl::texture::TextureViewError;
  ]] },
  { "gl43_has_texture_invalidation", { "gl43" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2d) { t.main_level().invalidate(); }
  ]] },
  { "gles32_has_no_texture_invalidation", { "gles32" }, "invalidate", [[
    pub fn f(t: &safegl::texture::Texture2d) { t.main_level().invalidate(); }
  ]] },
  { "gl42_has_no_etc2", { "gl42" }, "Etc2Rgb8", [[
    pub const F: safegl::texture::CompressedFormat = safegl::texture::CompressedFormat::Etc2Rgb8;
  ]] },
  { "gl43_has_etc2", { "gl43" }, nil, [[
    pub const F: safegl::texture::CompressedSrgbFormat = safegl::texture::CompressedSrgbFormat::Etc2Rgb8;
  ]] },
  { "gles30_has_swizzle", { "gles30" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2d) { t.set_swizzle(Default::default()); }
  ]] },
  { "gles30_has_no_depth_stencil_mode", { "gles30" }, "DepthStencilTextureMode", [[
    pub use safegl::texture::DepthStencilTextureMode;
  ]] },
  { "gles31_has_depth_stencil_mode", { "gles31" }, nil, [[
    pub use safegl::texture::{DepthStencilTextureMode, DepthStencilTextureModeError};
  ]] },
  { "gl42_has_no_depth_stencil_mode", { "gl42" }, "DepthStencilTextureMode", [[
    pub use safegl::texture::DepthStencilTextureMode;
  ]] },
  { "gles32_has_no_rectangle_textures", { "gles32" }, "TextureRectangle", [[
    pub use safegl::texture::TextureRectangle;
  ]] },
  { "gles31_has_no_copy_image", { "gles31" }, "copy_image", [[
    pub use safegl::texture::copy_image;
  ]] },
  -- desktop OpenGL only, at any OpenGL ES level
  { "gles31_has_no_polygon_mode", { "gles31" }, "PolygonMode", [[
    pub use safegl::draw_parameters::PolygonMode;
  ]] },
  { "gles31_has_no_polygon_mode_field", { "gles31" }, "no field named `polygon_mode`", [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { polygon_mode: Default::default(), ..Default::default() }
    }
  ]] },
  { "gles32_has_no_logic_ops", { "gles32" }, "no field named `logic_op`", [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { logic_op: None, ..Default::default() }
    }
  ]] },
  { "gles32_has_no_point_coord_origin", { "gles32" }, "PointCoordOrigin", [[
    pub use safegl::PointCoordOrigin;
  ]] },
  { "gles32_has_no_alpha_to_one", { "gles32" }, "no field named `alpha_to_one`", [[
    pub fn f() -> safegl::draw_parameters::MultisampleState {
        safegl::draw_parameters::MultisampleState { alpha_to_one: true, ..Default::default() }
    }
  ]] },
  { "gles32_has_no_1d_textures", { "gles32" }, "Texture1d", [[
    pub use safegl::texture::Texture1d;
  ]] },
  { "gles32_has_no_texture_clears", { "gles32" }, "ClearTextureError", [[
    pub use safegl::texture::ClearTextureError;
  ]] },
  -- OpenGL ES only, at any desktop level
  { "gl46_has_no_primitive_bounding_box", { "gl46" }, "no field named `primitive_bounding_box`", [[
    pub fn f() -> safegl::DrawParameters<'static> {
        safegl::DrawParameters { primitive_bounding_box: (-1.0..1.0, -1.0..1.0, -1.0..1.0, -1.0..1.0), ..Default::default() }
    }
  ]] },
  -- WebGL 2 is the OpenGL ES 3.0 set
  { "webgl_has_no_swizzle", { "webgl" }, "Swizzle", [[
    pub use safegl::texture::Swizzle;
  ]], WASM },
  { "webgl_has_mipmap_ranges", { "webgl" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2d) -> Result<(), safegl::texture::MipmapRangeError> {
        t.set_mipmap_range(0, 0)
    }
  ]], WASM },
  { "webgl_has_no_compute_shaders", { "webgl" }, "ComputeShader", [[
    pub use safegl::program::ComputeShader;
  ]], WASM },
  { "webgl_has_no_multisample_textures", { "webgl" }, "Texture2dMultisample", [[
    pub use safegl::texture::Texture2dMultisample;
  ]], WASM },
  { "webgl_has_no_program_binaries", { "webgl" }, "no method named `get_binary`", [[
    pub fn f(p: &safegl::Program) { let _ = p.get_binary(); }
  ]], WASM },

  { "webgl_has_transform_feedback_objects", { "webgl" }, nil, [[
    pub use safegl::vertex::TransformFeedback;
  ]], WASM },
  { "webgl_has_fence_timeouts", { "webgl" }, nil, [[
    pub fn f(s: &safegl::SyncFence) -> safegl::SyncWaitResult { s.wait_timeout(std::time::Duration::ZERO) }
  ]], WASM },
  { "webgl_has_shader_precision", { "webgl" }, nil, [[
    pub use safegl::program::{PrecisionQualifier, ShaderPrecision};
  ]], WASM },
  { "webgl_has_no_debug_groups", { "webgl" }, "no method named `push_debug_group`", [[
    pub fn f(c: &safegl::backend::Context) { let _group = c.push_debug_group(0, "group"); }
  ]], WASM },
  { "webgl_has_no_binary_format_limits", { "webgl" }, "no field `num_program_binary_formats`", [[
    pub fn f(l: &safegl::Limits) -> Option<i32> { l.num_program_binary_formats }
  ]], WASM },


  -- programs: pipelines, SPIR-V specialization, locations given before linking
  { "gl41_has_program_pipelines", { "gl41" }, nil, [[
    pub use safegl::program::{ProgramPipeline, SeparableProgram, SeparableSource};
  ]] },
  { "gles31_has_program_pipelines", { "gles31" }, nil, [[
    pub use safegl::program::{ProgramPipeline, SeparableProgram, SeparableSource};
  ]] },
  { "gl40_has_no_program_pipelines", { "gl40" }, "ProgramPipeline", [[
    pub use safegl::program::ProgramPipeline;
  ]] },
  { "gles30_has_no_program_pipelines", { "gles30" }, "SeparableProgram", [[
    pub use safegl::program::SeparableProgram;
  ]] },
  { "webgl_has_no_program_pipelines", { "webgl" }, "ProgramPipeline", [[
    pub use safegl::program::ProgramPipeline;
  ]], WASM },
  { "webgl_draws_programs", { "webgl" }, nil, [[
    pub fn f(p: &safegl::Program) -> safegl::program::DrawProgram<'_> { p.into() }
  ]], WASM },
  { "gl46_has_specialization", { "gl46" }, nil, [[
    pub fn f(b: &[u8]) -> safegl::program::SpirvEntryPoint<'_> {
        safegl::program::SpirvEntryPoint::new(b, "main")
            .specialized(&[(0, safegl::program::SpecializationConstant::Float(1.0))])
    }
  ]] },
  { "gl45_has_no_specialization", { "gl45" }, "SpecializationConstant", [[
    pub use safegl::program::SpecializationConstant;
  ]] },
  { "gl46_has_spirv_extensions", { "gl46" }, nil, [[
    pub fn f(c: &safegl::Capabilities) -> &[String] { &c.spirv_extensions }
  ]] },
  { "gl45_has_no_spirv_extensions", { "gl45" }, "no field `spirv_extensions`", [[
    pub fn f(c: &safegl::Capabilities) -> &[String] { &c.spirv_extensions }
  ]] },
  { "gles30_has_link_locations", { "gles30" }, nil, [[
    pub const L: safegl::program::LinkLocations<'static> = safegl::program::LinkLocations {
        attributes: &[("position", 0)],
        fragment_outputs: &[],
    };
  ]] },
  { "webgl_has_uniform_arrays", { "webgl" }, nil, [[
    pub fn f<'a>(v: &'a &'a [f32]) -> safegl::uniforms::UniformValue<'a> {
        safegl::uniforms::AsUniformValue::as_uniform_value(v)
    }
  ]], WASM },

  -- the combinations of levels
  { "two_desktop_levels", { "gl41", "gl43" },
    "SafeGL levels `gl41` and `gl43` are both selected: select a single desktop OpenGL level", "" },
  { "two_embedded_levels", { "gles30", "gles32" },
    "SafeGL levels `gles30` and `gles32` are both selected: select a single OpenGL ES level", "" },
  { "desktop_level_in_the_browser", { "gl46", "webgl" },
    "the `gl46` level can't be selected in the browser, which only has WebGL 2", "", WASM },
  { "browser_without_webgl", {}, "browsers only have WebGL 2: select the `webgl` level", "", WASM },

  -- vertex attribute types and texture formats at the version that made them core
  { "gl41_has_double_attributes", { "gl41" }, nil, [[
    pub const T: safegl::vertex::AttributeType = safegl::vertex::AttributeType::F64F64;
  ]] },
  { "gl40_has_no_double_attributes", { "gl40" }, "F64F64", [[
    pub const T: safegl::vertex::AttributeType = safegl::vertex::AttributeType::F64F64;
  ]] },
  { "gl41_has_fixed_attributes", { "gl41" }, nil, [[
    pub const T: safegl::vertex::AttributeType = safegl::vertex::AttributeType::FixedFloatI16U16;
  ]] },
  { "gl40_has_no_fixed_attributes", { "gl40" }, "FixedFloatI16U16", [[
    pub const T: safegl::vertex::AttributeType = safegl::vertex::AttributeType::FixedFloatI16U16;
  ]] },
  { "gles30_has_fixed_attributes", { "gles30" }, nil, [[
    pub const T: safegl::vertex::AttributeType = safegl::vertex::AttributeType::FixedFloatI16U16;
  ]] },
  { "gl44_has_packed_float_attributes", { "gl44" }, nil, [[
    pub const T: safegl::vertex::AttributeType =
        safegl::vertex::AttributeType::F10F11F11UnsignedIntReversed;
  ]] },
  { "gl43_has_no_packed_float_attributes", { "gl43" }, "F10F11F11UnsignedIntReversed", [[
    pub const T: safegl::vertex::AttributeType =
        safegl::vertex::AttributeType::F10F11F11UnsignedIntReversed;
  ]] },
  { "gl41_has_rgb565", { "gl41" }, nil, [[
    pub const F: safegl::texture::UncompressedFloatFormat = safegl::texture::UncompressedFloatFormat::U5U6U5;
  ]] },
  { "gl40_has_no_rgb565", { "gl40" }, "U5U6U5", [[
    pub const F: safegl::texture::UncompressedFloatFormat = safegl::texture::UncompressedFloatFormat::U5U6U5;
  ]] },
  { "gles30_has_rgb565", { "gles30" }, nil, [[
    pub const F: safegl::texture::UncompressedFloatFormat = safegl::texture::UncompressedFloatFormat::U5U6U5;
  ]] },
  { "webgl_has_rgb565", { "webgl" }, nil, [[
    pub const F: safegl::texture::UncompressedFloatFormat = safegl::texture::UncompressedFloatFormat::U5U6U5;
  ]], WASM },

  -- the extensions of WebGL 2 widen the gates of the desktop functionalities that they give
  { "webgl_has_draw_extensions", { "webgl" }, nil, [[
    use safegl::draw_parameters::{ClipControlDepth, ClipControlOrigin, DepthClamp, PolygonMode, ProvokingVertex};
    pub fn f<'a>(q: &'a safegl::draw_parameters::TimeElapsedQuery, o: &'a [safegl::OutputState])
        -> safegl::DrawParameters<'a> {
        safegl::DrawParameters {
            clip_control_origin: ClipControlOrigin::UpperLeft,
            clip_control_depth: ClipControlDepth::ZeroToOne,
            depth: safegl::Depth { clamp: DepthClamp::Clamp, ..Default::default() },
            polygon_mode: PolygonMode::Line,
            polygon_offset: safegl::draw_parameters::PolygonOffset { line: true, ..Default::default() },
            polygon_offset_clamp: 1.0,
            provoking_vertex: ProvokingVertex::FirstVertex,
            clip_planes_bitmask: 1,
            base_instance: 1,
            outputs: o,
            time_elapsed_query: Some(q),
            ..Default::default()
        }
    }
  ]], WASM },
  { "webgl_has_no_point_offsets", { "webgl" }, "no field named `point`", [[
    pub fn f() -> safegl::draw_parameters::PolygonOffset {
        safegl::draw_parameters::PolygonOffset { point: true, ..Default::default() }
    }
  ]], WASM },
  { "webgl_has_no_conditional_rendering", { "webgl" }, "ConditionalRendering", [[
    pub use safegl::draw_parameters::ConditionalRendering;
  ]], WASM },
  { "webgl_has_draw_ranges_and_timestamps", { "webgl" }, nil, [[
    pub fn f<'a>(c: &safegl::backend::Context, r: &'a [std::ops::Range<usize>]) -> safegl::index::IndicesSource<'a> {
        let _ = c.gpu_timestamp();
        safegl::index::NoIndicesRanges(safegl::index::PrimitiveType::Points, r).into()
    }
  ]], WASM },
  { "webgl_has_extension_formats_and_samplers", { "webgl" }, nil, [[
    pub const F: safegl::texture::CompressedFormat = safegl::texture::CompressedFormat::Etc2Rgb8;
    pub const W: safegl::uniforms::SamplerWrapFunction = safegl::uniforms::SamplerWrapFunction::MirrorClamp;
    pub use safegl::texture::DepthStencilTextureMode;
  ]], WASM },
  { "webgl_has_web_images", { "webgl" }, nil, [[
    pub fn f(d: &impl safegl::backend::Facade, i: &web_sys::ImageData)
        -> Result<safegl::Texture2d, safegl::texture::WebImageError> {
        safegl::Texture2d::from_web_image(d, i, safegl::texture::MipmapsOption::NoMipmap, Default::default())
    }
  ]], WASM, { "web-sys = { version = \"0.3\", features = [\"ImageData\"] }" } },
  { "webgl_has_multiview_without_layered_rendering", { "webgl" }, "named `new` found for struct `LayeredFrameBuffer", [[
    pub fn f<'a>(d: &impl safegl::backend::Facade, t: &'a safegl::texture::Texture2dArray) {
        let _ = safegl::framebuffer::LayeredFrameBuffer::multiview(d, t, Default::default(), 0..2);
        let _ = safegl::framebuffer::LayeredFrameBuffer::new(d, t);
    }
  ]], WASM },
  { "gles30_has_multiview_and_pending_programs", { "gles30" }, nil, [[
    pub fn f<'a>(d: &impl safegl::backend::Facade, t: &'a safegl::texture::Texture2dArray, s: &str)
        -> Result<safegl::PendingProgram, safegl::ProgramCreationError> {
        let _ = safegl::framebuffer::LayeredFrameBuffer::multiview(d, t, Default::default(), 0..2);
        safegl::Program::new_pending(d, safegl::program::ProgramCreationInput::SourceCode {
            vertex_shader: s, tessellation_control_shader: None, tessellation_evaluation_shader: None,
            geometry_shader: None, fragment_shader: s, transform_feedback_varyings: None,
            outputs_srgb: true, uses_point_size: false, locations: Default::default(),
        })
    }
  ]] },

  -- transfers of whole levels and of boxes of larger client images
  { "gl40_has_level_reads", { "gl40" }, nil, [[
    pub fn f(t: &safegl::texture::Texture3d) -> Result<Vec<Vec<Vec<u8>>>, safegl::ReadError> {
        t.main_level().read_volume()
    }
  ]] },
  { "gles32_has_no_level_reads", { "gles32" }, "read_volume", [[
    pub fn f(t: &safegl::texture::Texture3d) -> Result<Vec<Vec<Vec<u8>>>, safegl::ReadError> {
        t.main_level().read_volume()
    }
  ]] },
  { "gles30_has_box_writes", { "gles30" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2d, d: &[(u8, u8, u8, u8)]) -> Result<(), safegl::texture::UploadError> {
        t.main_level().write_box_from((0, 0, 0), (1, 1, 1), d, (2, 2, 1), (1, 1, 0))
    }
  ]] },
  { "gl42_has_compressed_box_writes", { "gl42" }, nil, [[
    pub fn f(t: &safegl::texture::CompressedTexture2d, d: &[u8]) -> Result<(), safegl::texture::UploadError> {
        t.main_level().write_compressed_box_from((0, 0, 0), (4, 4, 1), d, (8, 4, 1), (4, 0, 0))
    }
  ]] },
  { "gl41_has_no_compressed_box_writes", { "gl41" }, "write_compressed_box_from", [[
    pub fn f(t: &safegl::texture::CompressedTexture2d, d: &[u8]) -> Result<(), safegl::texture::UploadError> {
        t.main_level().write_compressed_box_from((0, 0, 0), (4, 4, 1), d, (8, 4, 1), (4, 0, 0))
    }
  ]] },
  { "gl45_has_box_reads_into_images", { "gl45" }, nil, [[
    pub fn f(t: &safegl::texture::Texture2d, d: &mut [(u8, u8, u8, u8)]) -> Result<(), safegl::ReadError> {
        t.main_level().read_box_into((0, 0, 0), (1, 1, 1), d, (2, 2, 1), (1, 1, 0))
    }
  ]] },
  { "gl44_has_no_box_reads_into_images", { "gl44" }, "read_box_into", [[
    pub fn f(t: &safegl::texture::Texture2d, d: &mut [(u8, u8, u8, u8)]) -> Result<(), safegl::ReadError> {
        t.main_level().read_box_into((0, 0, 0), (1, 1, 1), d, (2, 2, 1), (1, 1, 0))
    }
  ]] },

  -- macOS stops at OpenGL 4.1, but `gl46` compiles there and fails when creating the context
  { "gl46_compiles_for_macos", { "gl46" }, nil, [[
    pub use safegl::program::SpirvProgram;
  ]], MACOS },
}

local installed = {}
for target in io.popen("rustup target list --installed 2>/dev/null"):lines() do
  installed[target] = true
end

local selected = {}
for i = 1, #arg do selected[arg[i]] = true end

local function write(path, text)
  local f = assert(io.open(path, "w"))
  f:write(text)
  f:close()
end

local function quote(s) return "'" .. s:gsub("'", "'\\''") .. "'" end

local target_dir = root .. "/target/levels"
local lock = io.open(root .. "/Cargo.lock"):read("a")
local failures, skipped, passed = {}, {}, 0

for _, case in ipairs(cases) do
  local name, features, fail, code, target = case[1], case[2], case[3], case[4], case[5]
  -- more dependencies of the crate, as lines of its `[dependencies]`
  local dependencies = table.concat(case[6] or {}, "\n")
  if next(selected) == nil or selected[name] then
    if target and not installed[target] then
      skipped[#skipped + 1] = name .. " (target " .. target .. " not installed)"
    else
      local dir = target_dir .. "/cases/" .. name
      os.execute("mkdir -p " .. quote(dir .. "/src"))
      local feature_list = {}
      for _, f in ipairs(features) do feature_list[#feature_list + 1] = '"' .. f .. '"' end
      write(dir .. "/Cargo.toml", ([[
[package]
name = "%s"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
safegl = { path = %q, default-features = false, features = [%s] }
%s

[workspace]
]]):format(name:gsub("_", "-"), root, table.concat(feature_list, ", "), dependencies))
      write(dir .. "/Cargo.lock", lock)
      write(dir .. "/src/lib.rs", "#![allow(dead_code, unused_imports)]\n" .. code .. "\n")

      local command = "cd " .. quote(dir) .. " && CARGO_TARGET_DIR=" .. quote(target_dir) ..
        " cargo check --quiet" .. (target and (" --target " .. target) or "") .. " 2>&1"
      local pipe = io.popen(command)
      local output = pipe:read("a")
      local ok = pipe:close()

      local good
      if fail then
        good = not ok and output:find(fail, 1, true) ~= nil
      else
        good = ok == true
      end
      if good then
        passed = passed + 1
        io.write("ok    ", name, "\n")
      else
        failures[#failures + 1] = name
        io.write("FAIL  ", name, ": expected ", fail and ("an error with `" .. fail .. "`") or "success",
          "\n", output, "\n")
      end
    end
  end
end

for _, s in ipairs(skipped) do io.write("skip  ", s, "\n") end
io.write(("%d passed, %d failed, %d skipped\n"):format(passed, #failures, #skipped))
os.exit(#failures == 0 and 0 or 1)
