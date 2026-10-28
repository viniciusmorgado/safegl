# OpenGL 4.0 to 4.5 drivers

SafeGL runs on drivers of every version from OpenGL 4.0 to 4.6. A [level](../README.md#opengl-levels) is a ceiling: the `gl46` level compiles the whole API, and a driver below 4.6 runs it, with each feature checked at runtime against the version of the driver or the extension that gives the feature to lower versions. This chapter lists every extension that OpenGL 4.1 to 4.6 made core, with:

- the SafeGL API that gives its functionality;
- the levels that compile that API (a desktop level and an OpenGL ES level where it exists);
- what SafeGL checks when the program runs;
- what SafeGL does on a driver that has neither the version nor the extension: an error, never a panic and never a silent change of behavior, or another path that gives the same result;
- the tests that cover it.

The functionality of OpenGL 4.0 itself is available on every driver, since 4.0 is the lowest version that SafeGL accepts. Extensions that only add GLSL functions are used through `#extension` lines in the shaders: their rows say *GLSL only*.

## Promoted in OpenGL 4.1

| Extension | SafeGL API | Levels | Runtime check | Without it | Tests |
|-----------|------------|--------|---------------|------------|-------|
| `GL_ARB_ES2_compatibility` | `Context::shader_precision_format`, `FixedFloatI16U16` attributes, RGB565 textures (`U5U6U5`), `release_shader_compiler` | `gl41`, `gles30` | 4.1 or the extension | `Err` (`FormatNotSupported`, `None` precisions); `release_shader_compiler` does nothing | `texture_formats`, `shaders` |
| `GL_ARB_get_program_binary` | `Program::get_binary`, `ProgramCreationInput::Binary`; binaries made retrievable when programs link | `gl41`, `gles30` | 4.1 or the extension | `GetBinaryError::NotSupported`, `ProgramCreationError::BinaryHeaderError` | `shaders` |
| `GL_ARB_separate_shader_objects` | `SeparableProgram`, `ProgramPipeline`, `DrawProgram` | `gl41`, `gles31` | 4.1, ES 3.1 or the extension | `ProgramPipelineCreationError::NotSupported` | `program_pipeline` |
| `GL_ARB_shader_precision` | GLSL only: the precision of shader operations | every level | none | the driver's precision | none (a precision guarantee) |
| `GL_ARB_vertex_attrib_64bit` | `AttributeType::F64*`, double constant attributes (`AttributeValue::Double`) | `gl41` | 4.1 or the extension | `DrawError::AttributeTypeMismatch` / `VertexBuffer` creation error | `attribute_types`, `constant_attributes` |
| `GL_ARB_viewport_array` | `DrawParameters::viewports`, `ViewportState`, `Limits::max_viewports` | `gl41` | 4.1 or the extension | `DrawError::ViewportArrayNotSupported` | `viewports` |

## Promoted in OpenGL 4.2

| Extension | SafeGL API | Levels | Runtime check | Without it | Tests |
|-----------|------------|--------|---------------|------------|-------|
| `GL_ARB_base_instance` | `DrawParameters::base_instance`, `base_instance` of indirect commands | `gl42` | 4.2 or the extension (`EXT_base_instance` on ES) | `DrawError::BaseInstanceNotSupported`; commands must have 0 | `base_instance` |
| `GL_ARB_shading_language_420pack` | GLSL only: initializer lists, `.length()`, swizzles of scalars, `binding` layouts | every level | GLSL 4.20 or the extension | the shader doesn't compile | `glsl_coverage::shading_language_420pack` |
| `GL_ARB_transform_feedback_instanced` | Instanced draws of `CapturedVertices` | `gl42` | 4.2 or the extension | `DrawError::CapturedVerticesDrawNotSupported` | `transform_feedback` |
| `GL_ARB_compressed_texture_pixel_storage` | `TextureAnyMipmap::write_compressed_box_from` | `gl42` | 4.2 or the extension | `UploadError::NotSupported` | `texture_regions` |
| `GL_ARB_conservative_depth` | GLSL only: `layout(depth_greater)` and the others | every level | GLSL 4.20 or the extension | the shader doesn't compile | `glsl_coverage::conservative_depth` |
| `GL_ARB_internalformat_query` | `Capabilities::internal_formats_textures` and `_renderbuffers` (sample counts), the validation of multisample textures | every level | 4.2, ES 3.0 (render buffers), ES 3.1 or the extension | the counts are `None` and the limits of each kind of format are used | `texture_kinds::multisample_sample_counts` |
| `GL_ARB_map_buffer_alignment` | Mapped buffers, `Limits::min_map_buffer_alignment` | every level | 4.2 or the extension | a misaligned mapping is copied to aligned memory; immutable buffers aren't made | `buffer` in `mesa-41-buffer-storage-without-map-alignment` |
| `GL_ARB_shader_atomic_counters` | `atomic_uint` uniforms with `BufferType::AtomicCounterBuffer`, their limits | `gl42`, `gles31` | 4.2, ES 3.1 or the extension | `BufferCreationError::BufferTypeNotSupported` | `atomic_counter` (also without the program interface query) |
| `GL_ARB_shader_image_load_store` | `ImageUnit`, `ImageUnitFormat`, automatic memory barriers | `gl42`, `gles31` | 4.2, ES 3.1 or the extension | shaders with images don't compile, and image units give `DrawError::InsufficientImageUnits` | `memory_barriers`, `compute`, `glsl_coverage` |
| `GL_ARB_shading_language_packing` | GLSL only: `packHalf2x16` and the other packing functions | every level | GLSL 4.20 or the extension | the shader doesn't compile | `glsl_coverage::packing_functions` |
| `GL_ARB_texture_storage` | The storage of every texture (internal) | every level | 4.2, ES 3.0 or the extension | `glTexImage*` for each level and each face of cube maps | `mesa-bare-40`, `mesa-bare-41` |

## Promoted in OpenGL 4.3

| Extension | SafeGL API | Levels | Runtime check | Without it | Tests |
|-----------|------------|--------|---------------|------------|-------|
| `GL_ARB_arrays_of_arrays` | Uniforms of arrays of arrays (`UniformArray`) | every level | GLSL 4.30 or the extension | the shader doesn't compile | `uniform_arrays::arrays_of_arrays` |
| `GL_ARB_ES3_compatibility` | ETC2 and EAC formats, `PrimitiveRestart::FixedIndex`, conservative occlusion queries, `Limits::max_element_index`, GLSL ES 3.00 | `gl43`, `gles30` | 4.3, ES 3.0 or the extension | `FixedIndex` emulated with `GL_PRIMITIVE_RESTART`; the exact occlusion query | `primitive_restart`, `texture_kinds`, `mesa-gl46-on-40-no-es3` |
| `GL_ARB_clear_buffer_object` | `Buffer::clear`, `BufferSlice::clear` | `gl43` | 4.3 or the extension | `ClearBufferError::NotSupported` | `buffer_clear` |
| `GL_ARB_compute_shader` | `ComputeShader`, `ComputeCommand`, indirect dispatches | `gl43`, `gles31` | 4.3, ES 3.1 or the extension | `ProgramCreationError::ShaderTypeNotSupported` | `compute` |
| `GL_ARB_copy_image` | `copy_image`, `TextureAnyMipmap::copy_to` | `gl43`, `gles32` | 4.3, ES 3.2 or the extension | `CopyImageError::NotSupported` | `texture_operations` |
| `GL_KHR_debug` | `push_debug_group`, `debug::ObjectLabel`, message control, insertion and log, the debug callback | `gl43`, `gles32` | 4.3, ES 3.2 or the extension (`ARB_debug_output` for the callback) | no groups or labels; the callback through `ARB_debug_output` only fires in debug contexts | `debug`, `reentrancy`, `mesa-bare-43` |
| `GL_ARB_explicit_uniform_location` | `layout(location)` on uniforms, `Limits::max_uniform_locations` | `gl43`, `gles31` (limit) | GLSL 4.30 or the extension | the shader doesn't compile | `glsl_coverage::explicit_uniform_locations`, `subroutines` |
| `GL_ARB_fragment_layer_viewport` | `gl_Layer` and `gl_ViewportIndex` read by fragment shaders | every level | GLSL 4.30 or the extension | the shader doesn't compile | `layered_rendering`, `viewports::fragment_shaders_read_the_viewport_index` |
| `GL_ARB_framebuffer_no_attachments` | `EmptyFrameBuffer` | `gl43`, `gles31` | 4.3, ES 3.1 or the extension | `ValidationError::EmptyFramebufferObjectsNotSupported` | `framebuffer` |
| `GL_ARB_internalformat_query2` | `Context::format_info` and every answer of `FormatInfo` | `gl43` | 4.3 or the extension | `FormatInfoNotSupportedError` | `format_info` |
| `GL_ARB_invalidate_subdata` | `invalidate` of buffers, texture levels and framebuffers | `gl43` (textures), every level (buffers) | 4.3 or the extension | buffers are orphaned; textures and framebuffers are left as they are (a hint) | `framebuffer_invalidation`, `buffer` |
| `GL_ARB_multi_draw_indirect` | Multidraw command buffers (`DrawCommandsNoIndicesBuffer` sources) | `gl43` | 4.3 or the extension | one command draws without it, none draws nothing, several give `DrawError::MultidrawNotSupported` | `multi_draw`, `draw_indirect` |
| `GL_ARB_program_interface_query` | Reflection of programs (internal) | every level | 4.3, ES 3.1 or the extension | the older queries (`glGetActiveAtomicCounterBufferiv`, transform feedback varyings) | `atomic_counter` in `mesa-41-atomic-without-piq` |
| `GL_ARB_robust_buffer_access_behavior` | `Capabilities::robust_buffer_access` | every level | robust context and 4.3, ES 3.2 or the extension | `false` | `robustness` |
| `GL_ARB_shader_image_size` | GLSL only: `imageSize` | every level | GLSL 4.30 or the extension | the shader doesn't compile | `glsl_coverage::image_size_and_samples` |
| `GL_ARB_shader_storage_buffer_object` | Shader storage blocks, `BufferType::ShaderStorageBuffer` | `gl43`, `gles31` | 4.3, ES 3.1 or the extension | `BufferCreationError::BufferTypeNotSupported` | `ssbo` |
| `GL_ARB_stencil_texturing` | `TextureAny::set_depth_stencil_mode`, sampling stencil values | `gl43`, `gles31` | 4.3, ES 3.1 or the extension | `DepthStencilTextureModeError::NotSupported` | `texture_parameters` |
| `GL_ARB_texture_buffer_range` | `BufferTexture::from_buffer_range` | `gl43`, `gles32` | 4.3, ES 3.2 or the extension | `RangeNotSupported` | `texture_kinds` |
| `GL_ARB_texture_query_levels` | GLSL only: `textureQueryLevels` | every level | GLSL 4.30 or the extension | the shader doesn't compile | `glsl_coverage::texture_queries` |
| `GL_ARB_texture_storage_multisample` | The storage of multisample textures (internal), texture views of them | every level | 4.3, ES 3.1 or the extension | `glTexImage*Multisample`; views of them return `SourceNotImmutable` | `texture_kinds`, `mesa-bare-42` |
| `GL_ARB_texture_view` | `new_texture_view`, the `view` constructors | `gl43` | 4.3 or the extension | `TextureViewError::NotSupported` | `texture_kinds` |
| `GL_ARB_vertex_attrib_binding` | The vertex formats of vertex array objects (internal) | every level | 4.3, ES 3.1 or the extension | `glVertexAttribPointer` for each attribute | `vertex_bindings`, `mesa-bare-42` |

## Promoted in OpenGL 4.4

| Extension | SafeGL API | Levels | Runtime check | Without it | Tests |
|-----------|------------|--------|---------------|------------|-------|
| `GL_ARB_buffer_storage` | `BufferMode::Persistent` and `Immutable` (internal storage) | every level | 4.4 or the extension (`EXT_buffer_storage` on ES), with aligned mappings | `glBufferData`; persistent buffers are mapped for each access | `buffer` in `mesa-43-dsa-without-buffer-storage` |
| `GL_ARB_clear_texture` | `TextureAnyMipmap::clear`, `clear_region` | `gl44` | 4.4 or the extension | `ClearTextureError::NotSupported` | `texture_operations` |
| `GL_ARB_enhanced_layouts` | `xfb_buffer`/`xfb_offset` capture, `layout(component)` attributes, block layouts | every level | 4.4 or the extension (with the program interface query) | the older transform feedback reflection; components start at 0 | `transform_feedback`, `attribute_components` |
| `GL_ARB_multi_bind` | Not used: SafeGL binds each object | every level | none | the same binds | every draw |
| `GL_ARB_query_buffer_object` | `to_buffer` of queries, `QueryBufferMode` | `gl44` | 4.4 or the extension | `QueryCreationError::NotSupported` | `query` |
| `GL_ARB_texture_mirror_clamp_to_edge` | `SamplerWrapFunction::MirrorClamp` | `gl44` | 4.4 or the extension | `DrawError::SamplerNotSupported` | `samplers` |
| `GL_ARB_texture_stencil8` | `StencilTexture*` types | `gl44`, `gles32` | 4.4, ES 3.2 or the extension | `TextureCreationError::FormatNotSupported` | `texture_parameters`, `texture_levels` |
| `GL_ARB_vertex_type_10f_11f_11f_rev` | `AttributeType::F10F11F11UnsignedIntReversed` | `gl44` | 4.4 or the extension | the vertex buffer can't be made with the attribute | `attribute_types::packed_float_attributes` |

## Promoted in OpenGL 4.5

| Extension | SafeGL API | Levels | Runtime check | Without it | Tests |
|-----------|------------|--------|---------------|------------|-------|
| `GL_ARB_clip_control` | `DrawParameters::clip_control_origin`, `clip_control_depth` | `gl45` | 4.5 or the extension | `DrawError::ClipControlNotSupported` for other values than the defaults | `rasterization::clip_control` |
| `GL_ARB_conditional_render_inverted` | `ConditionalRendering::inverted` | `gl45` | 4.5 or the extension | `DrawError::ConditionalRenderingInvertedNotSupported` | `query` |
| `GL_ARB_cull_distance` | `gl_CullDistance`, `Limits::max_cull_distances` | `gl45`, `gles30` | 4.5 or the extension | the limit is `None` | `rasterization::cull_distances` |
| `GL_ARB_derivative_control` | GLSL only: `dFdxFine` and the others | every level | GLSL 4.50 or the extension | the shader doesn't compile | `glsl_coverage::derivative_control` |
| `GL_ARB_direct_state_access` | Internal: buffers, framebuffers, texture parameters and queries | every level | 4.5 or the extension | the objects are bound | `mesa-bare-40` to `mesa-bare-44` |
| `GL_ARB_ES3_1_compatibility` | GLSL ES 3.10, `gl_HelperInvocation`, float `imageAtomicExchange` | every level | 4.5 or the extension | the shader doesn't compile | `glsl_coverage` |
| `GL_ARB_get_texture_sub_image` | `read_box`, `read_compressed_box`, `read_box_into`; whole-level reads | `gl45` (boxes), every desktop level (`read_volume`) | 4.5 or the extension | `ReadError::NotSupported` for boxes; levels are read with `glGetTexImage` | `texture_io`, `texture_levels`, `texture_regions` |
| `GL_KHR_robustness` | `Context::graphics_reset_status`, `Capabilities::robustness`, robust reads | `gl45`, `gles32` | 4.5, ES 3.2, the extension or `ARB_robustness` | the status is `None`; reads aren't bounded | `robustness`, `mesa-44-khr-robustness-without-arb` |
| `GL_ARB_shader_texture_image_samples` | GLSL only: `textureSamples`, `imageSamples` | every level | GLSL 4.50 or the extension | the shader doesn't compile | `glsl_coverage` |
| `GL_ARB_texture_barrier` | `Context::texture_barrier` | `gl45` | 4.5 or the extension (`NV_texture_barrier` too) | `TextureBarrierNotSupportedError` | `texture_operations` |
| `GL_KHR_context_flush_control` | `Capabilities::release_behavior` | every level | 4.5 or the extension | `Flush`, the default | `robustness::contexts_flush_when_released` |

## Promoted in OpenGL 4.6

| Extension | SafeGL API | Levels | Runtime check | Without it | Tests |
|-----------|------------|--------|---------------|------------|-------|
| `GL_ARB_indirect_parameters` | `with_draw_count` of multidraw sources (`DrawCount`) | `gl46` | 4.6 or the extension | `DrawError::DrawCountNotSupported` | `draw_count` |
| `GL_ARB_pipeline_statistics_query` | `PipelineStatisticsQuery` | `gl46` | 4.6 or the extension | `QueryCreationError::NotSupported` | `pipeline_queries` |
| `GL_ARB_polygon_offset_clamp` | `DrawParameters::polygon_offset_clamp` | `gl46` | 4.6 or the extension (`EXT_polygon_offset_clamp` too) | `DrawError::PolygonOffsetClampNotSupported` | `polygon_offset_clamp` |
| `GL_ARB_shader_atomic_counter_ops` | GLSL only: `atomicCounterAdd` and the others | every level | GLSL 4.60 or the extension (`ARB`-suffixed functions) | the shader doesn't compile | `glsl_coverage::atomic_counter_operations` |
| `GL_ARB_gl_spirv` | `SpirvProgram`, `SpirvEntryPoint`, specialization | `gl46` | 4.6 or the extension (`glSpecializeShaderARB`) | `ProgramCreationError::CompilationNotSupported` | `spirv`, `mesa-gl46-on-40` |
| `GL_ARB_spirv_extensions` | `Capabilities::spirv_extensions` | `gl46` | 4.6 or the extension | an empty list | `spirv` |
| `GL_ARB_texture_filter_anisotropic` | `SamplerBehavior::max_anisotropy` | every level | 4.6 or the extension (`EXT_texture_filter_anisotropic` too) | `DrawError::SamplerNotSupported` above 1 | `samplers` |
| `GL_ARB_transform_feedback_overflow_query` | `TransformFeedbackOverflowQuery` | `gl46` | 4.6 or the extension | `QueryCreationError::NotSupported` | `pipeline_queries` |
| `GL_ARB_shader_draw_parameters` | GLSL only: `gl_DrawID`, `gl_BaseVertex`, `gl_BaseInstance` | every level | GLSL 4.60 or the extension | the shader doesn't compile | `multi_draw` |
| `GL_ARB_shader_group_vote` | GLSL only: `allInvocations` and the others | every level | GLSL 4.60 or the extension (`ARB`-suffixed functions) | the shader doesn't compile | `glsl_coverage::shader_group_vote` |
| `GL_KHR_no_error` | `Capabilities::no_error`, `Context::is_no_error`, `SimpleWindowBuilder::with_robustness` | `gl46` | 4.6 or the extension | contexts check errors | `robustness`, `*-no-error` configurations |

## How the drivers are tested

The test suite runs on Mesa's llvmpipe made to look like each version: `MESA_GL_VERSION_OVERRIDE` sets the version, and on *bare* drivers the extensions promoted after it are removed. Two variables remove them:

- `MESA_EXTENSION_OVERRIDE` makes Mesa hide an extension, from SafeGL and from the shader compiler;
- `SAFEGL_DISABLE_EXTENSIONS`, a space-separated list of extension names, makes SafeGL treat them as missing. Only debug builds read it.

Mesa can't hide every extension: it refuses to disable `GL_ARB_get_program_binary`, `GL_ARB_separate_shader_objects`, `GL_ARB_compressed_texture_pixel_storage`, `GL_ARB_map_buffer_alignment`, `GL_ARB_texture_storage`, `GL_ARB_clear_buffer_object`, `GL_KHR_debug`, `GL_ARB_invalidate_subdata`, `GL_ARB_program_interface_query`, `GL_ARB_vertex_attrib_binding`, `GL_ARB_clear_texture`, `GL_ARB_multi_bind`, `GL_ARB_direct_state_access`, `GL_ARB_get_texture_sub_image`, `GL_ARB_robustness`, `GL_KHR_context_flush_control` and `GL_KHR_no_error`, and disabling `GL_ARB_multi_draw_indirect`, `GL_ARB_texture_storage_multisample` or `GL_ARB_ES2_compatibility` breaks older core functionality of llvmpipe (indirect draws, the limits of multisample textures, cleared depth textures). These are only hidden from SafeGL, which then takes the paths of drivers without them; Mesa still implements them, so a driver whose behavior differs without them isn't simulated.

The configurations of the tests are:

| Configuration | Driver | Level | What it covers |
|---------------|--------|-------|----------------|
| `mesa-gl40` to `mesa-gl45` | Mesa as OpenGL 4.0 to 4.5 | the same version | The API of each level on its own driver |
| `mesa-gl46-on-40` | Mesa as OpenGL 4.0, with every extension | `gl46` | The paths through the extensions, and the GLSL functionality through `#extension` |
| `mesa-bare-40` to `mesa-bare-45` | Mesa as OpenGL 4.0 to 4.5, without the extensions promoted later | `gl46` | The errors and fallbacks of drivers without the version or the extension |
| `mesa-41-atomic-without-piq` | Mesa as OpenGL 4.1, without the program interface query | `gl46` | Atomic counters reflected with the older query |
| `mesa-43-dsa-without-buffer-storage` | Mesa as OpenGL 4.3, with direct state access and without buffer storage | `gl46` | Buffers made with `glBufferData` |
| `mesa-41-buffer-storage-without-map-alignment` | Mesa as OpenGL 4.1, without mapping alignment | `gl46` | Buffers without immutable storage |
| `mesa-44-khr-robustness-without-arb` | Mesa as OpenGL 4.4, with `GL_KHR_robustness` only | `gl46` | Robust access read from the context flags |

The continuous integration runs the `gl41` to `gl45` levels on their drivers and the `gl46` level on a bare OpenGL 4.0 driver.

## What can't be verified here

- **macOS**: its OpenGL 4.1 has few extensions. The bare OpenGL 4.1 configuration stands in for it, but the driver of Apple isn't run.
- **NVIDIA and AMD below OpenGL 4.6**: their version can't be lowered. Their paths below 4.6 are those of Mesa, and drivers differ in what they report: NVIDIA gives the compressed size of a whole cube map for each face, the texel size of image units in bytes where the specification and Mesa give bits, and the size of compressed blocks in bits.
- **Extensions that Mesa can't disable**: listed above.
