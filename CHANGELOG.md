# Changelog

All notable changes to this project are documented in this file. Versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

Releases published as `glium` before the fork are preserved in
[CHANGELOG-glium.md](CHANGELOG-glium.md).
