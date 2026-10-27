# SafeGL

[![Build Status](https://github.com/viniciusmorgado/safegl/actions/workflows/ci.yml/badge.svg)](https://github.com/viniciusmorgado/safegl/actions/workflows/ci.yml)
[![crates.io page](https://img.shields.io/crates/v/safegl.svg)](https://crates.io/crates/safegl)
[![Dependency status](https://deps.rs/repo/github/viniciusmorgado/safegl/status.svg)](https://deps.rs/repo/github/viniciusmorgado/safegl)

Safe OpenGL bindings for Rust.

SafeGL is an intermediate layer between OpenGL and your application. You still need to manage the
graphics pipeline yourself, but you don't have to deal with OpenGL's old, error-prone API or with
manual memory management.

This project is a fork of [glium](https://github.com/glium/glium), maintained by
[Oxidian Studios](https://github.com/oxidianindustries), a video game company, with the intent of using it to develop
graphical tools for internal use. We are backporting a large number of [PRs](https://github.com/glium/glium/pulls)
and [bug fixes](https://github.com/glium/glium/issues) that have been left unattended in the original GitHub repository,
including a few PRs that we submitted ourselves.

One of the main reasons why this fork exists is that we don't believe that bugs, PRs, and other project issues are being
addressed quickly enough by the original glium community. Even though the project is not officially discontinued,
maintenance by the surrounding community has slowed down over time.

If you want to understand how we ended up here, check the links below:
- [Glium post-mortem](https://users.rust-lang.org/t/glium-post-mortem/7063)
- [Reword "Note to current and future Glium users"? (glium/glium#1912)](https://github.com/glium/glium/issues/1912)
- [Original glium repo](https://github.com/glium/glium)

## Where SafeGL fits in the Rust graphics ecosystem

Rust crates for the open graphics APIs come in two layers: unsafe bindings that mirror the API,
and safe layers built on top of them. Renderers and engines are not listed here.

| Layer            | Vulkan                                        | OpenGL                                  |
|------------------|-----------------------------------------------|-----------------------------------------|
| Safe layer       | [vulkano](https://github.com/vulkano-rs/vulkano) | **SafeGL**                           |
| Unsafe bindings  | [ash](https://github.com/ash-rs/ash)          | [glow](https://github.com/grovesNL/glow) |

SafeGL fills the OpenGL slot that vulkano fills for Vulkan, using glow as its binding layer the
way vulkano uses ash.

### Discontinued

| Crate                                   | Layer           | Last release      |
|-----------------------------------------|-----------------|-------------------|
| [glium](https://github.com/glium/glium) | Safe layer      | 0.36.0 (Oct 2024) |
| [gl](https://github.com/brendanzab/gl-rs) | Unsafe bindings | 0.14.0 (Oct 2019) |

## Objectives

- **Be safe to use.** Misuse of the OpenGL API that could cause crashes or undefined behavior is
  prevented by the type system or validated before reaching the driver.
- **Enforce good practices** such as RAII and stateless function calls.
- **Support modern OpenGL only**: desktop OpenGL 4.0 to 4.6 in the core profile, OpenGL ES 3.0
  to 3.2 and WebGL 2, each version a level selected with a Cargo feature, instead of the whole of
  OpenGL.
- **Produce optimized OpenGL function calls**, and make modern OpenGL techniques easy to use.

SafeGL validates how you use OpenGL, but it cannot detect or work around bugs in the drivers
themselves.

## OpenGL levels

Glium, SafeGL's predecessor, tries to expose the whole of OpenGL across a wide range of versions.
SafeGL supports three chains of modern versions, and each version is a level, selected with a
Cargo feature:

| Chain                        | Levels                                                | Typical targets                                                                                    |
|------------------------------|-------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| Desktop OpenGL, core profile | `gl40` (**default**), `gl41`, `gl42`, `gl43`, `gl44`, `gl45`, `gl46` | Windows and Linux with NVIDIA, AMD, Intel or Mesa drivers (`gl46`); macOS (`gl41` at most); Asahi Linux on Apple M1/M2 (`gl46`) |
| OpenGL ES                    | `gles30`, `gles31`, `gles32`                          | Embedded Linux, such as the Raspberry Pi 4/5 (`gles31`), and other OpenGL ES devices               |
| WebGL                        | `webgl`                                               | Browsers, through `wasm32-unknown-unknown` (see [examples/web](examples/web))                     |

A level is a ceiling: SafeGL compiles the API of its version and of the versions below it in its
chain, and the API of the versions above it doesn't compile. The API that only desktop OpenGL has
needs a desktop level, and the API that only OpenGL ES has needs an OpenGL ES level. No level
implies another:

- Select at most one desktop level and one OpenGL ES level. Both together compile the API of both,
  for an application that runs on either.
- Without any level, the level is `gl40`.
- In the browser, select `webgl` alone: it is the OpenGL ES 3.0 API.

Two levels of the same chain, a desktop or OpenGL ES level for `wasm32-unknown-unknown`, or no
`webgl` level there are compile errors that name the levels.

```toml
[dependencies]
# OpenGL 4.0
safegl = "1"
# OpenGL 4.6
safegl = { version = "1", features = ["gl46"] }
# OpenGL 4.6, or OpenGL ES 3.2 where desktop OpenGL isn't available
safegl = { version = "1", features = ["gl46", "gles32"] }
# OpenGL ES 3.0
safegl = { version = "1", features = ["gles30"] }
# WebGL 2, drawing through `safegl::backend::web::WebDisplay`
safegl = { version = "1", default-features = false, features = ["webgl"] }
```

Whatever the level, SafeGL runs on desktop OpenGL 4.0 and above, in the core profile only, and on
OpenGL ES 3.0 and above. It refuses other contexts with `IncompatibleOpenGl`. `SimpleWindowBuilder` requests the version of the selected desktop level in
the core profile, then the version of the selected OpenGL ES level, and
`backend::glutin::level_context_apis()` gives them to code that creates its own contexts. glutin
can't request a forward-compatible context, so SafeGL behaves as one on desktop OpenGL: lines
wider than 1 pixel return `DrawError::LineWidthNotSupported`.

The default `backtrace` feature prints a backtrace along with the OpenGL debug messages that
SafeGL reports as errors; leave it out to avoid compiling the `backtrace` crate.

### What each level compiles

Everything that isn't listed here compiles at every level: buffers, textures and framebuffers of
the OpenGL ES 3.0 set, vertex and index buffers, programs with vertex and fragment shaders,
transform feedback, uniform blocks, instancing, samplers, sync fences, `AnySamplesPassedQuery`
and `TransformFeedbackPrimitivesWrittenQuery`, among others. The table gives the lowest level of
each chain that compiles a functionality; — means that the chain doesn't have it.

| Functionality                                    | Desktop | OpenGL ES | WebGL | Gated items                                                                                                                                  |
|--------------------------------------------------|---------|-----------|-------|----------------------------------------------------------------------------------------------------------------------------------------------|
| Geometry and tessellation shaders                | `gl40`  | `gles32`  | —     | geometry and tessellation sources of `ProgramCreationInput::SourceCode`, `Program::from_source` and `program!`; `PrimitiveType::{LinesListAdjacency, LineStripAdjacency, TrianglesListAdjacency, TriangleStripAdjacency, Patches}`; their `DrawError`s |
| Per-output blending and color masks              | `gl40`  | `gles32`  | —     | `DrawParameters::outputs`, `OutputState`, `DrawError::{PerOutputStateNotSupported, OutputStateIndexOutOfBounds}`                            |
| Sample shading                                   | `gl40`  | `gles32`  | —     | `MultisampleState::sample_shading`, `DrawError::SampleShadingNotSupported`                                                                   |
| Primitives generated queries                     | `gl40`  | `gles32`  | —     | `PrimitivesGeneratedQuery`, `DrawParameters::primitives_generated_queries`                                                                   |
| Buffer textures                                  | `gl40`  | `gles32`  | —     | `texture::buffer_texture`, `UniformValue::BufferTexture`                                                                                     |
| Cube map array and 2D multisample array textures | `gl40`  | `gles32`  | —     | `texture::{CubemapArray, Texture2dMultisampleArray, ...}`, `is_cubemap_arrays_supported`, `is_texture_2d_multisample_array_supported`        |
| Border colors                                    | `gl40`  | `gles32`  | —     | `SamplerWrapFunction::BorderClamp`, `Sampler::border_color`, `uniforms::BorderColor`                                                         |
| 2D multisample textures and sample masks         | `gl40`  | `gles31`  | —     | `texture::{Texture2dMultisample, ...}`, `is_texture_2d_multisample_supported`, `MultisampleState::sample_mask`, `DrawError::SampleMaskNotSupported` |
| Subroutines                                      | `gl40`  | —         | —     | `UniformValue::Subroutine`, `Program::get_subroutine_uniforms`, `program::{is_subroutine_supported, SubroutineData, SubroutineUniform}`      |
| Double-precision and 64-bit integer uniforms     | `gl40`  | —         | —     | `UniformValue::Double*`, `Int64*` and `UnsignedInt64*`, `RawUniformValue::Double*` and `*Int64*`, their `AsUniformValue` and block impls     |
| 1D textures                                      | `gl40`  | —         | —     | `texture::{Texture1d, Texture1dArray, ...}`, `is_texture_1d_supported`, `is_texture_1d_array_supported`                                     |
| Rasterization state of desktop OpenGL            | `gl40`  | —         | —     | `DrawParameters::{polygon_mode, point_size, clip_planes_bitmask, smooth, provoking_vertex}`, `MultisampleState::{enabled, alpha_to_one}`, `Depth::clamp`, `PolygonOffset::{point, line}`, `PolygonMode`, `Smooth`, `ProvokingVertex`, `DepthClamp`, their `DrawError`s |
| Samples passed and time elapsed queries, conditional rendering, GPU timestamps | `gl40` | — | — | `SamplesPassedQuery`, `TimeElapsedQuery`, `DrawParameters::{time_elapsed_query, condition}`, `ConditionalRendering`, `DrawError::ConditionalRenderingNotSupported`, `Context::gpu_timestamp` |
| 64-bit integer and packed float vertex attributes | `gl40` | —        | —     | `AttributeType::{I64*, U64*, F64*, F10F11F11UnsignedIntReversed}`                                                                            |
| Bindless textures (`GL_ARB_bindless_texture`)    | `gl40`  | —         | —     | `texture::bindless`, `texture::{ResidentTexture, TextureHandle, BindlessTexturesNotSupportedError}`, `resident()` on textures                 |
| Chosen primitive restart index, BGRA attributes, ranges drawn with one call, default tessellation levels | `gl40` | — | — | `PrimitiveRestart::Index`, `DrawError::PrimitiveRestartIndexNotSupported`, `vertex::Bgra8`, `AttributeType::*Bgra`, `index::NoIndicesRanges`, `IndexBuffer::{ranges, ranges_with_base_vertices}`, `IndicesSource::{NoIndicesRanges, IndexBufferRanges}`, `DrawError::{DrawRangesNotSupported, DrawRangeOutOfBounds, InstancedDrawRangesNotSupported}`, `DrawParameters::patch_default_levels`, `PatchDefaultLevels` |
| Draws of captured vertices and vertex streams    | `gl40`  | —         | —     | `CapturedVertices::{indices, stream_indices}`, `IndicesSource::TransformFeedback`, `PrimitivesGeneratedQuery::with_stream`, `TransformFeedbackPrimitivesWrittenQuery::with_stream`, `DrawError::CapturedVerticesDrawNotSupported` |
| Double-precision vertex attributes               | `gl41`  | —         | —     | the `Attribute` impls of `f64`, its arrays and its tuples                                                                                   |
| Program binaries                                 | `gl41`  | `gles30`  | —     | `Program::get_binary`, `ComputeShader::{get_binary, from_binary}`, `ProgramCreationInput::Binary`, `program::{Binary, GetBinaryError, is_binary_supported}` |
| Shader precision formats                         | `gl41`  | `gles30`  | `webgl` | `Context::shader_precision_format`, `program::{PrecisionQualifier, ShaderPrecision}`                                                          |
| Base instances                                   | `gl42`  | —         | —     | `DrawParameters::base_instance`, `DrawError::BaseInstanceNotSupported`                                                                     |
| Indirect draws of one command                    | `gl40`  | `gles31`  | —     | `index::{DrawCommandsNoIndicesBuffer, DrawCommandsIndicesBuffer, DrawCommandNoIndices, DrawCommandIndices, DrawCommandLayout}`, `command()` and `command_with_index_buffer()` on them, `implement_draw_command!`, `IndicesSource::Multidraw*`, `DrawError::{IndirectDrawsNotSupported, MultidrawNotSupported, IndirectDrawWithTransformFeedback}` |
| Image load/store and atomic counters             | `gl42`  | `gles31`  | —     | `uniforms::{ImageUnit, ImageUnitBehavior, ImageUnitAccess, ImageUnitFormat, ImageUnitError}`, `image_unit()` on textures, `UniformValue::Image`, `BufferType::AtomicCounterBuffer`, `Program::get_atomic_counters` |
| Compute shaders, shader storage and empty framebuffers | `gl43` | `gles31` | — | `program::{ComputeShader, ComputeCommand}`, `BufferType::{ShaderStorageBuffer, DispatchIndirectBuffer}`, `Program::get_shader_storage_blocks`, `framebuffer::EmptyFrameBuffer` |
| Copying images                                   | `gl43`  | `gles32`  | —     | `texture::{copy_image, is_copy_image_supported, CopyImageTarget, CopyImageError}`, `copy_to()` on mipmaps                                    |
| Clearing buffers                                 | `gl43`  | —         | —     | `clear()` on buffers and their slices, `buffer::{ClearValue, ClearContent, ClearBufferError}`                                                |
| Multidraw indirect                               | `gl43`  | —         | —     | `with_primitive_type()` and `with_index_buffer()` on the draw command buffers                                                               |
| Framebuffer invalidation                         | `gl43`  | `gles30`  | `webgl` | `invalidate()` and `invalidate_rect()` on framebuffers, `framebuffer::InvalidateAttachments`                                              |
| Debug groups, object labels and debug messages   | `gl43`  | `gles32`  | —     | `Context::{push_debug_group, set_debug_messages_enabled, insert_debug_message, debug_message_log, is_debug_output_supported}`, `debug::{DebugGroup, ObjectLabel, DebugMessage, DebugMessageFilter, DebugError}`, `DebugCallbackBehavior::Log`, the debug fields of `Limits` |
| Stencil textures                                 | `gl44`  | `gles32`  | —     | `texture::{StencilTexture2d, ...}`                                                                                                           |
| Clearing textures and query buffers              | `gl44`  | —         | —     | `texture::{is_clear_texture_supported, ClearTextureError}`, `clear()`, `clear_region()` and `clear_depth_stencil()` on mipmaps, `to_buffer_u32()` and `to_buffer()` on queries, `debug::TimestampQuery::to_buffer`, `draw_parameters::{ToBufferError, QueryBufferMode, QueryBufferValue}`, `BufferType::QueryBuffer` |
| Mirror-clamp wrapping                            | `gl44`  | —         | —     | `SamplerWrapFunction::MirrorClamp`                                                                                                           |
| Texture barrier and clip control                 | `gl45`  | —         | —     | `texture::{texture_barrier, is_texture_barrier_supported, TextureBarrierNotSupportedError}`, `DrawParameters::{clip_control_origin, clip_control_depth}`, `ClipControlOrigin`, `ClipControlDepth`, `DrawError::ClipControlNotSupported` |
| Graphics reset status                            | `gl45`  | `gles32`  | —     | `Context::graphics_reset_status`, `GraphicsResetStatus`                                                                                      |
| Inverted conditional rendering                   | `gl45`  | —         | —     | `ConditionalRendering::inverted`, `DrawError::ConditionalRenderingInvertedNotSupported`                                                      |
| Draws with a number of commands from a buffer    | `gl46`  | —         | —     | `DrawCommandsNoIndicesBuffer::with_draw_count`, `DrawCommandsIndicesBuffer::with_index_buffer_and_draw_count`, `index::DrawCount`, `BufferType::ParameterBuffer`, `DrawError::DrawCountNotSupported` |
| Pipeline statistics and transform feedback overflow queries | `gl46` | — | — | `draw_parameters::{PipelineStatisticsQuery, PipelineStatistic, TransformFeedbackOverflowQuery}`, `DrawParameters::{pipeline_statistics_queries, transform_feedback_overflow_query}`, `ComputeShader::execute_with_statistics`, `ConditionQuery::TransformFeedbackOverflowQuery` |
| Polygon offset clamp and SPIR-V shaders          | `gl46`  | —         | —     | `DrawParameters::polygon_offset_clamp`, `DrawError::PolygonOffsetClampNotSupported`, `ProgramCreationInput::SpirV`, `program::{SpirvProgram, SpirvEntryPoint}` |
| Contexts without error checking                  | `gl46`  | —         | —     | `Capabilities::no_error`, `Context::is_no_error`                                                                                              |
| Primitive bounding boxes                         | —       | `gles32`  | —     | `DrawParameters::primitive_bounding_box`                                                                                                     |

At the levels without geometry and tessellation shaders, their sources have the type
`program::NotInLevel`, which has no values: only `None` compiles. The fields of
`Capabilities::limits` follow the same rule: a limit, such as `Limits::max_viewports` (`gl41`) or
`Limits::compute` (`gl43`, `gles31`), exists at the levels whose versions have it. The documentation of the gated items shows their condition,
where `safegl_gl43` means the `gl43` level or a higher one and `safegl_desktop` any desktop level.

A level doesn't guarantee that the driver of a machine has its version, and some functionalities
are extensions at every level, so the gated APIs keep their runtime checks: for example
`ComputeShader::is_supported` returns `false` and `resident()` returns an error when the driver
lacks them, and 64-bit integer uniforms need `GL_ARB_gpu_shader_int64`. Texture formats, vertex
attribute types and the other values that describe what the driver has are checked at runtime
too: `is_supported` on a format tells whether the context has it. `BufferMode::Persistent` and
`BufferMode::Immutable` work at every level: they use `glBufferStorage` (OpenGL 4.4) when the
driver has it and fall back to `glBufferData` otherwise.

### Platform notes

- **macOS** stops at OpenGL 4.1, and Apple has deprecated OpenGL since macOS 10.14: select `gl41`
  or a lower level. Higher levels compile, but creating their context fails with a message that
  says so.
- **Asahi Linux** on Apple M1/M2 ships a conformant OpenGL 4.6 driver, so the same hardware that is
  limited to `gl41` under macOS can use `gl46` under Linux.
- **Raspberry Pi 4/5** and similar boards expose OpenGL ES 3.1 but only desktop OpenGL 3.1; use
  `gles31`.
- **OpenGL ES** has fewer formats than desktop OpenGL. Data uploaded to a texture must match its
  format, so a texture created from data gets a format chosen from that data. OpenGL ES only
  reads textures back as `(i32, i32, i32, i32)`, `(u32, u32, u32, u32)` or `(f32, f32, f32, f32)`
  for integer and float textures, and as `(u8, u8, u8, u8)` for normalized ones; `read()` and
  `read_as()` convert the other pixel formats on the CPU.
- **OpenGL ES** can't move shader storage blocks and image uniforms to other binding points:
  SafeGL binds them where the shader's `layout(binding = N)` puts them, and two of them with the
  same binding return `DrawError::SharedBindingPoint`. It also converts the colors written to
  sRGB targets unless the driver has `GL_EXT_sRGB_write_control`, whatever `outputs_srgb` is.
- **Virtual machines** (VirtualBox, VMware) usually expose OpenGL 4.1 to 4.3; recent Mesa
  software renderers (llvmpipe) expose OpenGL 4.5 or 4.6.
- **Windows without a vendor driver** (Microsoft's basic display driver, some Remote Desktop
  sessions) only provides OpenGL 1.1 and is not supported by any level.
- **Browsers** provide WebGL 2, which SafeGL treats as OpenGL ES 3.0. WebGL can't map buffers or
  block on fences, so buffer mappings are emulated with copies and `SyncFence::wait` returns after
  `glFinish`. Shaders must start with `#version 300 es` on their very first line.

## Why SafeGL matters for Oxidian

At Oxidian, we want to use SafeGL to build practical internal tools such as asset viewers,
texture inspectors, material previews, and thumbnail generators. These tools address everyday
production needs while giving programmers new to computer graphics manageable projects with
useful results.

SafeGL's safe abstractions let newcomers learn the graphics pipeline by working with geometry,
textures, shaders, and draw calls, while handling much of the resource lifetime and state management
that makes raw OpenGL error-prone. They can develop these foundations without first tackling
Vulkan's explicit GPU memory management or many of the manual resource management pitfalls of
raw OpenGL.

For us, this also creates a practical path for onboarding and developing graphics programmers:
new team members contribute useful tools as they learn, build confidence through hands-on work,
and prepare to take on more demanding rendering tasks. Investing in SafeGL supports both our
production workflow and the growth of our team.

## [SafeGL tutorials](book/README.md)

Learn to draw with SafeGL, from your first window and triangle to textures,
3D models, cameras and lighting.

## Examples

The examples in [examples/standalone](examples/standalone) each show one feature, from the
tutorial's triangle to shadow mapping, deferred shading and order-independent transparency. Run one
by its name:

```sh
cargo run --example triangle
cargo run --release --example weighted_oit
```

An example that needs a level above `gl40` says which one when it runs at a lower level; select
it with `--features`:

```sh
cargo run --example gpgpu --features gl43
```

## Running the tests

The tests open hidden windows, so they need a display server and an OpenGL driver:

```sh
cargo test
```

Environment variables select the context that the tests use:

- `SAFEGL_GL_VERSION`: the version of the context, such as `"OpenGL 4.1"` or `"OpenGL ES 3.2"`.
  Without it, the tests use the versions of the selected levels.
- `SAFEGL_GL_DISPLAY=egl`: create the context with EGL instead of GLX or WGL. Under Wayland, the
  tests use EGL anyway; unset `WAYLAND_DISPLAY` to use GLX through XWayland.
- `SAFEGL_ROBUSTNESS`: create the contexts with robust buffer access (`robust`), with robust
  buffer access and lost on GPU resets (`lose`), or without error checking (`no_error`), to run
  the suite in these contexts.

Tests of features that the context lacks return early, while every other error fails them. Mesa's
software renderer runs the whole suite (`LIBGL_ALWAYS_SOFTWARE=1`, plus
`__GLX_VENDOR_LIBRARY_NAME=mesa` or `__EGL_VENDOR_LIBRARY_FILENAMES` pointing to Mesa's EGL vendor
file on a machine with another driver). Set `MESA_SHADER_CACHE_DISABLE=true` when switching between
desktop OpenGL and OpenGL ES with Mesa: its shader cache can report as compiled a desktop GLSL
shader that OpenGL ES rejects, and its linker then crashes. `MESA_GL_VERSION_OVERRIDE` and
`MESA_GLES_VERSION_OVERRIDE` make Mesa run as an older version, such as OpenGL 4.0 or OpenGL ES 3.0:

```sh
MESA_GL_VERSION_OVERRIDE=4.0 MESA_GLSL_VERSION_OVERRIDE=400 LIBGL_ALWAYS_SOFTWARE=1 cargo test
MESA_GLES_VERSION_OVERRIDE=3.0 LIBGL_ALWAYS_SOFTWARE=1 cargo test --features gles30
```

What each level compiles is checked by a [Lua](https://www.lua.org) script, which builds small
crates against every level, the browser and macOS (`rustup target add wasm32-unknown-unknown
aarch64-apple-darwin`):

```sh
lua tests/levels/check_levels.lua
```

## Renderer examples

The [renderer collection](examples/renderers/README.md) contains 20 standalone
experiments covering forward, Forward+, deferred, and visibility rendering with
single or many lights and several anti-aliasing methods. All use OpenGL 4.6 and
share a Ferris model. The collection can be copied and run independently
with the published SafeGL crate.

## [SafeGL Book - Building a renderer from scratch]()

SafeGL book are design to teach you to implement a Forward+ renderer with MSAA anti-aliasing using Rust, OpenGL 4.6 and 
slang shaders that compiles to GLSL.

## [SafeGL Developer Handbook]()

SafeGL developer handbook explain architectural, implementations and other decisions related with the development
of the SafeGL and aim to help new developers to contribute to the project,

## When should I use SafeGL or raw OpenGL?

SafeGL is a good fit when you want to build a renderer with a Rust API that handles much of
OpenGL's resource management and state bookkeeping. You still design the rendering pipeline,
write shaders, and choose how to organize your data and draw calls.

[glow](https://github.com/grovesNL/glow), the bindings SafeGL itself is built on, is a better fit
when you need direct control over OpenGL calls and are prepared to manage their safety
requirements yourself. It also targets desktop OpenGL, OpenGL ES and WebGL with a single API.

### Choose SafeGL when

 - You want typed buffers, textures, and uniforms, with RAII to manage resource lifetimes
   and less manual work configuring vertex attributes and bindings.

 - You want to describe draw parameters explicitly while SafeGL manages framebuffer objects,
   samplers, vertex array objects, and cached OpenGL state to avoid redundant state changes.

 - You want validation that catches many invalid resource combinations and draw configurations
   before they reach OpenGL, reducing the amount of unsafe code in your application.

 - You want one API across desktop OpenGL, OpenGL ES and WebGL 2, with a level per version that
   decides at compile time which APIs are available, plus capability checks and fallback paths
   within that level. Individual features still depend on your context, hardware, and driver.

 - You want to focus on rendering techniques and application behavior while relying on an
   existing abstraction for common OpenGL operations.

### Choose lower-level bindings when

 - You need OpenGL features or extensions outside SafeGL's levels, or OpenGL versions below 4.0
   and OpenGL ES 3.0. You can call them through glow directly, or alongside SafeGL through
   `Context::glow_context`.

 - You are building your own graphics abstraction and need to define its resource ownership,
   state tracking, synchronization, and error-handling rules.

 - You are integrating with an existing renderer or external library that needs to control
   the same OpenGL context and state directly.

 - Profiling has identified a bottleneck in SafeGL's abstraction, and you need precise control
   over the underlying calls, bindings, or resource updates to address it.

 - You want to learn or experiment with the OpenGL API directly, including its state model
   and resource management requirements.

With raw bindings, you are responsible for context setup and current-context rules, function
loading, resource cleanup, valid state, synchronization, and feature availability. The bindings
provide access to OpenGL; these responsibilities remain with your application.

### Performance and interoperability

SafeGL caches state and uses optimized OpenGL paths where supported. Raw calls give you more
control, but performance depends on how you use that control. Measure your workload on the
hardware you intend to support before choosing lower-level bindings for performance reasons.

Mixing SafeGL with raw calls requires care: SafeGL tracks OpenGL state internally. SafeGL
re-exports glow as `safegl::glow`, and `Context::glow_context` returns the `glow::Context` it uses,
so libraries built on glow (such as `egui_glow`) can share the same context. Issue those calls
inside `exec_in_context`, then call `Context::restore_state`, which brings the OpenGL state back to
what SafeGL expects, before SafeGL is used again. For integrations that require frequent direct
state manipulation, using glow on its own may be simpler.

Limitations:

 - Robustness isn't supported everywhere yet, so you can still get crashes if you do incorrect
   things in your shaders.

 - SafeGL gives you access to all the tools but doesn't prevent you from doing horribly slow
   things. Some knowledge of modern techniques is required if you want to reach maximum
   performances.
