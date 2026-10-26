# Changelog

All notable changes to this project are documented in this file. Versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
