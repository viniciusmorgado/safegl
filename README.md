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
- **Support a small set of well-established OpenGL feature tiers**, selected with Cargo features,
  instead of the whole of OpenGL.
- **Produce optimized OpenGL function calls**, and make modern OpenGL techniques easy to use.

SafeGL validates how you use OpenGL, but it cannot detect or work around bugs in the drivers
themselves.

## OpenGL feature tiers

Glium, SafeGL's predecessor, tries to expose the whole of OpenGL across a wide range of versions.
SafeGL deliberately supports only a small set of well-established feature tiers, selected with
Cargo features. Each tier includes everything in the tiers below it.

| Feature  | API level                                     | Typical targets                                                                                |
|----------|-----------------------------------------------|------------------------------------------------------------------------------------------------|
| `gl46`   | Desktop OpenGL 4.6 core (**default**)         | Windows and Linux with current NVIDIA, AMD, Intel or Mesa drivers; Asahi Linux on Apple M1/M2   |
| `gl41`   | Desktop OpenGL 4.1 core                       | macOS (Intel and Apple Silicon), older GPUs, most virtual machines and software renderers       |
| `gles30` | Common subset of OpenGL ES 3.0 and OpenGL 4.1 | Embedded Linux (e.g. Raspberry Pi 4/5) and other OpenGL ES 3.0-class devices                     |
| `webgl`  | WebGL 2, the `gles30` tier in a browser       | Browsers, through `wasm32-unknown-unknown` (see [examples/web](examples/web))                  |

Selecting a tier above what the target platform supports is a compile error: `gl46` on macOS, or
anything above `gles30` in the browser.

### APIs that need `gl46`

The `gl41`, `gles30` and `webgl` tiers only expose what OpenGL 4.1 and OpenGL ES 3.0 provide.
The following APIs exist only when the `gl46` feature is enabled. Other builds don't compile them
at all, even where a 4.1 driver offers them as an extension:

| Functionality       | Core since                        | Gated items                                                                                                                                  |
|---------------------|-----------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| Compute shaders     | OpenGL 4.3                        | `program::ComputeShader`, `program::ComputeCommand`, `BufferType::DispatchIndirectBuffer`                                                    |
| Shader storage      | OpenGL 4.3                        | `BufferType::ShaderStorageBuffer`, `Program::get_shader_storage_blocks`, binding buffers to GLSL `buffer` blocks                             |
| Atomic counters     | OpenGL 4.2                        | `BufferType::AtomicCounterBuffer`, `Program::get_atomic_counters`, binding buffers to `atomic_uint` uniforms                                 |
| Image load/store    | OpenGL 4.2                        | `uniforms::{ImageUnit, ImageUnitBehavior, ImageUnitAccess, ImageUnitFormat, ImageUnitError}`, `image_unit()` on textures, `UniformValue::*Image*`, `DrawError::InsufficientImageUnits` |
| Multidraw indirect  | OpenGL 4.3                        | `index::{DrawCommandsNoIndicesBuffer, DrawCommandsIndicesBuffer, DrawCommandNoIndices, DrawCommandIndices}`, `IndicesSource::Multidraw*`     |
| Empty framebuffers  | OpenGL 4.3                        | `framebuffer::EmptyFrameBuffer`                                                                                                              |
| Copying images      | OpenGL 4.3                        | `texture::{copy_image, is_copy_image_supported, CopyImageTarget, CopyImageError}`, `copy_to()` on mipmaps                                 |
| Clearing textures   | OpenGL 4.4                        | `texture::{is_clear_texture_supported, ClearTextureError}`, `clear()`, `clear_region()` and `clear_depth_stencil()` on mipmaps            |
| Query buffers       | OpenGL 4.4                        | `to_buffer_u32()` on queries, `draw_parameters::ToBufferError`, `BufferType::QueryBuffer`                                                    |
| Texture barrier     | OpenGL 4.5                        | `texture::{texture_barrier, is_texture_barrier_supported, TextureBarrierNotSupportedError}`                                                 |
| Clip control        | OpenGL 4.5                        | `DrawParameters::{clip_control_origin, clip_control_depth}`, `ClipControlOrigin`, `ClipControlDepth`, `DrawError::ClipControlNotSupported`   |
| Polygon offset clamp | OpenGL 4.6                       | `DrawParameters::polygon_offset_clamp`, `DrawError::PolygonOffsetClampNotSupported`                                                         |
| SPIR-V shaders      | OpenGL 4.6                        | `ProgramCreationInput::SpirV`, `program::{SpirvProgram, SpirvEntryPoint}`                                                                    |
| Bindless textures   | never (`ARB_bindless_texture`)    | `texture::bindless`, `texture::{ResidentTexture, TextureHandle, BindlessTexturesNotSupportedError}`, `resident()` on textures                 |

A `gl46` build still runs on an older driver, so these APIs keep their runtime checks: for example
`ComputeShader::is_supported` returns `false` and `resident()` returns an error when the driver lacks
the version or extension. Optional features that every tier exposes (tessellation or geometry
shaders on OpenGL ES 3.0, for example) are checked at runtime in the same way. Among them:

