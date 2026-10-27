# Changelog

All notable changes to this project are documented in this file. Versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Breaking changes:

- `Blend::alpha_blending` blends the alpha channel with `source + destination × (1 - source
  alpha)` instead of `source alpha × source alpha + destination × (1 - source alpha)`, so the alpha
  of the target is the coverage of what was drawn over it (glium#1844). Code that reads back or
  composites the alpha of targets drawn with it sees higher, correct alphas.
- Samplers repeat textures by default (`SamplerWrapFunction::Repeat`), like OpenGL and like textures
  used without a sampler, instead of mirroring them (glium#1967, glium#1536). To keep the previous
  behavior, call `.wrap_function(SamplerWrapFunction::Mirror)` on the sampler.
- `Vertex` is an unsafe trait, since SafeGL trusts its layout to read the vertices (glium#1684).
  `implement_vertex!` is unchanged; manual implementations become `unsafe impl Vertex for ...`.
- Buffers can only be created without data (`Buffer::empty*`, `VertexBuffer::empty*`,
  `UniformBuffer::empty*`, ...), invalidated, read or mapped for reading when their content
  implements the new unsafe trait `buffer::AnyBitPattern`, which says that every bit pattern is a
  valid value. The content of such buffers is undefined, so reading it as `bool`, `char` or a
  reference was undefined behavior, and an empty buffer of `TextureHandle`s made the GPU use
  garbage handles (glium#1083). The trait is implemented for integers, floats, and arrays, slices
  and tuples of them; implement it for your structs with
  `implement_any_bit_pattern!(Struct, field1, field2, ...)` (all fields, or their indices for
  tuple structs), which checks every field at compile time. `Index`, `PixelValue` and
  `TextureBufferContent` require it, and `Content::read` requires it.
- `UniformBlock` is an unsafe trait, since the GPU trusts the layout it describes, including the
  texture handles in it. `implement_uniform_block!` is unchanged; manual implementations become
  `unsafe impl UniformBlock for ...`. `UniformValue::Block` can no longer be built directly: use
  `UniformValue::block(buffer_slice)`, which checks the layout with the type of the buffer, or the
  unsafe `UniformValue::block_unchecked`.
- `get()` on queries and on `debug::TimestampQuery` returns `Result<_, QueryResultError>`, with
  `QueryResultError::ContextLost` when the OpenGL context has been lost, since the result is lost
  with it (glium#1286); `is_ready()` returns `false` then. Add `.unwrap()` or `?` to the calls.
- `SamplesPassedQuery`, `TimeElapsedQuery`, `PrimitivesGeneratedQuery` and
  `TransformFeedbackPrimitivesWrittenQuery` return `u64` results instead of `u32`: 32 bits of
  nanoseconds overflow after 4.3 seconds, and counters can exceed them too.
- `DrawError::RasterizerDiscardNotSupported` is removed: `draw_primitives: false` works on OpenGL
  ES 3.0 and above, where `GL_RASTERIZER_DISCARD` is core, instead of returning that error.
- OpenGL objects can only be used with the context that created them, and SafeGL now checks it
  everywhere instead of using whatever object has the same name in the other context
  (glium#24): draws and compute dispatches return `DrawError::ContextMismatch`, framebuffers
  `ValidationError::ContextMismatch`, copies `CopyError::ContextMismatch` and
  `CopyImageError::ContextMismatch`, buffer textures and transform feedback sessions their own
  `ContextMismatch`, and texture transfers `UploadError::ContextMismatch` and
  `ReadError::ContextMismatch`. `LinearSyncFence::into_sync_fence` returns the fence when given
  another context.
- The blit methods of `Surface` (`blit_color`, `blit_whole_color_to`, `fill`, `blit_from_*`,
  `blit_buffers_from_*`) return `Result<(), BlitError>`: a source from another context, a depth or
  stencil blit with a linear filter (which used to panic) and a blit from an external framebuffer
  to a surface that can't receive it (which used to panic too) are errors. `blit_from_frame` and
  `blit_buffers_from_frame` take the context whose default framebuffer is the source.
- `UniformValue` has a single `Texture(&TextureAny, Option<SamplerBehavior>)` variant and a single
  `Image(&TextureAny, ImageUnitBehavior)` variant instead of one per texture type (glium#1035). The
  kind and dimensions of the texture are checked against the sampler or image type of the uniform
  at draw time, which also makes multisample array textures and `usampler1DArray` uniforms work.
  The image unit behavior is no longer optional: `None` panicked.
- Sampling a texture with a shadow sampler (`sampler2DShadow`, ...) without a depth comparison in
  its sampler, or with another sampler and a depth comparison, returns
  `DrawError::DepthComparisonMismatch`: OpenGL returns undefined values then.
- `ComputeShader::execute` and `execute_indirect` return `Result<(), DrawError>` instead of
  panicking on uniform errors, on unsupported compute shaders
  (`DrawError::ComputeShadersNotSupported`), on too many work groups
  (`DrawError::WorkGroupCountTooLarge`, which also allows the maximum itself now) and on unaligned
  indirect offsets.
- Texture writes (`write`, `write_compressed_data`, `raw_write`, `raw_upload_from_pixel_buffer*`)
  return `Result<(), texture::UploadError>`, and texture reads (`read`, `read_to_pixel_buffer`,
  `raw_read`, `raw_read_to_pixel_buffer`) return `Result<_, ReadError>`, instead
  of panicking on regions out of the texture, data whose size doesn't match (glium#1875),
  unsupported formats, multisample textures, small pixel buffers and lost contexts. Add `.unwrap()`
  or `?` to the calls.
- A transform feedback session must be used with the program it was built for, or the draw
  returns `DrawError::TransformFeedbackProgramMismatch`.
- `UniformType` has an `Unknown(u32)` variant for the types of vendor extensions, which no value
  can be bound to, instead of panicking at program creation.
- `TextureCreationError` of buffer textures, `CopyError`, `CopyImageError`, `ReadError`,
  `ValidationError`, `TransformFeedbackSessionCreationError` and `DrawError` have new variants.
- Every uniform of a program must have a value when drawing with it (glium#1388). Samplers,
  images, uniform blocks, shader storage blocks and atomic counter buffers must be given for every
  draw command, since their bindings are shared between programs, or the draw returns
  `DrawError::UniformBindingMissing`. Other uniforms keep their value between draw commands, but
  must have been given one once, or the draw returns `DrawError::UniformMissing`; pass
  `uniforms::Keep` to keep the initializer of the shader instead. Every stage with subroutine
  uniforms needs all of them, even when none was given. Giving a value to a member of a block by
  name returns `DrawError::UniformBlockMember` instead of panicking.
- Textures created with `new` have no mipmaps, whatever their format: use
  `with_mipmaps(facade, data, MipmapsOption::AutoGeneratedMipmaps)` to have them (glium#843).
  Mipmaps take memory and time, and textures drawn at their size don't need them. Creating
  a texture with `AutoGeneratedMipmaps` in a format whose mipmaps the backend can't generate
  returns `TextureCreationError::MipmapsNotSupported` instead of silently creating a single level,
  and `generate_mipmaps` is safe and returns that error for such textures and multisample ones.
- Drawing patches with a program that has no tessellation evaluation shader returns
  `DrawError::PatchesWithoutTessellation`, and drawing other primitives with a tessellation program
  returns `DrawError::TessellationWithoutPatches`, instead of raising `GL_INVALID_OPERATION`: the
  checks were disabled. Drawing to a `MultiOutputFrameBuffer` whose program has no output for one
  of its attachments returns `DrawError::FragmentOutputMissing` instead of panicking.
- The unsafe `unchecked_read` and `unchecked_read_to_pixel_buffer` of textures are replaced by the
  safe `read_as` and `read_to_pixel_buffer_as`: an unsupported pixel format returns
  `ReadError::OutputFormatNotSupported` instead of invalid data. Read floating-point textures
  exactly with `read_as::<_, (f32, f32, f32, f32)>()` (glium#1502).
- There can only be one `Frame` of a context at a time (glium#1005): `Frame::new` returns
  `Result<Frame, FrameError>`, `draw()` panics with a message while the previous frame isn't
  finished, and the new `try_draw()` returns `FrameError::AlreadyDrawing` instead. A second frame
  drew to the same back buffer, and swapped it a second time with undefined content.
- `Uniforms::visit_values` gives names that live as long as the uniforms (`&'a str`), so that
  SafeGL can collect the values before it starts to issue commands: implementations of `Uniforms`
  and `AsUniformValue` can now use SafeGL.
- On OpenGL ES, which has no `glShaderStorageBlockBinding`, a shader storage block is bound at the
  binding point that the shader gives it instead of one chosen by SafeGL, which raised
  `GL_INVALID_OPERATION` on drivers that follow the specification. Two storage blocks with the same
  `layout(binding = N)` that get a buffer each return the new `DrawError::SharedBindingPoint`
  there; give them different binding points.
- On OpenGL ES, which can't change the unit of an image uniform, an image is bound to the unit
  that the shader gives it with `layout(binding = N)` instead of one chosen by SafeGL with
  `glUniform1i`, which raised `GL_INVALID_OPERATION` and left the image unbound. Two image
  uniforms with the same binding return `DrawError::SharedBindingPoint` there. The reflected
  `Uniform` gets the `binding` field, the unit of the shader for images.
- SafeGL only supports the core profile of desktop OpenGL: a compatibility profile context is
  refused with `IncompatibleOpenGl`. `SimpleWindowBuilder`, and the new
  `backend::glutin::context_attributes_builder`, request the core profile explicitly. `Profile`,
  `Capabilities::profile` and `Context::get_opengl_profile` are removed. The minimum desktop
  version is OpenGL 4.0 instead of 4.1, and `IncompatibleOpenGl` displays why the context was
  refused.
- SafeGL uses desktop contexts as forward-compatible ones, which have no wide lines: a
  `line_width` above 1 pixel returns the new `DrawError::LineWidthNotSupported` on desktop
  OpenGL, and so does a width that isn't positive everywhere. OpenGL ES still draws wider lines.
- The code and APIs of OpenGL versions below 4.0 and of OpenGL ES 2 are removed:
  - `texture::{is_texture_2d_supported, is_texture_3d_supported, is_texture_2d_array_supported,
    is_cubemaps_supported}`, `framebuffer::is_dimensions_mismatch_supported` and
    `vertex::is_transform_feedback_supported`, which always returned `true`, and
    `TransformFeedbackSessionCreationError::NotSupported`;
  - `SyncFence::new` returns the fence and `per_instance()` the `PerInstance` directly, without
    `SyncNotSupportedError` and `InstancingNotSupported`, which never happened;
  - `DrawError::SamplersNotSupported`, `DrawError::TransformFeedbackNotSupported`,
    `ProgramCreationError::TransformFeedbackNotSupported` and `PointSizeNotSupported`, which
    nothing returned;
  - `AttributeType::I10I10I10I2` and `U10U10U10U2`, which only existed on OpenGL ES 2;
  - the `ExtensionsList` fields of 84 extensions that are core in every supported version, or
    only existed for OpenGL ES 2 and the compatibility profile;
  - GLSL 1.10 to 1.30, which the core profile doesn't accept, from the supported versions.

  `Context::insert_debug_marker` uses `glDebugMessageInsert` (OpenGL 4.3, OpenGL ES 3.2 or
  `GL_KHR_debug`) instead of `GL_GREMEDY_string_marker`.
- Before OpenGL ES 3.2, without `GL_EXT_geometry_shader` or `GL_OES_geometry_shader`, a draw
  with a transform feedback session must be a draw without indices of points, lines or
  triangles lists, or it returns the new `DrawError::TransformFeedbackDrawNotSupported`:
  OpenGL ES raised `GL_INVALID_OPERATION` and captured nothing.
- The Cargo features are levels, one per version: `gl40` to `gl46` for desktop OpenGL (core
  profile), `gles30` to `gles32` for OpenGL ES, and `webgl` for WebGL 2 in browsers. A level is a
  ceiling: the API of the versions above it doesn't compile, the API that only desktop OpenGL has
  needs a desktop level, and the API that only OpenGL ES has needs an OpenGL ES level. At most one
  level of each chain can be selected, a desktop and an OpenGL ES level together give both APIs,
  and without any level the level is `gl40`; no level implies another. Two levels of a chain, a
  desktop or OpenGL ES level for `wasm32-unknown-unknown`, or no `webgl` level there are compile
  errors that name the levels. The previous features change meaning: `gl46`, the default, enabled
  every API, and `gl41` and `gles30` were fallbacks. `gl46` compiles for macOS, where creating its
  context fails with a message that explains that macOS stops at OpenGL 4.1.
- `backend::glutin::level_context_apis()` replaces `tier_context_apis()`: it returns the versions
  of the selected levels, the desktop one then the OpenGL ES one, instead of falling back from
  OpenGL 4.6 to 4.1 and to OpenGL ES 3.0. `SimpleWindowBuilder` requests them.
- The API is gated by the levels that have it:
  - the desktop levels: `DrawParameters::{point_size, clip_planes_bitmask, polygon_mode,
    multisampling, time_elapsed_query, condition, smooth, provoking_vertex}`, `Depth::clamp`,
    `PolygonOffset::{point, line}`, `PolygonMode`, `Smooth`, `ProvokingVertex`, `DepthClamp`,
    `ConditionalRendering`, `SamplesPassedQuery`, `TimeElapsedQuery`, 1D textures and their
    support checks, subroutines (`UniformValue::Subroutine`, `Program::get_subroutine_uniforms`,
    `program::is_subroutine_supported`, `SubroutineData`, `SubroutineUniform`), double-precision
    and 64-bit integer uniforms and `RawUniformValue`s, 64-bit and
    `F10F11F11UnsignedIntReversed` vertex attributes, and bindless textures;
  - `gl41`: `f64` vertex attributes;
  - `gl41` or `gles30`: program binaries (`Program::get_binary`, `ComputeShader::get_binary` and
    `from_binary`, `ProgramCreationInput::Binary`, `program::Binary`, `GetBinaryError`,
    `program::is_binary_supported`);
  - a desktop level or `gles32`: geometry and tessellation shaders, the adjacency and `Patches`
    primitives, `DrawParameters::{outputs, sample_shading, primitives_generated_query}`,
    `OutputState`, `PrimitivesGeneratedQuery`, `SamplerWrapFunction::BorderClamp`,
    `Sampler::border_color`, `BorderColor`, buffer textures, and cube map array and 2D
    multisample array textures. At the other levels, the geometry and tessellation sources of
    `ProgramCreationInput::SourceCode` and `Program::from_source` have the type
    `program::NotInLevel` (see the new `program::StageSource`), which only accepts `None`;
  - a desktop level or `gles31`: `DrawParameters::sample_mask` and 2D multisample textures;
  - `gl44` or `gles32`: stencil textures; `gl44`: `SamplerWrapFunction::MirrorClamp`;
  - `gles32`: `DrawParameters::primitive_bounding_box`;
  - `gl43`, `gles30` or `webgl`: framebuffer invalidation (`invalidate`, `invalidate_rect` and
    `InvalidateAttachments`);
  - the API that needed the `gl46` feature: atomic counters and image units at `gl42` or
    `gles31`; compute shaders, shader storage blocks and `EmptyFrameBuffer` at `gl43` or `gles31`;
    multidraw indirect at `gl43`; `copy_image` at `gl43` or `gles32`; texture clears and query
    buffers at `gl44`; texture barriers and clip control at `gl45`; SPIR-V and the polygon offset
    clamp at `gl46`.

  The `DrawError`s of these features exist at the same levels. Texture formats and the other
  values that describe what the driver has stay checked at runtime, and so do the extensions.
- `DrawCommandIndices::base_vertex` is an `i32`, like the base vertex of OpenGL, so that it can
  be negative.
- At the `gl46` level, `IndicesSource::MultidrawArray` and `IndicesSource::MultidrawElement` have
  a `draw_count` field.
- The query of `ConditionalRendering` is a `ConditionQuery`, which can also be a transform
  feedback overflow query; `(&query).into()` still builds it from the samples queries, and from a
  `SamplesQueryParam`.
- `Capabilities` is `#[non_exhaustive]`, and has a `limits` field: code that builds it or
  destructures it without `..` doesn't compile anymore. Reading its fields is unchanged.
- `RenderBufferCreationError` is `#[non_exhaustive]` and has two new variants: render buffers
  larger than `Limits::max_renderbuffer_size` return `DimensionsNotSupported`, and those with more
  samples than `Limits::max_samples` or than their format has return `SamplesNotSupported`,
  instead of being created without storage and failing when drawn to.
- `Capabilities::can_lose_context: bool` is replaced by `Capabilities::reset_notification`, a
  `ResetNotificationStrategy` (`NoResetNotification` or `LoseContextOnReset`).
  `Context::is_context_loss_possible()` is unchanged.
- `DebugCallbackBehavior` has a `Log` variant (`gl43`, `gles32`), so matches on it need an arm for
  it at these levels.
- `ConditionalRendering` has an `inverted` field at the `gl45` level, and `ToBufferError` has a
  `ContextMismatch` variant: a query refuses to write to a buffer of another context.
- `DrawParameters::primitive_restart_index: bool` is replaced by `primitive_restart`, a
  `PrimitiveRestart` (`Disabled`, `FixedIndex`, or `Index(u32)` on desktop OpenGL), and
  `DrawError::FixedIndexRestartingNotSupported` is replaced by
  `PrimitiveRestartIndexNotSupported`, for `Index` on OpenGL ES: restarting at the largest index
  works on every desktop driver now.
- The third field of `VerticesSource::VertexBuffer` is an `InstanceRate` (`PerVertex` or
  `PerInstance(divisor)`) instead of a `bool`.
- `AttributeType` has the `U8U8U8U8Bgra`, `I2I10I10I10ReversedBgra` and `U2U10U10U10ReversedBgra`
  variants at the desktop levels, `VerticesSource` a `Constant` variant, and `DrawError` an
  `AttributeProvidedTwice` variant.
- The draw command buffers and their `IndicesSource` variants exist at the levels with indirect
  draws (`any(desktop, gles31)`), and the variants have a `command_offset` field; drawing all the
  commands of a buffer with `with_primitive_type` and `with_index_buffer` stays at `gl43`.
  `DrawError` has `IndirectDrawsNotSupported`, `MultidrawNotSupported` and
  `IndirectDrawWithTransformFeedback` variants, and indirect buffers can be created on OpenGL 4.0
  to 4.2 and OpenGL ES 3.1 without multidraw. `IndicesSource` also has `NoIndicesRanges` and
  `IndexBufferRanges` variants at the desktop levels, and `DrawError` `DrawRangesNotSupported`,
  `DrawRangeOutOfBounds` and `InstancedDrawRangesNotSupported` variants.
- `DrawParameters::primitives_generated_query` and `transform_feedback_primitives_written_query`
  are replaced by `primitives_generated_queries` and
  `transform_feedback_primitives_written_queries`, slices of queries of different vertex streams:
  write `&[&query]` instead of `Some(&query)`.
- `vertex::TransformFeedbackSession` is replaced by `vertex::TransformFeedback`, a transform
  feedback object built with `TransformFeedback::new(facade, program)` and given the buffers of
  the program with `with_buffer(buffer)`; `TransformFeedbackSessionCreationError` becomes
  `TransformFeedbackCreationError`. A draw without the object pauses its capture instead of ending
  it. `IndicesSource` has a `TransformFeedback` variant at the desktop levels, and `DrawError` has
  `TransformFeedbackBuffersMissing`, `TransformFeedbackPrimitivesMismatch`,
  `CapturedVerticesLost`, `CapturedVerticesOutOfBounds` and `CapturedVerticesDrawNotSupported`
  variants.
- `DrawParameters::multisampling`, `alpha_to_coverage`, `sample_shading` and `sample_mask` move
  into `DrawParameters::multisample`, a `draw_parameters::MultisampleState` with the fields
  `enabled` (formerly `multisampling`), `alpha_to_coverage`, `sample_shading` and `sample_mask`:
  write `multisample: MultisampleState { alpha_to_coverage: true, ..Default::default() }`.
  `DrawError` has an `AlphaToOneNotSupported` variant at the desktop levels.
- `UniformValue` and `uniforms::ImageUnitError` are `#[non_exhaustive]`. Multisample textures
  can be bound to image units (`image_unit()`, `image2DMS` uniforms, `gl42`; up to
  `Limits::max_image_samples` samples, beyond which `ImageUnitError::TooManySamples` is
  returned), and buffer textures too (`BufferTexture::image_unit`, `BufferImageUnit`,
  `UniformValue::BufferImage`, `imageBuffer` uniforms; `any(gl42, gles32)`), whose shader
  writes are made visible to the later uses of the buffer. `ImageUnit::new` returns
  `ImageUnitError::GetFormat` instead of panicking when the format of the texture can't be
  queried.
- `texture::buffer_texture::TextureCreationError` is `#[non_exhaustive]`, and
  `BufferTexture::from_buffer_range` builds a buffer texture that reads a range of its buffer
  (`any(gl43, gles32)`, `glTexBufferRange`; OpenGL 4.3, OpenGL ES 3.2,
  `GL_ARB_texture_buffer_range` or the texture buffer extensions of OpenGL ES at runtime), whose
  offset must be a multiple of `Limits::texture_buffer_offset_alignment`
  (`TextureCreationError::{RangeNotSupported, RangeOutOfBounds, MisalignedRange}`).
  `BufferTexture::range` returns it.
- `texture::CompressedFormat` and `CompressedSrgbFormat` are `#[non_exhaustive]`, and have the
  ETC2 and EAC formats of mobile GPUs: `Etc2Rgb8`, `Etc2Rgb8PunchthroughAlpha1`, `Etc2Rgba8Eac`,
  `EacR11Unsigned`, `EacR11Signed`, `EacRg11Unsigned` and `EacRg11Signed`, and their sRGB
  variants (`any(gl43, gles30)`; OpenGL 4.3 or `GL_ARB_ES3_compatibility` on desktop OpenGL).
- `SamplerBehavior` has a `level_of_detail` field, a `uniforms::LevelOfDetail` with the lowest
  and highest levels of detail (`GL_TEXTURE_MIN_LOD` and `GL_TEXTURE_MAX_LOD`) and, on desktop
  OpenGL, the bias added to them (`GL_TEXTURE_LOD_BIAS`), set with `Sampler::lod_range` and
  `Sampler::lod_bias`: struct literals of `SamplerBehavior` need it, or `..Default::default()`.
- `framebuffer::ValidationError` is `#[non_exhaustive]` too.
- `DrawError` is `#[non_exhaustive]`: matches on it need a wildcard arm. It has the new variants
  `LogicOpNotSupported`, `LogicOpWithBlending` and `PointCoordOriginNotSupported` at the desktop
  levels, and `DualSourceBlendingNotSupported` and `DualSourceBlendingTooManyDrawBuffers`.

Additions:

- `PipelineStatisticsQuery` counts one of the 11 statistics of `PipelineStatistic`, such as the
  invocations of the fragment shader, during the draws of
  `DrawParameters::pipeline_statistics_queries` and the dispatches of the new
  `ComputeShader::execute_with_statistics`. `TransformFeedbackOverflowQuery`, of every vertex
  stream or of one, tells whether transform feedback overflowed during the draws of
  `DrawParameters::transform_feedback_overflow_query`, and can be the condition of a draw. Both
  are at the `gl46` level and need OpenGL 4.6, `GL_ARB_pipeline_statistics_query` or
  `GL_ARB_transform_feedback_overflow_query`.
- Buffers and their slices can be filled with a value on the GPU (`glClearBufferSubData`,
  `gl43`): `clear(value)` on `Buffer`, `BufferSlice` and `BufferMutSlice`, whose content is the
  new `buffer::ClearValue`, writes its bits: integers of 8, 16 and 32 bits, `f32`, and arrays of
  2 or 4 of them or of 3 of the 32-bit ones. It needs OpenGL 4.3 or `GL_ARB_clear_buffer_object`,
  or returns `buffer::ClearBufferError::NotSupported`.
- Multidraws can read their number of commands from a buffer, which a compute shader can write
  (`glMultiDraw*IndirectCount`, `gl46`): `DrawCommandsNoIndicesBuffer::with_draw_count` and
  `DrawCommandsIndicesBuffer::with_index_buffer_and_draw_count` take a `BufferSlice<u32>` and a
  maximum number of commands, and give an `IndicesSource` whose `draw_count` is the new
  `index::DrawCount`. They need OpenGL 4.6 or `GL_ARB_indirect_parameters`, otherwise the draw
  returns the new `DrawError::DrawCountNotSupported`. `BufferType::ParameterBuffer` is the target
  of these buffers.
- `DrawParameters::base_instance` (`gl42`) draws from a base instance: the per-instance
  attributes start at this element after the start of their slices, and the shaders read it in
  `gl_BaseInstance`. It needs OpenGL 4.2 or `GL_ARB_base_instance`, or returns the new
  `DrawError::BaseInstanceNotSupported`; a draw without per-instance buffers draws one instance.
- Draws from per-instance slices that don't start at the beginning of their buffers use the
  start of the slices as base instance where the backend has base instances (OpenGL 4.2,
  `GL_ARB_base_instance` or, on OpenGL ES, `GL_EXT_base_instance`), like per-vertex slices use a
  base vertex, instead of binding the buffers at other offsets.
- With OpenGL 4.3, OpenGL ES 3.1 or `GL_ARB_vertex_attrib_binding`, vertex array objects set the
  formats of the attributes once and bind their vertex buffers with `glBindVertexBuffer`, so that
  drawing from other offsets of the same buffers reuses them and only binds the buffers again,
  instead of building a VAO per offset. `Capabilities` gives `max_vertex_attrib_bindings`,
  `max_vertex_attrib_relative_offset` and `max_vertex_attrib_stride`; formats beyond these
  limits, and WebGL, use attribute pointers as before.
- `UniformValue::DoubleMat2x3` and the other non-square double-precision matrices, from
  `[[f64; R]; C]` arrays, and 64-bit integer uniform values are bound to their GLSL types.
- `UniformValue` implements `AsUniformValue`, so a value built in advance, for example from a
  `Sampler`, can be passed to `uniform!`.
- A buffer slice that starts at an offset can be bound to a uniform block, a shader storage block
  or an atomic counter buffer, which used to panic; an offset that isn't aligned as the backend
  requires returns `DrawError::BufferOffsetNotAligned`. `Capabilities` gives the
  `uniform_buffer_offset_alignment` and `shader_storage_buffer_offset_alignment`.
- A basic type or an array matches a uniform block that has it as single member, like slices do.
- `raw_upload_from_pixel_buffer` uploads to cubemaps and cubemap arrays, whose faces are layers,
  instead of panicking.
- `implement_vertex!` works on tuple structs, whose fields are given by index with the name of
  their attribute: `implement_vertex!(Vertex, 0 => position, 1 => color normalize(true))`
  (glium#1523).
- `uniform!` accepts strings as names, for the members of structs and the elements of arrays:
  `uniform! { "lights[0].color": color }` (glium#941).
- `PrimitivesGeneratedQuery` works on OpenGL ES 3.2 and with `GL_EXT_geometry_shader` or
  `GL_OES_geometry_shader`, and `TimeElapsedQuery` with `GL_EXT_disjoint_timer_query`, instead of
  returning `QueryCreationError::NotSupported` on every version of OpenGL ES.
- `Display::rebuild(context, surface)` moves a display to another glutin context and surface,
  for example of another window, keeping its objects, and `Display::gl_context()` gives the glutin
  context to share them with.
- `Capabilities::limits` gives the implementation limits of OpenGL 4.6 and OpenGL ES 3.2 that
  `Capabilities` didn't have, in a `Limits`: those of each shader stage (`StageLimits`: uniform
  components and blocks, texture image units, inputs, outputs, image uniforms, atomic counters,
  shader storage blocks) and of all stages together, vertex attributes and varyings, element
  indices, tessellation, geometry shaders, transform feedback, viewport arrays, clip and cull
  distances, point and line sizes, samples of each kind of target, texel, gather and
  interpolation offsets, image units, block sizes, alignments, compute work groups, uniform
  locations, subroutines, binary formats, debug output, the server wait timeout and the bits of
  query counters (`QueryCounterBits`). Each limit exists at the levels whose versions have it,
  like the rest of the API, and is `None` when the driver lacks the version or extension that
  brings it. `ReleaseBehavior`, the type of `Capabilities::release_behavior`, is exported.
- `Context::graphics_reset_status()` (`gl45`, `gles32`) tells whether the GPU reset that lost the
  context was caused by it (`GraphicsResetStatus::Guilty`), by another context (`Innocent`) or by
  an unknown cause.
- `SimpleWindowBuilder::with_robustness` requests a context with robust buffer access, lost or not
  on GPU resets, or without error checking (`Robustness::NoError`), and falls back to a context
  without robustness. `Capabilities::no_error` and `Context::is_no_error()` (`gl46`) tell whether
  the context checks errors. The tests run in such contexts with `SAFEGL_ROBUSTNESS=robust`,
  `lose` or `no_error`.
- Debug output (`gl43`, `gles32`; OpenGL 4.3, OpenGL ES 3.2 or `GL_KHR_debug` at runtime):
  `Context::push_debug_group` returns a `DebugGroup` that pops the group when dropped; the
  `debug::ObjectLabel` trait gives labels to buffers, textures, render buffers, programs, compute
  shaders, queries and sync fences, and reads them back; `Context::set_debug_messages_enabled`
  enables or disables the messages that a `DebugMessageFilter` matches;
  `Context::insert_debug_message` sends a `DebugMessage` from the application or a third party;
  `DebugCallbackBehavior::Log` keeps the messages in the message log of the context, which
  `Context::debug_message_log` reads. Lengths and the depth of debug groups are checked against
  the limits, and errors are `DebugError`s. `Context::is_debug_output_supported` tells whether the
  context has all of it.
- `Context::shader_precision_format` (`gl41`, `gles30`, `webgl`) gives the range and the precision
  of a `PrecisionQualifier` (`lowp`, `mediump` or `highp` floats and integers) in the vertex or
  fragment shader.
- Queries write their result or its availability to buffers of `u32`, `i32`, `u64` or `i64`
  with `to_buffer(target, mode)` (`gl44`), where `QueryBufferMode` is the result, the result if
  it is available without waiting, or the availability. `debug::TimestampQuery::to_buffer` writes
  timestamps, and `BufferSlice::get` gives the slice of one element of an array, to write several
  results to one buffer.
- `ConditionalRendering::inverted` (`gl45`; OpenGL 4.5 or `GL_ARB_conditional_render_inverted`)
  draws when the query found no samples, or no overflow, instead.
- `SyncFence::wait_timeout` waits for a fence with a timeout and tells whether it was signaled,
  `SyncFence::wait_on_gpu` makes the GPU wait for it (`glWaitSync`), and
  `Context::gpu_timestamp` (`desktop`) reads the clock of the GPU (`GL_TIMESTAMP`).
- `PrimitiveRestart::Index` (`desktop`) restarts primitives at any index value
  (`glPrimitiveRestartIndex`), and `PrimitiveRestart::FixedIndex` works on OpenGL 4.0 to 4.2
  without `GL_ARB_ES3_compatibility`, with `GL_PRIMITIVE_RESTART` at the largest value of the
  index type.
- `DrawParameters::patch_default_levels` (`desktop`) sets the tessellation levels of programs with
  a tessellation evaluation shader and no tessellation control shader, in a
  `PatchDefaultLevels`.
- `PerInstance::with_divisor` makes each element of a per-instance buffer provide the attributes
  of several consecutive instances (`glVertexAttribDivisor`).
- BGRA vertex attributes (`desktop`): `vertex::Bgra8` and the `*Bgra` attribute types reach the
  vertex shader as RGBA `vec4`s, always normalized.
- `vertex::ConstantAttribute` gives an attribute the same value for every vertex
  (`glVertexAttrib*`), among the vertex sources of a draw: floats, signed and unsigned integers,
  and doubles at the `gl41` level, as an `AttributeValue`. Its type is checked against the
  attribute, `restore_state` sets the values again, and a constant for an attribute that a buffer
  provides too is a `DrawError::AttributeProvidedTwice`.
- Indirect draws of one command (`any(desktop, gles31)`): `command(index, primitives)` and
  `command_with_index_buffer(index, index_buffer)` on the draw command buffers draw one of their
  commands with `glDrawArraysIndirect` or `glDrawElementsIndirect`, on OpenGL 4.0 and OpenGL ES
  3.1, and a buffer of a single command draws without multidraw. Commands can be inside other
  values: `implement_draw_command!` implements `index::DrawCommandLayout` for a struct with a
  command field, and `from_buffer` makes a buffer of such structs a buffer of draw commands, read
  with the size of the struct as stride.
- Several ranges of vertices or indices drawn with one call (`desktop`): `index::NoIndicesRanges`
  (`glMultiDrawArrays`), and `IndexBuffer::ranges` and `ranges_with_base_vertices`
  (`glMultiDrawElementsBaseVertex`). The ranges are checked against the vertices and the indices.
- `PrimitivesGeneratedQuery::with_stream` and `TransformFeedbackPrimitivesWrittenQuery::with_stream`
  (`desktop`) count the primitives of a vertex stream of geometry shaders, and a draw can count
  several streams at once. A compute dispatch with statistics leaves these queries active.
- Transform feedback objects (`vertex::TransformFeedback`) capture into all the buffers of a
  program: separate buffers, the buffers that `gl_NextBuffer` separates, and the vertex streams of
  geometry shaders. Several objects capture in turn, each pausing and resuming where it was, and
  a capture survives draws and dispatches without it. `finish` returns the `CapturedVertices`,
  which draw again without reading their number back (`desktop`, `glDrawTransformFeedback*`,
  instanced at OpenGL 4.2 or with `GL_ARB_transform_feedback_instanced`): `indices(primitives)`
  for stream 0 and `stream_indices(stream, primitives)` for the others. Transform feedback
  objects are created again after `Display::rebuild`, which loses the captures.
- Programs that place their captured varyings with the `xfb_buffer`, `xfb_offset` and `xfb_stride`
  qualifiers (OpenGL 4.4 or `GL_ARB_enhanced_layouts`) reflect their transform feedback buffers,
  offsets and strides, read from the program interface.
- `DrawParameters::front_face` chooses the winding of the triangles that face the camera
  (`glFrontFace`, `FrontFace::CounterClockwise` or `Clockwise`), which `gl_FrontFacing` reads.
  `backface_culling` and the two-sided stencil settings keep naming the triangles by their
  winding.
- `MultisampleState::sample_coverage` covers a fraction of the samples of each fragment, or the
  other samples when inverted (`glSampleCoverage`, `SampleCoverage { value, invert }`), and
  `MultisampleState::alpha_to_one` writes an alpha of 1 into the samples (`desktop`,
  `GL_SAMPLE_ALPHA_TO_ONE`; drawing with it into an OpenGL ES context returns
  `DrawError::AlphaToOneNotSupported`).
- `DrawParameters::logic_op` replaces the colors written into normalized and integer color buffers
  with a bitwise operation between them and the colors already there (`desktop`, `glLogicOp`,
  `LogicOp`). Since OpenGL disables blending on those buffers, a logic operation with blending
  returns `DrawError::LogicOpWithBlending`, and one drawn into an OpenGL ES context
  `DrawError::LogicOpNotSupported`.
- `DrawParameters::point_coord_origin` chooses the corner of points where `gl_PointCoord` is
  `(0, 0)` (`desktop`, `GL_POINT_SPRITE_COORD_ORIGIN`, `PointCoordOrigin`; `LowerLeft` returns
  `DrawError::PointCoordOriginNotSupported` in an OpenGL ES context), and
  `DrawParameters::point_fade_threshold` the size under which multisampled points fade
  (`GL_POINT_FADE_THRESHOLD_SIZE`).
- Border colors of integers, for integer and stencil textures (`Sampler::border_color_int` and
  `border_color_uint`, `glSamplerParameterIiv` and `glSamplerParameterIuiv`;
  `any(desktop, gles32)`), and `BorderColor::value` with `uniforms::BorderColorValue`. A border
  color of another kind than its texture returns `DrawError::BorderColorMismatch`.
- `TextureAnyMipmap::invalidate` and `invalidate_region` discard the content of a mipmap level
  or of a box of it, a hint like framebuffer invalidation (`gl43`, `glInvalidateTexImage`,
  `glInvalidateTexSubImage`, `texture::RegionOutOfBoundsError`).
- Multisample textures can place their samples differently in each texel
  (`empty_with_sample_locations` with `fixed_sample_locations: false`,
  `GL_TEXTURE_FIXED_SAMPLE_LOCATIONS`), and `TextureAny::has_fixed_sample_locations` tells how
  they were created.
- Textures have a swizzle, which chooses the component or the constant that sampling returns in
  each channel (`TextureAny::set_swizzle`, `texture::Swizzle`; `any(desktop, gles30)`), a mipmap
  range, which limits the levels that sampling and mipmap generation use
  (`TextureAny::set_mipmap_range`, `GL_TEXTURE_BASE_LEVEL` and `GL_TEXTURE_MAX_LEVEL`, every
  level), and depth-stencil textures can be sampled for their stencil index
  (`TextureAny::set_depth_stencil_mode`, `texture::DepthStencilTextureMode`; `any(gl43, gles31)`,
  OpenGL 4.3, OpenGL ES 3.1 or `GL_ARB_stencil_texturing` at runtime) through `usampler*`
  uniforms.
- `DrawParameters::viewports` gives a draw several viewports, each with its depth range and
  scissor box, among which geometry shaders choose with `gl_ViewportIndex` (`gl41`,
  `glViewportArrayv`, `glDepthRangeArrayv`, `glScissorArrayv`, `ViewportState`; OpenGL 4.1 or
  `GL_ARB_viewport_array` at runtime). Viewports keep fractions of pixels.
- `framebuffer::LayeredFrameBuffer` attaches whole texture levels with their layers (array
  textures, cube maps, cube map arrays and 3D textures), into which geometry shaders send each
  primitive with `gl_Layer`, such as the six faces of a shadow cube map in one draw
  (`any(desktop, gles32)`, `glFramebufferTexture`; OpenGL ES 3.2 or `GL_EXT_geometry_shader` at
  runtime). It has one color attachment, several named outputs (`with_outputs`), depth and
  stencil attachments (`LayeredDepthStencil`) or only a depth attachment, and the layers of its
  attachment that has the fewest (`layers`). Clears clear every layer and blits read layer 0
  (`Surface::blit_from_layered_framebuffer`). The mipmaps of the textures with layers and the
  textures themselves implement `ToLayeredColorAttachment` and its depth and stencil variants.
  `ValidationError` has the `LayeredFramebuffersNotSupported` and `LayerTargetsMismatch`
  variants: the color attachments must be textures of the same kind.
- `sample_positions` on `SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `DefaultFramebuffer`,
  `ExternalFramebuffer` and `Frame` returns where the GPU places the samples inside the pixels
  (`glGetMultisamplefv(GL_SAMPLE_POSITION)`, `any(desktop, gles31)`), or nothing if the
  framebuffer isn't multisampled.
- `Limits::max_cull_distances` and `max_combined_clip_and_cull_distances` exist at the OpenGL ES
  levels too, `Some` with `GL_EXT_clip_cull_distance`, under which shaders write
  `gl_CullDistance`.
- `Program::get_frag_data_index` returns the index of a fragment output, 1 for the second color
  output of dual-source blending (`glGetFragDataIndex`), and `Limits::max_dual_source_draw_buffers`
  exists at every level, `Some` on OpenGL ES with `GL_EXT_blend_func_extended`.

Fixes:

- Stencil and depth-stencil textures can be used as texture uniforms and sampled, by `usampler*`
  uniforms for stencil textures and by `sampler*` and `sampler*Shadow` uniforms for the depth
  of depth-stencil textures, always with the nearest filters for stencil indices. They had no
  uniform value and no `sampled()`.
- A face of a cube map array can be attached to framebuffers (and read) without direct state
  access, on OpenGL ES and below OpenGL 4.5, instead of panicking.
- The color of the fragment output at location n is attached at `GL_COLOR_ATTACHMENTn`. A
  `MultiOutputFrameBuffer` whose outputs weren't listed in the order of their locations made
  `glDrawBuffers` fail on OpenGL ES, which only accepts `GL_COLOR_ATTACHMENTn` or `GL_NONE` as the
  n-th draw buffer, and drew nothing.
- Dual-source blending (the `*SourceOne*` factors of `LinearBlendingFactor`) returns
  `DrawError::DualSourceBlendingNotSupported` on OpenGL ES without `GL_EXT_blend_func_extended`
  and on WebGL, and `DrawError::DualSourceBlendingTooManyDrawBuffers` when the framebuffer has
  more draw buffers than `Limits::max_dual_source_draw_buffers`, instead of OpenGL errors.
- The columns of a matrix attribute whose components aren't 32-bit floats, such as
  `AttributeType::F16x4x4` fed to a `mat4`, are read at the offsets of their components instead of
  4 bytes per component.
- On OpenGL ES, `clip_planes_bitmask` enables clip distances with `GL_EXT_clip_cull_distance`,
  and enabling a plane without it returns `DrawError::ClipPlaneIndexOutOfBounds` instead of being
  ignored.
- `SamplerWrapFunction::MirrorClamp` falls back to `Clamp` on desktop OpenGL below 4.4 without
  `GL_ARB_texture_mirror_clamp_to_edge`, instead of raising `GL_INVALID_ENUM`.
- Buffer slices are bound with their own range to uniform blocks, shader storage blocks, atomic
  counter buffers and transform feedback, instead of the whole buffer.
- Programs using cube map array images, `samplerCubeArrayShadow`, `usampler1DArray`, 64-bit
  integers, double-precision matrix inputs or half-float inputs no longer panic at creation.
- Creating a `Texture2dMultisampleArray` no longer raises `GL_INVALID_ENUM`.
- A transform feedback session can be used by several draws in a row, which append their
  vertices, instead of panicking on the second one; transform feedback ends before the program
  changes and before compute dispatches, where it raised `GL_INVALID_OPERATION`.
- Indexed draws bind every vertex buffer even when they don't have the same length.
- Running out of texture units returns `DrawError::InsufficientTextureUnits` instead of panicking.
- Double-precision uniforms work on OpenGL 4.0 and above without `GL_ARB_gpu_shader_fp64` in the
  extensions list, instead of panicking.
- Empty framebuffers with a width, height, number of layers or number of samples of 0 return
  `ValidationError::EmptyFramebufferUnsupportedDimensions` instead of panicking.
- `BufferTexture::from_buffer` returns `TextureCreationError::NotSupported` without buffer
  textures instead of panicking.
- Conditional rendering with a query that was never used draws nothing, since the query counted
  no samples, instead of raising `GL_INVALID_OPERATION`.
- Drawing with the same query as `samples_passed_query` and as the condition returns
  `DrawError::WrongQueryOperation` instead of raising an OpenGL error.
- `to_buffer_u32` on a query that was never used writes 0, the result `get()` returns, instead of
  panicking.
- A `TimestampQuery` that is dropped without being read deletes its query object instead of
  leaking it.
- The documentation of `PrimitivesGeneratedQuery` says that it counts the primitives of the last
  vertex processing stage, without a geometry shader too.
- `slice(start..end)` on buffers returns `None` when `start > end`, instead of a slice whose size
  overflows.
- Reading a buffer into a `Vec` no longer exposes uninitialized memory to the closure of
  `Content::read`, and `Content::to_void_ptr` no longer panics on an empty slice.
- SafeGL can be used from the closure of `exec_in_context`, which no longer holds the context
  while it runs, and SafeGL's vertex array object is unbound before it so that the closure can't
  change it (glium#615). Validating a framebuffer no longer borrows the context a second time.
  Calling SafeGL while it is issuing commands, for example from a debug callback, panics with a
  message instead of `BorrowMutError`.
- The debug callback is called with a shared reference to the context instead of a mutable one
  made from a shared pointer, which was undefined behavior. A panic of the callback, which
  aborted the process since it can't unwind through OpenGL, is resumed when the command that
  triggered the message returns, and a callback that triggers messages itself isn't called again
  for them.
- Querying the depth and stencil bits of a render buffer no longer leaves SafeGL's cache of the
  bound render buffer out of date.
- On desktop OpenGL, the fragment outputs that have no `layout(location = N)` get locations in
  their declaration order, so that the first one is written to targets with a single color buffer
  on every driver instead of one chosen by the linker (glium#285).
- Reading a texture on OpenGL ES in another pixel format than the one it guarantees for the kind
  of texture, like the `(u8, u8, u8, u8)` of `read()` for a floating-point texture, converts the
  pixels on the CPU instead of failing (glium#1502).
- Compressed data whose width or height isn't a multiple of 4 is sized by whole 4x4 blocks
  instead of panicking (glium#410), and compressed data for one-dimensional or three-dimensional
  textures returns `FormatNotSupported` instead of panicking. `ClientFormatAny::get_num_components`
  works for compressed formats instead of panicking.
- Reading a texture into a pixel buffer that is too small returns `ReadError::BufferTooSmall`
  instead of panicking.
- Writing to a range of a buffer waits for all the fences of the GPU commands that use it: a
  fence that covered the whole range, like the one of a draw command that used the whole buffer,
  was missed, so a write to a persistent-mapped buffer could race with the GPU (glium#1789).
- `restore_state` also forgets the vertex array objects of SafeGL, whose element array buffer or
  attributes the other code may have changed while one was bound, and the uniform values and
  block bindings that the programs cached, which the other code may have changed too (glium#1696,
  glium#1867, glium#1417).
- The documentation explains the stalls caused by writing to buffers that the GPU still reads
  (glium#1789), and the CPU use of the NVIDIA driver without synchronous debug output
  (glium#1927).
- Blits check the rules that made OpenGL raise `GL_INVALID_OPERATION` and copy nothing
  (glium#653, glium#1748): they return `BlitError::ScaledMultisample`, `SampleCountMismatch`,
  `MultisampleTarget` (OpenGL ES), `FormatMismatch`, `LinearFilterOnIntegers` or
  `SameSourceAndTarget` (OpenGL ES). A framebuffer object of SafeGL is read from its first color
  attachment, instead of the read buffer left by a previous read. The documentation of `fill` says
  which way it copies.
- Tessellation works on OpenGL ES 3.2, where it is core, and on OpenGL ES with
  `GL_EXT_tessellation_shader` or `GL_OES_tessellation_shader`, whose `glPatchParameteri` has
  another name: SafeGL didn't detect it and returned `DrawError::TessellationNotSupported`.
- The reflection of a uniform or shader storage block lists the members of its structs in the
  order of their declaration, instead of the order of the driver (alphabetical on NVIDIA).
- The binding points of atomic counters are reflected with `glGetProgramResourceiv` on OpenGL ES,
  which has no `glGetActiveAtomicCounterBufferiv`: the counters were bound at binding point 0,
  whatever their `layout(binding = N)`.
- Faces that are counter-clockwise on the target used the stencil write mask of the clockwise
  faces when the two masks differ.
- Writing a query result into a buffer that a shader wrote to issues
  `glMemoryBarrier(GL_QUERY_BUFFER_BARRIER_BIT)` according to the previous query-buffer barrier,
  instead of the previous pixel-buffer barrier, which skipped or repeated it.
- The documentation of sRGB said that SafeGL enables `GL_FRAMEBUFFER_SRGB` by default, while the
  programs of `Program::from_source` and `program!` output sRGB and draw with it disabled. It
  now explains both values of `outputs_srgb`, `clear_color_srgb`, and the conversion that OpenGL
  ES can't turn off without `GL_EXT_sRGB_write_control` (glium#1115).
- `Context::rebuild` compares the version and the extensions of the new context with the first
  one: it compared the values that glow read from the first context, so it never refused a
  context. It also sets up the debug output and the seamless cube maps of the new context, and
  turns off the debug output of the previous one, whose messages would have reached a SafeGL
  context that could be gone. `Backend::create_glow_context` is called again for the new context.
- With the default `DebugCallbackBehavior::DebugMessageOnError`, the driver no longer generates the
  debug messages of low severity and the notifications, which the callback ignores: the NVIDIA
  driver formatted one for every buffer it created, and crashed doing it once while another thread
  created a context.
- Vertex attributes of type `F10F11F11UnsignedIntReversed` require OpenGL 4.4 or
  `GL_ARB_vertex_type_10f_11f_11f_rev`, and double-precision attributes OpenGL 4.1 or
  `GL_ARB_vertex_attrib_64bit`, instead of being accepted by every desktop context. Fixed-point
  attributes (`FixedFloatI16U16`) and program binaries require OpenGL 4.1 or one of
  `GL_ARB_ES2_compatibility` / `GL_ARB_get_program_binary` on desktop OpenGL.
- On OpenGL ES 3.2, or with the geometry shader and tessellation extensions, index buffers of
  adjacency primitives and patches are accepted, and tessellation shaders are created with
  `GL_EXT_tessellation_shader` too: only `GL_OES_tessellation_shader` was checked.
- Multi-draw indirect draws use the `...EXT` entry points of `GL_EXT_multi_draw_indirect` on
  OpenGL ES, instead of the desktop ones that the driver doesn't have.
- GLSL 4.60 is among the supported GLSL versions on OpenGL 4.6, so `program!` can choose it.
- `Capabilities::max_texture_buffer_size` is given on OpenGL ES 3.2, where buffer textures are
  core, and the sRGB encoding of the default framebuffer is read from its attachment on every
  context, instead of `GL_FRAMEBUFFER_SRGB_CAPABLE_EXT`, which the core profile doesn't have.
- Creating a `BufferTexture` without buffer texture support (OpenGL ES 3.0 and 3.1 without
  `GL_EXT_texture_buffer`) returns `TextureCreationError::NotSupported`, instead of the
  `BufferTypeNotSupported` of the buffer it starts with.
- Anisotropic filtering works on OpenGL 4.6, where it is core, and with
  `GL_ARB_texture_filter_anisotropic`: `Capabilities::max_texture_max_anisotropy` was `None` and
  samplers ignored their anisotropy unless the driver listed `GL_EXT_texture_filter_anisotropic`.
- Checking whether the context is lost calls the `glGetGraphicsResetStatus` of the context: on
  OpenGL ES 3.0 and 3.1 with `GL_KHR_robustness` it is `glGetGraphicsResetStatusKHR`, while SafeGL
  called the unsuffixed name, which panics when the driver doesn't export it.
  `Capabilities::can_lose_context` reads the reset strategy of OpenGL ES 3.2 contexts without
  robustness extensions, and an unknown strategy is reported as `false` instead of panicking.
- Programs whose interleaved transform feedback varyings contain `gl_SkipComponents1` to `4` or
  `gl_NextBuffer` are created, with their buffers, offsets and strides reflected, instead of
  panicking on these names.
- The debug callback receives the messages with the length the driver gives: the messages of
  debug groups and inserted messages aren't null-terminated with some drivers, which made the
  callback read past them.
- `Context::insert_debug_marker` works on OpenGL ES 3.0 and 3.1 with `GL_KHR_debug`, instead of
  returning `Err` there.
- `debug::TimestampQuery::get` and `is_ready` unbind the query buffer first: after a query wrote to
  a buffer, they made OpenGL write the timestamp in that buffer and returned 0.
- `is_ready` returns `true` for a query that never began, whose result `get` returns at once,
  instead of `false` forever.
- Memory barriers cover every use of what shaders write: buffer copies, transform feedback
  buffers and buffer textures after storage writes, textures drawn to or read through
  framebuffers, uploaded, downloaded or given mipmaps after image stores (also at the `gl42` and
  `gles31` levels, where only clears and copies had the barrier), and persistent mappings, which
  now wait for their barrier before the CPU reads them.
- `restore_state` restores `GL_PRIMITIVE_RESTART_FIXED_INDEX` on OpenGL 4.0 to 4.2 with
  `GL_ARB_ES3_compatibility`, where SafeGL uses it.

## [0.2.0]

Additions:

- `DrawParameters::alpha_to_coverage`, `sample_shading` (`glMinSampleShading`) and `sample_mask`
  (`glSampleMaski`), with the `DrawError::SampleShadingNotSupported` and `SampleMaskNotSupported`
  errors.
- `DrawParameters::outputs` and `OutputState`: blending and color mask per fragment shader output
  (`glBlendFuncSeparatei`, `glColorMaski`), with the `DrawError::PerOutputStateNotSupported` and
  `OutputStateIndexOutOfBounds` errors. Clearing a framebuffer ignores the per-output masks as it
  does the global one, and so does `raw_clear_buffer` on textures, which used to obey the color
  mask left by the previous draw.
- `DrawParameters::polygon_offset_clamp` (`gl46`, `glPolygonOffsetClamp`), with the
  `DrawError::PolygonOffsetClampNotSupported` error.
- `texture::copy_image` and `copy_to()` on mipmaps (`gl46`, `glCopyImageSubData`): copies between
  textures and render buffers, with format, sample count, bounds, block alignment and overlap
  checks.
- `clear()`, `clear_region()` and `clear_depth_stencil()` on mipmaps (`gl46`, `glClearTexImage`):
  clears any kind of texture without a framebuffer.
- `texture::texture_barrier` (`gl46`, `glTextureBarrier`), which makes reading a texture attached
  to the framebuffer being drawn to well defined between barriers.
- `invalidate()` and `invalidate_rect()` on `SimpleFrameBuffer`, `MultiOutputFrameBuffer`,
  `DefaultFramebuffer` and `Frame`, with `framebuffer::InvalidateAttachments`
  (`glInvalidateFramebuffer`). They do nothing where invalidation isn't supported.
- Non-square float matrix uniforms: `[[f32; R]; C]` values bind to `matCxR` uniforms, through the
  new `UniformValue` and `RawUniformValue` variants `Mat2x3`, `Mat2x4`, `Mat3x2`, `Mat3x4`,
  `Mat4x2` and `Mat4x3` (glium#1708, based on glium PR #2133).
- `backtrace` feature, enabled by default, for the backtrace printed with OpenGL debug messages
  reported as errors (glium#1707, based on glium PR #2155).
- `DrawError::BufferBindingOutOfRange`, returned when a block needs a buffer binding point beyond
  the backend's limit.
- `Context::restore_state`, which makes the OpenGL state match SafeGL's cache after code sharing
  the context changed it (glium#2106, glium#1505, glium#1443).
- `framebuffer::ExternalFramebuffer`, a framebuffer object created outside SafeGL (by a GUI
  toolkit, for example) that SafeGL can draw to, clear, blit to and from, and invalidate, with the
  `Surface::blit_from_external_framebuffer` and `blit_buffers_from_external_framebuffer` methods
  (glium#2153, glium#2108). `SimpleFrameBuffer::get_framebuffer_object` returns the framebuffer
  object SafeGL draws through (glium#1613).
- `ReadError::AttachmentMissing` and `ReadError::ContextLost`.
- `framebuffer::ValidationError::Incomplete`, `DrawError::NoStencilBuffer`,
  `DrawError::GeometryShaderInputMismatch` and `Stencil::requires_stencil_buffer`.
- `implement_vertex!` accepts instances of generic structs, like `Vertex<f32>` (glium#1509).
- `UncompressedFloatFormat::U5U6U5`, the RGB565 format (based on glium PR #2152).
- `SamplerBehavior::border_color`, `Sampler::border_color` and `uniforms::BorderColor`, for
  `SamplerWrapFunction::BorderClamp` (based on glium PR #2120).
- `From<Sampler<'a, T>> for UniformValue<'a>`, a uniform value that borrows the texture instead
  of the sampler (glium#1850).
- Texture writes beyond 2D textures (glium#2138, glium#2058, glium#1284, glium#644, based on glium
  PR #1198): `TextureAnyImage::raw_write` writes a rectangle of one image (a layer of an array
  texture, a face of a cubemap or a slice of a 3D texture); 1D and 3D textures and their mipmaps get
  `write` methods taking the offset of the data; layers of 1D and 2D array textures and cubemap
  faces get `write` methods; and 2D integer, unsigned and depth textures get `write` too.
- Depth, stencil and depth-stencil images can be read (as `f32`, `u16` or `u32` depth values, `u8`
  stencil values, or `u32` packing 24 bits of depth and 8 of stencil), and `DepthTexture2d` gets a
  `read` method (glium#1856). OpenGL ES still needs the `GL_NV_read_*` extensions.
- Examples: `outputs_srgb` (based on glium PR #2098), `headless` (glium#2091),
  `alpha_to_coverage`, which uses the CC0 "Fern 02" model from Poly Haven, `shadow_bias`,
  `programmable_blending` and `weighted_oit`.

Fixes:

- Multisample textures can be created and attached to framebuffers on OpenGL ES 3.1+, where they
  reached `unreachable!()`.
- Texture creation on OpenGL ES (glium#1730, glium#2017, glium#1906, glium#1482, glium#848,
  glium#1151, based on glium PRs #1731 and #1488):
  - textures use `glTexStorage*` on OpenGL ES 3.0+ and WebGL 2, so empty float, integer, depth and
    depth-stencil textures no longer raise `GL_INVALID_OPERATION`;
  - integer, depth and depth-stencil formats are reported as supported on OpenGL ES 3.0, so
    `empty_with_format` and depth render buffers no longer fail with `FormatNotSupported`;
  - formats that OpenGL ES doesn't have (16-bit normalized without `GL_EXT_texture_norm16`, 12-bit
    and other legacy formats, 32-bit integer depth) are reported as unsupported instead of failing
    in the driver;
  - textures created with data get a storage format that matches the data's components (and, for
    integer textures, its size), and depth textures created with floats get a float format;
  - textures created without data pass a format and type that match the internal format;
  - generic compressed textures, which OpenGL ES doesn't have, are created uncompressed;
  - automatically generated mipmaps are only created for formats OpenGL ES can generate them for
    (glium#1178); other textures get a single level;
  - float, half-float and 16-bit normalized formats are color-renderable with
    `GL_EXT_color_buffer_float`, `GL_EXT_color_buffer_half_float` and `GL_EXT_texture_norm16`;
  - cube map arrays and multisample texture arrays are core in OpenGL ES 3.2, and multisample
    texture arrays use `GL_OES_texture_storage_multisample_2d_array` before that;
  - integer and float textures can be read back as `(i32, i32, i32, i32)`, `(u32, u32, u32, u32)`
    and `(f32, f32, f32, f32)`, the formats OpenGL ES allows for them.
- Multisample textures use `glTexStorage2DMultisample` only on OpenGL 4.3 and with
  `GL_ARB_texture_storage_multisample`, instead of with `GL_ARB_texture_storage`, which panicked on
  OpenGL 4.1 drivers such as macOS (glium#1840).
- Creating a texture of a type the context doesn't have (1D textures on OpenGL ES, for example)
  returns `TextureCreationError::TypeNotSupported`, and dimensions above the context's limits
  return `DimensionsNotSupported`, instead of raising an OpenGL error (glium#1163, glium#1555,
  glium#74).
- Color render buffers check that their format is color-renderable (glium#505).
- `raw_upload_from_pixel_buffer_inverted` reports that OpenGL ES can't flip the components instead
  of raising an OpenGL error.
- Uniform buffers and transform feedback work on OpenGL ES 3.0, and shader storage and atomic
  counter buffers on OpenGL ES 3.1, where binding them panicked because their binding point limits
  were 0. Buffers are read back through a mapping on OpenGL ES, which has no `glGetBufferSubData`
  (glium#2112, based on glium PR #1700).
- Binding an atomic counter buffer at a point several entries above the previous ones no longer
  underflows, and running out of binding points returns `DrawError::BufferBindingOutOfRange`
  instead of panicking.
- `implement_buffer_content!` no longer has undefined behavior (null references, a `Box` with the
  wrong length and alignment) and works with generic structs (glium#1949, glium#1918). The code it
  generates uses `std::mem::size_of_val_raw`, so crates that call it need Rust 1.99.
- The name of a uniform array refers to its first element, as in OpenGL, and every element gets its
  own location instead of an assumed one, which was another uniform's location on WebGL
  (glium#288, based on glium PR #666).
- Debug builds no longer assert that the context is current right after making it current, which
  fails on some drivers such as macOS 10.14+ (based on glium PR #1724).
- The backtrace of OpenGL debug messages goes to the standard error, like the message
  (glium#1397).
- `Context::rebuild` returns an error, and keeps the old context, if the new context has another
  version or other extensions, instead of keeping stale ones (glium#563).
- `read_front_buffer` returns `ReadError::AttachmentMissing` where the default framebuffer has no
  front buffer, like most EGL window surfaces, instead of raising an OpenGL error (glium#2151).
- Blitting from a `MultiOutputFrameBuffer` reads from its first output instead of reaching
  `unimplemented!()`.
- Reading textures back no longer pads the rows when their size isn't a multiple of
  `GL_PACK_ALIGNMENT` (rows of single-byte or three-byte pixels, for example), which wrote past the
  end of the destination and returned misplaced texels.
- `from_id` with generated mipmaps binds the texture before generating them, instead of generating
  the mipmaps of whatever texture was bound (glium#1463).
- `get_internal_format` reports the format of stencil textures (glium#1240).
- `clear_color` clears integer and unsigned color attachments with `glClearBuffer`, since
  `glClearColor` doesn't apply to them (glium#498).
- Clearing the stencil buffer ignores the stencil write mask of the previous draw call
  (glium#2148).
- Drawing instances from a slice of vertices that doesn't start at the first one no longer shifts
  or underflows the per-instance buffers (glium#1507).
- `VertexBuffer::new_raw` and `new_raw_dynamic` use the given vertex size as the stride instead of
  ignoring it, and reject data that isn't a whole number of vertices (glium#2028, glium#1461).
  `VertexBufferAny::into_vertex_buffer` is implemented instead of reaching `unimplemented!()`
  (glium#1565).
- A vertex format that doesn't match the program returns `DrawError::AttributeTypeMismatch` or
  `DrawError::AttributeMissing` instead of panicking (glium#1434).
- Drawing from many slices of a buffer no longer creates vertex array objects without bound: only
  the 16 most recently used ones of each combination of buffers and program are kept (glium#2001).
- The columns of normalized matrix attributes are offset by the size of their components.
- Transform feedback captures the triangles of tessellated quads instead of panicking (glium#935).
- Reading a persistent mapping after shader writes issues `GL_CLIENT_MAPPED_BUFFER_BARRIER_BIT`
  (glium#1028).
- Framebuffers check `glCheckFramebufferStatus` when their framebuffer object is created, and
  `SimpleFrameBuffer`, `MultiOutputFrameBuffer` and `EmptyFrameBuffer` return
  `ValidationError::Incomplete` for attachments OpenGL doesn't support, like non-color-renderable
  formats, instead of raising OpenGL errors when drawing (glium#44, glium#1396).
- Stencil tests and operations on a surface without a stencil buffer return
  `DrawError::NoStencilBuffer` (glium#718).
- Blits copy the pixels without sRGB conversion, instead of depending on the `GL_FRAMEBUFFER_SRGB`
  state left by the previous clear or draw call (glium#1671).
- Reading pixels after the context was lost returns `ReadError::ContextLost` (glium#1014).
- The members of uniform and shader storage blocks declared with an instance name match the fields
  of the Rust struct (glium#2125).
- Drawing primitives that the geometry shader doesn't take returns
  `DrawError::GeometryShaderInputMismatch` instead of raising an OpenGL error (glium#850,
  glium#1460, glium#1575).
- Integer and unsigned textures are sampled with the nearest filters, since linear filters, like
  those of the default sampler, make them read as zero (glium#1515, glium#1420).
- The message of `DrawError::UniformTypeMismatch` names the expected type correctly (glium#1600).
- `GL_MAX_CLIP_DISTANCES` is only queried when the clip planes change.
- `SimpleWindowBuilder::build` no longer panics when the driver refuses the swap interval
  (glium#1929).
- `implement_uniform_block!` and `program!` work when called by their path, like
  `safegl::implement_uniform_block!`, without importing them.
- The `manual-creation` example resizes its surface with the window (glium#2090).
- Draw and compute commands keep their buffer fences inline instead of allocating a `Vec`
  (based on glium PR #2057).

## [0.1.0]

First release under the `safegl` name, forked from glium 0.36.0. SafeGL now issues OpenGL calls
through [glow](https://github.com/grovesNL/glow) and can run in the browser on WebGL 2. Breaking
changes compared to glium:

- **Crate renamed:** the crate is now published as `safegl` instead of `glium`, so paths become
  `safegl::...` and `#[macro_use] extern crate safegl;`. Other renames: `GliumEventLoop` is now
  `SafeGlEventLoop`, and the test harness reads `SAFEGL_GL_VERSION` instead of `GLIUM_GL_VERSION`.
- **Bindings:** the generated `gl_generator` bindings are gone. GL calls go through `glow`, plus a
  small private table (`src/gl.rs`) for the entry points glow doesn't provide yet.
- **Minimum versions:** creating a context below desktop OpenGL 4.1 or OpenGL ES 3.0 now fails with
  `IncompatibleOpenGl`. Code paths for older versions and their extensions (ARB shader objects,
  EXT framebuffer objects, APPLE/OES vertex arrays, APPLE sync, ...) were removed.
- **Feature tiers:** new Cargo features `gl46` (default), `gl41`, `gles30` and `webgl`. Enabling
  `gl46` on macOS, or a tier above `gles30` for `wasm32-unknown-unknown`, is a compile error.
- **APIs above OpenGL 4.1 need `gl46`:** compute shaders, shader storage buffers, atomic counters,
  image load/store, multidraw indirect, empty framebuffers, query buffers, clip control, SPIR-V
  shaders and bindless textures are only compiled with the `gl46` feature. This includes
  `ARB_bindless_texture` and `ARB_clip_control`, even though some 4.1 drivers have them as
  extensions. `gl46` builds keep the runtime checks for older drivers. See "APIs that need `gl46`"
  in the README for the full list of gated items. The `compute_image`, `gpgpu` and `spirv` examples
  require `gl46`.
- **Object ids:** `GlObject::get_id()` returns glow handle types (`glow::Buffer`, `glow::Texture`,
  `glow::Program`, ...) instead of `GLuint`, and `from_id` takes a `glow::Texture`. The public
  `Handle` enum was removed. `glow` is re-exported as `safegl::glow`.
- **Backend trait:** `Backend::set_swap_interval` takes `safegl::backend::SwapInterval` instead of
  glutin's type. New methods `create_glow_context` (provided on native targets) and
  `is_context_lost` (defaults to `false`). `get_proc_address` only exists on native targets.
- **Context creation:** `SimpleWindowBuilder` requests the newest context version of the compiled
  tier, then falls back (4.6 → 4.1 → OpenGL ES 3.0). See
  `safegl::backend::glutin::tier_context_apis`.
- **Vulkan interop removed:** `memory_object`, `semaphore`, texture import from file descriptors
  (`new_from_fd`), `UuidError` and the `vk_interop` feature.
- **Direct state access:** `EXT_direct_state_access` is no longer used, and a few OpenGL 4.5 DSA
  paths that glow can't express now bind objects instead.

Additions:

- WebGL 2 backend: `safegl::backend::web::{WebDisplay, WebGlBackend}` behind the `webgl` feature,
  with an example in `examples/web`.
- `Context::glow_context()` gives access to the underlying `glow::Context` (for example to share it
  with `egui_glow`).
- `SyncFence::is_signaled()` checks a fence without blocking.

Fixes:

- Primitive restart returns `FixedIndexRestartingNotSupported` on OpenGL 4.1/4.2 drivers without
  `ARB_ES3_compatibility` instead of issuing an invalid enum.
- Uniform names longer than 62 bytes are no longer truncated during program reflection.
- Copying between buffers no longer overwrites the source binding when the source is already bound
  to `COPY_WRITE_BUFFER`.

---

The releases below were published as `glium`, before the fork.

## Version 0.36.0 (2024-10-11)

- Fix UB around FDs being closed
- Fix static mut ref warnings in tests
- Make `SimpleWindowBuilder` accept `ActiveEventLoop`

## Version 0.35.0 (2024-08-04)

- Updated glutin to version 0.32.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0320).

## Version 0.34.0 (2024-01-03)

- Updated glutin to version 0.31.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0310).

## Version 0.33.0 (2023-11-01)

- Add `read` methods for `Unsigned` textures
- Add `gl::HALF_FLOAT` to `bind_attribute`
- Lowered version check for `gl_PointSize` from 3.0 to 2.0
- Added Dynamic Uniform based on `HashMap`.
- Updated glutin to version 0.30.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0300).

## Version 0.32.1 (2022-07-31)

- Bugfix release to not panic when given multiple vertex attributes with unspecified location numbers.

## Version 0.32.0 (2022-07-30)

- Updated glutin to version 0.29.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0290-2022-07-30).
- Support for location binding for Vertex attributes.

## Version 0.31 (2021-12-11)

- Updated glutin to version 0.28.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0280-2021-12-02).
- Use mdbook for the book.

## Version 0.30.2 (2021-09-06)

- Added depth and stencil buffer blitting.
- Added dual source blending.
- Implemented AsUniformValue for Texture2d directly.

## Version 0.30.1 (2021-07-12)

- Added `ClientFormat::U1U5U5U5Reversed`.
- Updated `ouroboros` to 0.10.

## Version 0.30.0 (2021-06-23)

- Updated glutin to version 0.27.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0270-2021-06-01).
- Replaced unmaintained `rental` with `ouroboros`.
- Allow instancing on GLES3 or later.
- Support for importing and using Vulkan semaphores.
- Made `Content::read` an unsafe API.
- Modernized and fixed the third chapter of the tutorial.

## Version 0.29.1 (2021-04-22)

- Updated rand to 0.8.
- Updated cgmath to 0.18.
- Made `UniformsStorage` implement `Copy` and `Clone`.
- Use `$crate::uniform!` internally.
- Implemented `From` instead of `Into` for some conversions.

## Version 0.29.0 (2020-12-15)

- Updated glutin to version 0.26.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0260-2020-12-10).
- Updated memoffset to 0.6.
- Added support for `glPolygonOffset`.
- Added support for loading SPIR-V shaders.
- Switched to edition 2018.
- Switched from Travis CI to Github Actions

## Version 0.28.0 (2020-10-03)

- Updated glutin to version 0.25.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0250-2020-10-02).
- Implemented atomic counters.
- Added `CubeMapArrays` support.

## Version 0.27.0 (2020-03-26)

- Updated glutin to version 0.24.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0240-2020-03-11).

## Version 0.26.0 (2020-02-09)

- Updated glutin to version 0.23.0. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0230-2020-02-06).
- Removal of some unsound code that rustc warns about
- Report the precise shader stage in which a shader failed compilation

## Version 0.25.1 (2019-07-08)

- Silenced deprecation warning when invoking `implement_vertex`.

## Version 0.25.0 (2019-05-24)

- Updated glutin to version 0.21.

## Version 0.24.0 (2019-04-08)

 - Updated glutin to version 0.20. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0200-2019-03-09).
 - Depth comparison and shadow mapping ([#1679](https://github.com/glium/glium/pull/1679)).

## Version 0.23.0 (2018-12-05)

 - Updated glutin to version 0.19. See the glutin release notes [here](https://github.com/rust-windowing/glutin/blob/master/CHANGELOG.md#version-0190-2018-11-09).

## Version 0.22.0 (2018-07-02)

 - Updated glutin to version 0.17.
 - Updated glutin to version 0.16. Added 'icon_loading' feature.

## Version 0.21.0 (2018-04-11)

 - Updated glutin to version 0.14. Fixes handling of HiDPI on macOS.
 - Updated gl_generator to version 0.9.

## Version 0.20.0 (2018-01-22)

 - Updated glutin to version 0.12.
 - Updated smallvec from version 0.4 to 0.6.
 - Updated misc internal dependencies and dev-dependencies (lazy_static, cgmath, rand, image, gl_generator).
 - Replaced the `IntoVerticesSource` trait with `Into<VerticesSource>`.
 - Fixed [rendering bug](https://github.com/glium/glium/issues/1657) on high-DPI screens.
 - Added support for clipping via `gl_ClipDistance`.
 - Enabled [depth-stencil FBO attachments](https://github.com/glium/glium/issues/253).

## Version 0.19.0 (2017-12-11)

 - Updated glutin to version 0.11. Notably includes the [winit 0.9 update](https://github.com/rust-windowing/winit/blob/master/CHANGELOG.md#version-090-2017-12-01).
 - Updated gl_generator to 0.7 (internal dependency).

## Version 0.18.1 (2017-11-05)

 - Fixed links pointing to tomaka/glium instead of glium/glium
 - Various documentation updates, bugfixes, and dependency updates.

## Version 0.18.0 (2017-10-15)

 - Updated glutin to version 0.10.
 - Added support for is_buffer_supported usage on all buffer types.
 - Various documentation updates and bugfixes.

## Version 0.17.1 (2017-08-27)

 - Changed documentation to docs.rs.
 - Various bugfixes and updates to internal dependencies.

## Version 0.17.0 (2017-07-12)

 - Updated glutin to version 0.9.
 - Redesigned API around EventsLoop to match updated winit design.
 - Added support for vector normalization.
 - Various bugfixes.

## Version 0.16.0 (2017-01-07)

 - Added asynchronous screenshot example.
 - Updated tutorials to compile on Mac OS X.
 - Various tutorial documentation updates.
 - Fix buffer reads which could fail being safe.
 - Various bugfixes.

## Version 0.15.0 (2016-07-03)

 - Updated glutin to version 0.6.1
 - Various internal dependency updates.

## Version 0.14.0 (2016-04-11)

 - Updated glutin to version 0.5.
 - Various bugfixes.

## Version 0.13.5 (2016-02-04)

 - Fixed integer textures using a forbidden filtering by default.
 - Better error report in case of a uniform block layout mismatch.

## Version 0.13.4 (2016-01-21)

 - Added support for shader subroutines.
 - Added various functions to `Context` to retrieve information (like the content of `GL_VENDOR` or `GL_RENDERER` for example).
 - Added additional dimensions getters to the various texture types.

## Version 0.13.3 (2016-01-08)

 - Added constructors to `SimpleFrameBuffer` for depth-only, depth-stencil-only, stencil-only and depth & stencil framebuffers.

## Version 0.13.2 (2015-12-21)

 - Fixed compilation on ARM platform.
 - Added a hack to make screenshots and video taken with the FRAPS software work.

## Version 0.13.1 (2015-12-15)

 - `raw_read_to_pixel_buffer` now accepts all pixel types.

## Version 0.13.0 (2015-12-12)

 - Removed native support for cgmath and nalgebra in avoid to avoid dependency hell.
 - Reworked the layer-related, level-related functions and `into_image` from texture types where they are not relevant.
 - The `Backend::get_proc_address` function now takes a `*const std::os::raw::c_void` instead of a `*const ()`.

## Version 0.12.4 (2015-12-12)

 - Added small hack to make glium work with WebGL.
 - Fixed blend color state not being always updated.

## Version 0.12.3 (2015-12-04)

 - Added `raw_clear_buffer` to `TextureAnyImage`, which allows you to clear the content of any texture.
 - Added `TextureAny::from_id` to manipulate an unowned texture.

## Version 0.12.2 (2015-11-25)

 - Fixed a bug when matching frag output locations with framebuffer attachments.

## Version 0.12.1 (2015-11-25)

 - Added `MultiOutputFrameBuffer::with_stencil`, `with_depth_stencil` and `with_depth_and_stencil`.
 - Implemented the `ToColorAttachment` trait for unsigned and integral textures.
 - Implemented the `ToXAttachment` traits for the `XAttachment` types.
 - It is now possible to use another format than `(u8, u8, u8, u8)` with `raw_read` on a texture.

## Version 0.12.0 (2015-11-21)

 - Removed the "image" feature and native support for the `image`. You now have to perform conversions yourself.
 - The `Backend::get_proc_address` function now takes a `*const ()` instead of a `*const libc::c_void`.
 - Creating a `MultiOutputFrameBuffer` now requires passing an iterator instead of a `Vec`.
 - When checking a uniform block's layout, `[f32; 4]` now matches both `vec4` and `float[4]` instead of just `vec4` before. Same for all other arrays between 1 and 4 elements.
 - `UniformBlock` is now implemented for `[T; N]` where N is between 5 and 32, plus 64, 128, 256, 512, 1024 and 2048.
 - When checking a uniform block's layout, arrays (sized and unsized) now match structs with a single member that is an array.

## Version 0.11.1 (2015-11-10)

 - Fix broken compilation after libc 0.2.

## Version 0.11.0 (2015-11-04)

 - Updated the versions of glutin, image and cgmath.
 - Improved performance of stencil-related state changes.
 - Fixed `glBufferData` being called to invalidate a buffer created with `glBufferStorage`.
 - Changed use of `Range` in buffer slice to `RangeArgument`.

## Version 0.10.0 (2015-10-14)

 - Update glutin to 0.4, cgmath to 0.3 and nalgebra to 0.3.
 - Add the possibility to set the behavior of the debug callback in `Context::new`.
 - Add `build_glium_debug` and `build_glium_unchecked_debug` to set the behavior of the debug callback at initialization.
 - Rename `ClockWise` to `Clockwise`.
 - The `program!` macro now returns a `ProgramChooserCreationError` and no longer panics if no version is found.
 - Remove the `Texture` trait. All of its methods are already implemented on all texture types already.
 - Rename `UniformBlock::binding` to `id` and add `initial_binding`.

## Version 0.9.3 (2015-10-13)

 - Fixed an invalid enum error during initialization.

## Version 0.9.2 (2015-10-11)

 - Add `is_color_renderable` to the color image formats.
 - Fixed glium erroneously using a SSBO's and a uniform buffer's binding point instead of index.
 - Fixed `glInvalidateBuffer(Sub)Data` being called for persistent-mapped buffers.

## Version 0.9.1 (2015-09-30)

 - Add `ComputeShader::execute_indirect`.
 - Add support for 64bits integer uniforms.
 - `copy_to` now takes a `Into<BufferSlice>` instead of a `BufferSlice`.
 - Glium no longer calls `glBlendColor` if the blending algorithm doesn't use it.
 - Deprecated `DrawParameters::validate`.

## Version 0.9.0 (2015-09-15)

 - The blending, depth and stencil functions now use dedicated structs named `Blend`, `Depth` and `Stencil`.
 - `SimpleFrameBuffer` and `MultiOutputFrameBuffer`'s constructors now return a `Result`.
 - Creating a renderbuffer now returns a `Result`.
 - Add support for empty framebuffers with the `EmptyFrameBuffer` type.
 - Fix an OpenGL error when mapping a buffer with `map_read`.
 - Glium no longer panics in case of OpenGL error. It prints a message with backtrace on stdout instead.
 - Removed the `DrawParametersBuilder` struct.
 - Add `clear_color_srgb` and derivates. The `clear` method has an additional `color_srgb: bool` parameter.
 - Added some state changes when swapping buffers to adapt to the FRAPS software.
 - Removed `StencilTexture3d` are they are not supported by OpenGL.

## Version 0.8.7 (2015-08-27)

 - Fix a panic when creating a stencil renderbuffer or texture.
 - Add support for `f64` uniforms.

## Version 0.8.6 (2015-08-26)

 - Renamed `BufferView` to `Buffer`. `BufferView` still exists for backward compatibility.
 - Add a `copy_to` method to buffers.
 - Add support for the `GL_OES_element_index_uint` extension.
 - Fixed OpenGL ES 3.2 not working.
 - Add support for `IntVec{2|3|4}` and `UnsignedIntVec{2|3|4}` uniform types.
 - Add a work-around for Radeon drivers crashing with 32+ texture units.
 - Add support for `boolean` uniforms.
 - Add support for `BoolVec{2|3|4}` uniform types.

## Version 0.8.5 (2015-08-12)

 - Added support for cubemaps and cubemap arrays. Not all operations are available yet.
 - `DrawParameters` no longer implements `Copy`.
 - Add support for OpenGL ES 3.2 for geometry shaders, tessellation shaders, robustness, debug output, buffer textures, stencil textures, base vertex.
 - Add support for `GL_OES_geometry_shader`, `GL_OES_tessellation_shader` `GL_OES_draw_elements_base_vertex`, `GL_OES_draw_elements_base_vertex`.
 - Add support for specifying the primitive bounding box as an optimization hint to the backend.
 - Added various `is_texture_*_supported` functions in the `texture` module to check whether a texture type is supported.

## Version 0.8.4 (2015-08-10)

 - Added `#[inline]` attributes on many functions.
 - Added `slice_custom` and `slice_custom_mut` to buffers, which allow you to get slices over anything inside the buffer.
 - Added `Query::to_buffer_u32` to write the result of a query to a buffer.
 - Added `is_supported` functions for all the texture and renderbuffer formats.
 - Added missing `I16I16I16I16` and `U16U16U16` float formats. Renamed `U3U32U`to `U3U3U2`.
 - Various fixes and improvements when determining whether a format is supported.
 - Changed the `framebuffer::To*Attachment` traits with a lifetime parameter, and to take by value instead of by reference.
 - The `framebuffer::To*Attachment` traits are now implemented on `TextureLayerMipmap` structs.
 - Fixed `.layer()` function returning `None` when it shouldn't.
 - GLSL ES 3.2 is now recognized.

## Version 0.8.3 (2015-08-04)

 - Textures are now inside submodules (for example `Texture2d` is in `texture::texture2d::Texture2d`) and reexported from `texture`.
 - Added `Context::flush()` and `Context::finish()`. Deprecated `Context::synchronize`.
 - Removed `Sized` constraint for `Surface` that was preventing one from using `&Surface` or `&mut Surface`.
 - Added `TextureAnyMipmap::raw_upload_from_pixel_buffer`.
 - Moved the `pixel_buffer` module to `texture::pixel_buffer` (the old module still exists for backward compatibility).

## Version 0.8.2 (2015-07-29)

 - Added a `buffer_texture` module in `texture`.
 - Added `DrawParameters::depth_clamp` and `DrawError::DepthClampNotSupported`.

## Version 0.8.1 (2015-07-27)

 - Added `DrawParameters::ProvokingVertex` and `DrawError::ProvokingVertexNotSupported`.
 - Added `DrawError::RasterizerDiscardNotSupported`.
 - `DrawParametersBuilder` is now deprecated.
 - `ProgramCreationInput` now has a `outputs_srgb` member.
 - Fixed a bug with offsets of members in arrays.
 - Added support for the `GL_EXT_geometry_shader` OpenGL ES extension.

## Version 0.8.0 (2015-07-19)

 - Removed all Cargo features related to OpenGL compatibility.
 - Replaced all `new_if_supported` or `empty_if_supported` functions with `new` or `empty` and a proper error type.
 - All `new` and `empty` constructors now return an error if the operation is not supported.
 - `VertexBuffer` and `IndexBuffer` constructors now take a `&[T]`.
 - Updated glutin to version 0.3.
 - `BufferView`'s constructors now take a `BufferMode`.
 - Replaced `read` and `read_if_supported` with `read` that returns a `ReadError`.
 - Added `buffer::is_buffer_read_supported`.
 - Removed the deprecated `new_empty` function from textures.
 - Reworked `TextureCreationError` and removed `TextureMaybeSupportedCreationError`.
 - `new` and `dynamic` are now less specialized, which should fix some performance bottlenecks.
 - All buffer constructors now come in four variants: `new`, `dynamic`, `persistent`, `immutable`.
 - Add support for all missing vertex attributes.
 - Fixed stencil operations sometimes not working.

## Version 0.7.1 (2015-07-14)

 - Glium now automatically calls `glDraw*BaseVertex` if it is supported.
 - Added the `CapabilitiesSource` trait implemented automatically on all types that implement `Facade`.
 - Added `is_supported` to `IndexType`, `Index`, `PrimitiveType`, `Attribute` and `AttributeType`.
 - Added `ComputeShader::is_supported`, `program::is_geometry_shader_supported` and `program::is_tessellation_shader_supported`.
 - The `implement_buffer_content!` and `implement_block_layout!` macros can now take a struct with a lifetime parameter by passing `Foo<'a>` instead of `Foo`.

## Version 0.7.0 (2015-07-08)

 - Buffers now contain a single, possibly unsized, element. `BufferView<T>` is now `BufferView<[T]>`.
 - Creating an empty `BufferView<[T]>` now requires calling `empty_array` instead of `empty`.
 - `BufferView::write` now takes a `&T` instead of a `P where P: AsRef<[T]>`. This means that you can no longer pass `Vec`s by value for example.
 - Removed the mock methods from `UniformBuffer` as they are available through the `Deref` to `BufferView`.
 - The `UniformBlock` trait is now implemented on `DrawCommandNoIndices` and `DrawCommandIndices`.
 - Added the `implement_buffer_content!` macro to put on unsized struct so that you can put them inside buffers.
 - Added a `uniforms::LayoutMismatchError` as an additional field to `DrawError::UniformBlockLayoutMismatch`.
 - Changed `UniformBlock::matches` to return a `Result<(), UniformBlockLayoutMismatch>`.
 - Removed the `matches` function from the `AsUniformValue` trait.
 - Removed `TextureSurface` in favor of `SimpleFrameBuffer`.

## Version 0.6.7 (2015-07-06)

 - Added `DrawCommandsIndicesBuffer` for multidraw elements indirect.
 - Added `ResidentTexture`, `TextureHandle` and `Texture::resident_if_supported()` for bindless textures.
 - Buffers no longer require their content to be `Send` or `'static` (except for types with the `Any` suffix).
 - Renamed `texture::TextureType` to `Dimensions`.
 - Added `Context::get_opengl_version()` and deprecated `get_version()`.

## Version 0.6.6 (2015-07-03)

 - Buffers with persistent mapping now synchronize individual segments of the buffer instead of the buffer as a whole.
 - Dynamic sized arrays are now properly handled by the introspection system.
 - Added support for matrix shader attributes.
 - Fixed the value of `depth_write` being ignored if `depth_test` was `Overwrite`.

## Version 0.6.5 (2015-06-29)

 - Renamed and changed `UniformBlockMember` to `BlockLayout`.
 - Added `map_read()` and `map_write()` to buffers.
 - Various bugfixes related to queries.
 - Added a `implement_uniform_block!` macro for uniform buffers and SSBOs similar to `implement_vertex!`.
 - Added `UniformBuffer::empty` and `UniformBuffer::empty_if_supported`.
 - Fixed sRGB being disabled when clearing before drawing for the first time.

## Version 0.6.4

 - Fixed an OpenGL error if GL_ARB_robustness was present with OpenGL < 3.0.
 - Now using the `GL_APPLE_SYNC` extension if OpenGL ES 3.0 is not available.
 - Now using the `GL_EXT_buffer_storage` extension on OpenGL ES if it is available.
 - Slightly improved performances when using a dynamic buffer.
 - Added `GL_PROGRAM_POINT_SIZE` support to `Program`.

## Version 0.6.3

 - Fixed the OpenGL compatibility check for SSBO reflection.
 - Fixed a potential OpenGL error if uniform buffer objects are not supported.
 - Fixed a potential OpenGL error if transform feedback buffers are not supported.

## Version 0.6.2

 - Added `is_context_lost`, `is_robust` and `is_context_loss_possible` methods to `Context` (callable through the `Display` as well).
 - Fixed using a framebuffer with a depth or stencil attachment causing the wrong texture to be displayed.

## Version 0.6.1

 - Fixed mipmaps generation with sRGB textures.

## Version 0.6.0

 - `Frame::finish` now returns a `Result`. `Frame`'s destructor will panic if `finish` has not been called.
 - `with_compressed_data` and `with_compressed_data_if_supported` now have an additional parameter for mipmaps.
 - All the texture constructors that used to take a boolean as parameter for mipmaps now takes an enum.
 - `empty_with_format` and `empty_with_format_if_supported` are now allowed for compressed textures.
 - `write`, `write_compressed_data`, `write_compressed_data_if_supported` and `read_compressed_data` are now available for mipmap objects.
 - Removed the `is_closed()` function.
 - Removed the deprecated `render_buffer` module.
 - The `Backend::swap_buffers` function must now return a `Result`.

## Version 0.5.6

 - The panic at initialization in case of OpenGL error has been replaced by a warning.

## Version 0.5.5

 - Added `with_compressed_data`, `read_compressed_data` and `write_compressed_data` to compressed texture types.
 - Fixed a panic at initialization on OS/X.

## Version 0.5.4

 - Added the `LineLoop` primitive type.
 - Added the `smooth` draw parameters.
 - Fixed a potential `GL_INVALID_ENUM` error generated at initialization.
 - Glium will now panic if an OpenGL error is triggered during initialization.
 - Fixed gamma correction with OpenGL ES.
 - Fixed `get_internal_format_if_supported()` sometimes panicking.
 - Fixed a panic with a tessellation evaluation shader that outputs quads.

## Version 0.5.3

 - Added support for compute shaders with the `program::ComputeShader` struct.
 - Can now bind a uniform buffer as a shader storage buffer.
 - Added `Program::get_shader_storage_blocks()` to obtain the list of shader storage blocks.
 - The `Attribute` trait is now implemented on types from cgmath.
 - Now caching the actual format of a texture in case it is retrieved multiple times.

## Version 0.5.2

 - Added `vertex::TransformFeedbackSession` and `transform_feedback` to the draw parameters.
 - Added `RenderBufferAny`. Render buffers now deref to `RenderBufferAny`.
 - Deprecated `.into_vertex_buffer_any()` in favor of `.into()`.
 - Added `.get_total_bits()` to `texture::InternalFormat`.
 - Fixed values in uniform blocks being required to implement the `Default` trait.
 - Fixed buffer sometimes not always correctly unbound.

## Version 0.5.1 (2015-05-30)

 - Added `index::DrawCommandsNoIndicesBuffer` for multidraw indirect drawing.
 - Fixed a potential panic when using a buffer.

## Version 0.5.0 (2015-05-27)

 - `IndexBuffer` now takes the type of indices as template parameter.
 - `IndexBuffer` now derefs to `BufferView`. Allows modifying the index buffer after it has been created.
 - Added `IndexBufferAny`.
 - Removed the `ToIndicesSource` and `IntoIndexBuffer` traits. The former is replaced with `Into<IndicesSource>`.
 - `PixelBuffer` now takes the type of pixels as template parameter.
 - Renamed `PixelBuffer::read` to `PixelBuffer::read_as_texture_2d`.
 - `PixelBuffer` now derefs to `BufferView`.
 - Added `BufferView::read_as_texture_1d` and `BufferView::read_as_texture_1d_if_supported`.
 - Reworked `TextureDataSink` traits to take a precise format.
 - Fixed a panic when destroying a buffer with persistent mapping.
 - Removed deprecated function `VertexBuffer::new_dynamic`.
 - It is now safe to call `mem::forget` on a `Mapping` object.

## Version 0.4.2 (2015-05-25)

 - Removed the `buffer::Builder` type as it was proven unsound.
 - Renamed `SubBuffer` to `BufferView`.
 - `Display`/`GlutinFacade` now derefs to `Context`.
 - Mapping a buffer now simply calls `glMapBuffer` again, instead of writing to a temporary buffer.
 - `glutin` is now an optional dependency (enabled by default).
 - Creating an index buffer now correctly uses a `GL_ELEMENT_ARRAY_BUFFER`.

## Version 0.4.1 (2015-05-22)

 - Added a `buffer::Builder` type to build multiple sub-buffers within the same buffer.
 - Added an `invalidate` method to buffers.
 - Updated glutin to 0.1.6.

## Version 0.4.0 (2015-05-17)

 - Removed support for using indices in RAM.
 - `IndexBuffer`, `VertexBuffer` and `UniformBuffer` now deref to `SubBuffer`/`SubBufferAny`.
 - Added a `get_internal_format_if_supported` method to textures.
 - Replaced the `IntoProgramCreationInput` trait with `Into<ProgramCreationInput>`.
 - The `VertexFormat` type is now a `Cow<'static, []>` instead of a `Vec`.
 - Updated cgmath dependency to version 0.2.

## Version 0.3.7 (2015-05-12)

 - Added `AnySamplesPassedQuery`, `PrimitivesGeneratedQuery` and `TransformFeedbackPrimitivesWrittenQuery`, and corresponding fields in `DrawParameters`.
 - The `samples_passed_query` draw parameter can now also take a `AnySamplesPassedQuery`.
 - Added a `condition` parameter to the draw parameters, allowing you to use conditional rendering.
 - Textures now have a `sampled()` method to make it easier to create a `Sampler`.
 - Added a `color_mask` member to the draw parameters.
 - Added `per_instance` and `per_instance_if_supported` to `VertexBufferAny`.

## Version 0.3.6 (2015-05-09)

 - Added `SamplesPassedQuery` and `TimeElapsedQuery` types. They can be passed to the draw parameters.
 - Buffers are no longer mapped with `GL_MAP_COHERENT_BIT`. Flushing is handled by glium.
 - Changed `Surface::clear` to take an additional optional `Rect` parameter that specifies the rectangle to clear.
 - Fixed the `program!` macro not usable with version numbers >= 256.
 - Added support for `GL_OES_depth_texture` and `GL_OES_packed_depth_stencil`.
 - Moved the content of the `render_buffer` module to `framebuffer`. `render_buffer` still exists but is deprecated.

## Version 0.3.5 (2015-05-02)

 - Glium now reexports glutin. You can access glutin with `glium::glutin`.
 - Fixed trying to retrieve uniform blocks on OpenGL ES 2.

## Version 0.3.4 (2015-04-28)

 - Added caching some uniform values in the `Program` struct to avoid calls to `glUniform`.
 - `VertexBuffer::dynamic` and `write` can now take a  `&[T]` as well as a `Vec<T>`.
 - `assert_no_error` now takes an optional user-defined string.
 - Fixed triggering an OpenGL error on initialization with non-compatibility contexts.
 - Added a better texture units assignment system when drawing, in order to avoid some redundant state changes.

## Version 0.3.3 (2015-04-24)

 - Removed the `gl_persistent_mapping` feature.
 - Creating a dynamic buffer now creates a persistent mapped buffer automatically if supported.
 - Creating a non-dynamic buffer now creates an immutable buffer. Modifying such a buffer is still possible but very expensive.
 - Deprecated the `new_dynamic` function in favor of `dynamic`.
 - Fixed zero-sized textures triggering OpenGL errors.
 - Added the possibility to add a debug string in the OpenGL commands queue.

## Version 0.3.2 (2015-04-23)

 - Added `vertex::EmptyVertexAttributes` and `vertex::EmptyInstanceAttributes` markers in order to draw without a vertex source.
 - Added more formats in `CompressedFormats` and `CompressedSrgbFormats`.
 - Fixed the `Mapping` objects not implementing `Sync`.
 - Fixed rendering to an sRGB texture not possible.
 - `GliumCreationError` now has a template parameter indicating the backend creation error type.
 - The `DisplayBuild` trait has a new associated type indicating the error type.
 - Fixed the debug output callback not being disabled when compiling with --release.

## Version 0.3.1 (2015-04-18)

 - Fixed lifetime-related issues when using the `uniform!` macro.

## Version 0.3.0 (2015-04-16)

 - Changed the `slice()` functions to take a `Range` instead of two parameters.
 - Added `SrgbTexture` and `CompressedSrgbTexture` types.
 - Added `SrgbFormat` and `CompressedSrgbFormat` enums.
 - `GL_FRAMEBUFFER_SRGB` is now enabled by default.

## Version 0.2.3 (2015-04-14)

 - Added a `program!` macro which builds a program. Glium chooses the right shaders depending on what the backend supports.
 - Added a `VertexBuffer::empty` method that creates a vertex buffer with uninitialized data.
 - Fixed drawing with offsets in vertex buffers different than 0 not permitted.
 - Changed transform feedback reflection API to be compatible with what OpenGL 4.4 or ARB_enhanced_layouts allow.
 - `VertexBuffer::new` can now take a slice as parameter.
 - Revert the fix for sRGB. `GL_FRAMEBUFFER_SRGB` is no longer enabled.

## Version 0.2.2 (2015-04-10)

 - Added support for backends that don't have vertex array objects (like OpenGL ES 2/WebGL).
 - Glium now has basic support for OpenGL ES 2/WebGL.
 - Added stencil operations in `DrawParameters`.
 - Fixed `GL_FRAMEBUFFER_SRGB` not being enabled, leading to different brightness depending on the target.
 - Fixed trailing commas not working in `implement_vertex!` and `uniform!`.

## Version 0.2.1 (2015-04-03)

 - Creating a texture with a specific format now properly checks for available extensions.

## Version 0.2.0 (2015-03-30)

 - Removed `PerInstanceAttributesBuffer` and `PerInstanceAttributesBufferAny`.
 - Added `per_instance` and `per_instance_if_supported` to `VertexBuffer`.
 - Removed the deprecated `index_buffer` module. Use the `index` module instead.
 - Fixed viewport dimensions on retina screens.
 - The `Backend` trait is now marked unsafe.

## Version 0.1.3 (2015-03-24) & 0.1.4 (2015-03-26)

 - Added the `Backend` and `Facade` traits. `Display` implements the `Facade` trait.
 - Changed all buffer/texture/etc. creation functions to take any type that implements `Facade` instead of a `Display`.
 - Added `GlutinWindowBackend` and `GlutinHeadlessBackend` that implement the `Backend` trait.
 - Changed the private `Context` struct to be public. This allows users to implement the `Facade` trait themselves.
 - Added an associated type to the `DisplayBuild` trait.
 - Fixed scissor boxing not being disabled before a blit.

## Version 0.1.2 (2015-03-20)

 - Fixed a memory leak with vertex array objects.
 - Fixed an issue where you couldn't reuse the same uniform values created with `uniform!` multiple times.

## Version 0.1.1 (2015-03-13)

 - Added `ToColorAttachment` trait implementation for `Texture2dMultisample`.
 - Added `Texture2dMultisample::as_surface` method.
 - Updated the crate for the new I/O.
 - Changed `VertexFormat` to take a `Cow<'static, str>` instead of a `String`.
 - Fixed a stack overflow in release mode.
 - Removed the `fence` argument from vertices, indices and uniforms sources. Fences are now directly gathered from buffers when drawing.