| Functionality       | Needs                                         | Items                                                                                    |
|---------------------|-----------------------------------------------|------------------------------------------------------------------------------------------|
| Sample shading      | OpenGL 4.0, OpenGL ES 3.2; not WebGL          | `DrawParameters::sample_shading`, `DrawError::SampleShadingNotSupported`                 |
| Sample mask         | OpenGL 3.2, OpenGL ES 3.1; not WebGL          | `DrawParameters::sample_mask`, `DrawError::SampleMaskNotSupported`                       |
| Per-output state    | OpenGL 4.0, OpenGL ES 3.2; not WebGL          | `DrawParameters::outputs`, `OutputState`, `DrawError::PerOutputStateNotSupported`        |
| Invalidation        | OpenGL 4.3, OpenGL ES 3.0, WebGL 2            | `invalidate()` and `invalidate_rect()` on framebuffers, `framebuffer::InvalidateAttachments`; a no-op elsewhere |

`DrawParameters::alpha_to_coverage` is core everywhere, WebGL 2 included.

`BufferMode::Persistent` and `BufferMode::Immutable` are available in every tier. They use
`glBufferStorage` (OpenGL 4.4) when the driver has it and fall back to `glBufferData` otherwise.

`gl46` is enabled by default:

```toml
[dependencies]
safegl = "0.1"
```

To select a lower tier, disable the default features:

```toml
[dependencies]
safegl = { version = "0.1", default-features = false, features = ["gl41", "glutin_backend", "glutin/default"] }
```

The default `backtrace` feature prints a backtrace along with the OpenGL debug messages that
SafeGL reports as errors; leave it out to avoid compiling the `backtrace` crate.

For the browser, build for `wasm32-unknown-unknown` with the `webgl` feature and draw through
`safegl::backend::web::WebDisplay`:

```toml
[dependencies]
safegl = { version = "0.1", default-features = false, features = ["webgl"] }
```

### Platform notes

- **macOS** stops at OpenGL 4.1, and Apple has deprecated OpenGL since macOS 10.14. Enabling `gl46`
  on macOS is a compile error; use `gl41`.
- **Asahi Linux** on Apple M1/M2 ships a conformant OpenGL 4.6 driver, so the same hardware that is
  limited to `gl41` under macOS can use `gl46` under Linux.
- **Raspberry Pi 4/5** and similar boards expose OpenGL ES 3.1 but only desktop OpenGL 3.1; use
  `gles30`.
- **OpenGL ES** has no 1D textures (`TextureCreationError::TypeNotSupported`) and fewer formats
  than desktop OpenGL; `is_supported` on a format tells whether the context has it. Data uploaded
  to a texture must match its format, so a texture created from data gets a format chosen from
  that data. Integer and float textures are read back as `(i32, i32, i32, i32)`,
  `(u32, u32, u32, u32)` or `(f32, f32, f32, f32)`, and normalized ones as `(u8, u8, u8, u8)`.
- **Virtual machines and software renderers** (VirtualBox, VMware, Mesa llvmpipe) usually expose
  OpenGL 4.1–4.5; use `gl41`.
- **Windows without a vendor driver** (Microsoft's basic display driver, some Remote Desktop
  sessions) only provides OpenGL 1.1 and is not supported by any tier.
- **Browsers** provide WebGL 2, which SafeGL treats as OpenGL ES 3.0. WebGL can't map buffers or
  block on fences, so buffer mappings are emulated with copies and `SyncFence::wait` returns after
  `glFinish`. Shaders must start with `#version 300 es` on their very first line.

> [!NOTE]
> A tier describes which APIs SafeGL compiles in. It does not guarantee that the driver on a given
> machine actually provides that version. Whatever the tier, SafeGL refuses to create a context
> below desktop OpenGL 4.1 or OpenGL ES 3.0, and `SimpleWindowBuilder` requests the newest context
> version of the compiled tier.

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

The examples that need OpenGL 4.2+ features require the `gl46` feature, which is enabled by
default; the list is in `Cargo.toml`.

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

 - You want one API across desktop OpenGL, OpenGL ES and WebGL 2, with a feature tier that
   decides at compile time which APIs are available, plus capability checks and fallback paths
   within that tier. Individual features still depend on your context, hardware, and driver.

 - You want to focus on rendering techniques and application behavior while relying on an
   existing abstraction for common OpenGL operations.

### Choose lower-level bindings when

 - You need OpenGL features or extensions outside SafeGL's feature tiers. You can call them
   through glow directly, or alongside SafeGL through `Context::glow_context`.

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
inside `exec_in_context`, and restore any OpenGL state your code changes before SafeGL is used
again. For integrations that require frequent direct state manipulation, using glow on its own
may be simpler.

Limitations:

 - Robustness isn't supported everywhere yet, so you can still get crashes if you do incorrect
   things in your shaders.

 - SafeGL gives you access to all the tools but doesn't prevent you from doing horribly slow
   things. Some knowledge of modern techniques is required if you want to reach maximum
   performances.
