# WebGL 2

The `webgl` level compiles SafeGL for browsers, on the `wasm32-unknown-unknown` target, where it draws with the WebGL 2 context of a `<canvas>`. WebGL 2 is OpenGL ES 3.0 with its own rules: some things that OpenGL ES 3.0 allows are errors there, a few functions are missing, and many functionalities of desktop OpenGL and of later OpenGL ES versions come as WebGL extensions instead. This chapter tells how SafeGL handles each of them:

- how a `WebDisplay` is created, configured and survives the loss of its context;
- every rule of WebGL 2 that OpenGL ES 3.0 doesn't have, and the SafeGL error that reports it before WebGL raises a GL error;
- every extension of the WebGL registry that WebGL 2 has, with the SafeGL API that uses it;
- how the browser tests run, and what they don't verify.

A WebGL 2 program can only use what the browser gives it: SafeGL checks every extension at runtime, like the extensions of drivers, and returns an error, never a panic, when the browser lacks one.

## Creating the display

```toml
[dependencies]
safegl = { version = "...", default-features = false, features = ["webgl"] }
```

`WebDisplay::from_canvas(canvas)` creates the WebGL 2 context with the defaults of the browser: an alpha channel, a depth buffer, no stencil buffer, antialiasing, premultiplied alpha, and a drawing buffer that is cleared after each frame. `WebDisplay::builder(canvas)` chooses them:

```rust,ignore
use safegl::backend::web::{PowerPreference, WebDisplay};

let display = WebDisplay::builder(canvas)
    .stencil(true)
    .antialias(false)
    .power_preference(PowerPreference::HighPerformance)
    .build()?;
```

The builder sets every attribute of `WebGLContextAttributes`: `alpha`, `antialias`, `depth`, `stencil`, `premultiplied_alpha`, `preserve_drawing_buffer`, `power_preference`, `fail_if_major_performance_caveat` and `desynchronized`, plus the debug behavior of SafeGL. `build` returns `WebDisplayCreationError::WebGl2NotSupported` when the browser has no WebGL 2 for the canvas, and `IncompatibleOpenGl` when it lacks what SafeGL needs.

A canvas is sized in CSS pixels by the page and in pixels by its drawing buffer. `WebDisplay::resize_to_display(css_size, device_pixel_ratio)` gives the drawing buffer one pixel per pixel of the screen (`window.devicePixelRatio`), so that the drawing is sharp on high-density screens, and returns the size that the browser gave, which can be smaller than requested.

Each frame is drawn between `display.draw()` and `frame.finish()`, before control returns to the browser: the browser shows the drawing buffer at the end of the task, and there is no buffer swap to wait for.

### Losing the context

Browsers take the WebGL context away when the GPU is reset, when its driver is updated, or when a page has too many contexts. SafeGL listens to the `webglcontextlost` event and asks the browser to restore the context; from then on:

- `display.is_context_lost()` returns true;
- the draws, uploads and reads do nothing, `frame.finish()` returns `SwapBuffersError::ContextLost`, and the creation of objects returns their `ContextLost` errors, never a panic;
- once the browser has restored the context (the `webglcontextrestored` event), `display.is_context_restored()` returns true, and `display.restore()` returns a new display that draws with it.

Every object of the lost context is gone: the application creates its buffers, textures and programs again with the new display. The objects of the old display can only be dropped; the new display refuses them with the `ContextMismatch` errors. [`examples/web`](../examples/web) shows the whole cycle, and a page can trigger it with `WEBGL_lose_context`.

### Debug output

WebGL has no debug messages. With `DebugCallbackBehavior::DebugMessageOnError` (the default of debug builds), SafeGL calls `getError` after each operation and reports the errors as messages of high severity from the API, which the default callback prints to the console of the browser. `getError` waits for the GPU process of the browser, so this costs much more than on native drivers: release builds don't do it, and `DebugCallbackBehavior::Ignore` turns it off.

## The rules of WebGL 2

Each rule that WebGL 2 adds to OpenGL ES 3.0 is checked by SafeGL before the call, and returns an error that names it. The tests of the `webgl_restrictions` file break each rule in Firefox, and check that WebGL raised no error.

| Rule of WebGL 2 | SafeGL error | Tests |
|-----------------|--------------|-------|
| The stencil reference value and masks are the same for front and back faces | `DrawError::StencilFacesMismatch` | `webgl_restrictions`, `stencil` |
| Blending can't use a constant color factor and a constant alpha factor together | `DrawError::BlendConstantsMismatch` | `webgl_restrictions` |
| The near value of the depth range isn't above the far value | `DrawError::InvalidDepthRange` | `webgl_restrictions`, `draw_parameters` |
| An index buffer is never copied to or from a buffer of another type | `CopyError::IndexBufferMismatch` | `webgl_restrictions` |
| A draw can't sample a texture level that it renders to (a feedback loop) | `DrawError::FeedbackLoop`, on every API without texture barriers | `feedback_loop` |
| The names of attributes and outputs don't start with `webgl_` or `_webgl_`, and have 1024 bytes at most | `ProgramCreationError::InvalidLinkLocation` | `link_locations` |
| Primitives always restart at the largest index | `DrawError::PrimitiveRestartAlwaysOn` when `PrimitiveRestart::Disabled` is drawn with indices that may hold it | `primitive_restart`, `webgl_restrictions` |
| Blending into 32-bit float attachments needs `EXT_float_blend` | `DrawError::FloatBlendNotSupported` | `format_rendering` |
| The S3TC, RGTC and BPTC textures have a level 0 of whole blocks | `TextureCreationError::DimensionsNotSupported` | `texture_creation` |
| The ETC1 and PVRTC textures are written whole, and PVRTC textures have power-of-two sizes | `UploadError::NotSupported`, `TextureCreationError::DimensionsNotSupported` | `texture_kinds` |
| Lines are 1 pixel wide | `DrawError::LineWidthNotSupported` | `draw_parameters` |
| The draws to several views of a multiview framebuffer can't capture transform feedback nor be timed, and the views can't be read | `DrawError::MultiviewRestricted`, `BlitError::Multiview` | `multiview` |

Other differences need no error, because SafeGL does the same thing another way:

- **Mapped buffers.** WebGL can't map buffers: `map`, `map_read` and `map_write` give a copy of the buffer, written back with `bufferSubData` when it is dropped, and `read` uses `getBufferSubData`.
- **Fences.** WebGL only signals fences between two tasks of the event loop of the browser, and limits the timeout of `clientWaitSync` to `MAX_CLIENT_WAIT_TIMEOUT_WEBGL`, which is often 0: `SyncFence::wait_timeout` with a zero timeout polls, and a longer one calls `glFinish`. The results of queries also arrive in a later task: `is_ready` stays false within the task that ended the query.
- **Reads.** `readPixels` only reads the formats that WebGL allows for each kind of attachment: SafeGL reads normalized, integer and float attachments in those formats, and converts them.
- **Depth textures** are never filtered linearly, and 32-bit float textures are only filtered with `OES_texture_float_linear`: SafeGL samples them without filtering otherwise, where WebGL would read zeros.
- **Program binaries**, `glReleaseShaderCompiler`, client-side vertex arrays and `glGetTexImage` don't exist: they aren't in the `webgl` level, or SafeGL doesn't need them.
- **Uniform blocks** are reflected with `getActiveUniforms`, which glow doesn't call on the web.

## Extensions

SafeGL enables every extension of this table that the browser has when the display is created (`getExtension`), so that their functions, their constants and their GLSL `#extension` are available, and records them in `ExtensionsList`: the extensions that OpenGL ES has too set the same fields (`EXT_color_buffer_float` sets `gl_ext_color_buffer_float`), and the ones that only WebGL has set the fields of their own name (`WEBGL_multi_draw` sets `webgl_multi_draw`). The API of an extension is at the `webgl` level, and its runtime check returns the error of the last column without it.

| Extension | SafeGL API | Without it | Tests |
|-----------|------------|------------|-------|
| `EXT_clip_control` | `DrawParameters::{clip_control_origin, clip_control_depth}` | `DrawError::ClipControlNotSupported` | `rasterization` |
| `EXT_color_buffer_float` | rendering to float formats, reading them | `ValidationError::FormatNotRenderable` | `format_rendering`, `webgl_extensions` |
| `EXT_color_buffer_half_float` | rendering to half-float formats | `ValidationError::FormatNotRenderable` | `format_rendering` |
| `EXT_conservative_depth` | GLSL only: `layout(depth_greater)` and the others | the shader doesn't compile | `glsl_es_extensions` |
| `EXT_depth_clamp` | `Depth::clamp` (`DepthClamp::Clamp`) | `DrawError::DepthClampNotSupported` | `draw_parameters` |
| `EXT_disjoint_timer_query_webgl2` | `TimeElapsedQuery`, `debug::TimestampQuery`, `Context::gpu_timestamp` | `QueryCreationError::NotSupported`, `None` | `query` |
| `EXT_float_blend` | blending into 32-bit float attachments | `DrawError::FloatBlendNotSupported` | `format_rendering` |
| `EXT_polygon_offset_clamp` | `DrawParameters::polygon_offset_clamp` | `DrawError::PolygonOffsetClampNotSupported` | `polygon_offset_clamp` |
| `EXT_render_snorm` | rendering to signed normalized formats | `ValidationError::FormatNotRenderable` | `format_rendering` |
| `EXT_texture_compression_bptc` | `CompressedFormat::{BptcUnorm4, BptcSignedFloat3, BptcUnsignedFloat3}`, `CompressedSrgbFormat::Bptc` | `TextureCreationError::FormatNotSupported` | `texture_kinds` |
| `EXT_texture_compression_rgtc` | `CompressedFormat::Rgtc*` | `TextureCreationError::FormatNotSupported` | `texture_kinds` |
| `EXT_texture_filter_anisotropic` | `Sampler::anisotropy`, `Context::get_max_anisotropy_support` | `DrawError::SamplerNotSupported` (`SamplerNotSupportedError::Anisotropy`) above 1 | `samplers`, `webgl_extensions` |
| `EXT_texture_mirror_clamp_to_edge` | `SamplerWrapFunction::MirrorClamp` | `DrawError::SamplerNotSupported` | `samplers` |
| `EXT_texture_norm16` | the 16-bit normalized formats, rendering to them | `TextureCreationError::FormatNotSupported` | `format_rendering`, `texture_formats` |
| `KHR_parallel_shader_compile` | `Program::new_pending`, `PendingProgram::is_ready` | the program is ready at once | `parallel_compile` |
| `OES_draw_buffers_indexed` | `DrawParameters::outputs` (`OutputState`) | `DrawError::PerOutputStateNotSupported` | `output_state` |
| `OES_sample_variables` | GLSL only: `gl_SampleID`, `gl_SamplePosition`, `gl_SampleMaskIn` | the shader doesn't compile | `glsl_es_extensions` |
| `OES_shader_multisample_interpolation` | GLSL only: `sample` qualifiers, `interpolateAt*` | the shader doesn't compile | `glsl_es_extensions` |
| `OES_texture_float_linear` | linear filtering of 32-bit float textures | the textures are sampled without filtering | `format_rendering` |
| `OVR_multiview2` | `LayeredFrameBuffer::{multiview, multiview_with_outputs}` | `ValidationError::MultiviewNotSupported` | `multiview` |
| `WEBGL_blend_func_extended` | dual-source blending (the `LinearBlendingFactor::SourceOne*` factors, outputs of index 1) | `DrawError::DualSourceBlendingNotSupported` | `output_state` |
| `WEBGL_clip_cull_distance` | `DrawParameters::clip_planes_bitmask`, `gl_ClipDistance`, `gl_CullDistance`, their limits | `DrawError::ClipPlaneIndexOutOfBounds` | `state_extensions` |
| `WEBGL_compressed_texture_astc` | `CompressedFormat::Astc`, `CompressedSrgbFormat::Astc` (LDR block sizes) | `TextureCreationError::FormatNotSupported` | `texture_kinds` |
| `WEBGL_compressed_texture_etc` | the ETC2 and EAC formats | `TextureCreationError::FormatNotSupported` | `texture_kinds` |
| `WEBGL_compressed_texture_etc1` | `CompressedFormat::Etc1Rgb8` | `TextureCreationError::FormatNotSupported` | `texture_kinds` |
| `WEBGL_compressed_texture_pvrtc` | `CompressedFormat::Pvrtc*` | `TextureCreationError::FormatNotSupported` | `texture_kinds` |
| `WEBGL_compressed_texture_s3tc`, `WEBGL_compressed_texture_s3tc_srgb` | `CompressedFormat::S3tc*`, `CompressedSrgbFormat::S3tc*` | `TextureCreationError::FormatNotSupported` | `texture_kinds`, `texture_creation` |
| `WEBGL_debug_renderer_info` | `Context::get_opengl_renderer_string` and `get_opengl_vendor_string` give the real GPU when the browser allows it | the masked strings of the browser | `webgl_extensions` |
| `WEBGL_draw_instanced_base_vertex_base_instance` | `DrawParameters::base_instance`, base vertices of index buffers and ranges | `DrawError::BaseInstanceNotSupported`; base vertices bind the attributes at an offset instead | `base_instance`, `multi_draw` |
| `WEBGL_lose_context` | used by the tests and by pages to lose the context | — | `webgl_context` |
| `WEBGL_multi_draw` | ranges drawn with one call (`index::NoIndicesRanges`, `IndexBuffer::ranges`) | the ranges are drawn one by one | `multi_draw` |
| `WEBGL_multi_draw_instanced_base_vertex_base_instance` | the same ranges with base vertices and base instances | drawn one by one | `multi_draw` |
| `WEBGL_polygon_mode` | `DrawParameters::polygon_mode` (`Line`, `Fill`), `PolygonOffset::line` | `DrawError::PolygonModeNotSupported`, `PolygonOffsetNotSupported` | `state_extensions` |
| `WEBGL_provoking_vertex` | `DrawParameters::provoking_vertex` | `DrawError::ProvokingVertexNotSupported` | `draw_parameters` |
| `WEBGL_render_shared_exponent` | rendering to `F9F9F9` | `ValidationError::FormatNotRenderable` | `format_rendering` |
| `WEBGL_stencil_texturing` | `TextureAny::set_depth_stencil_mode` (sampling the stencil of depth-stencil textures) | `DepthStencilTextureModeError::NotSupported` | `texture_parameters` |

WebGL 2 also takes textures from what the page shows. `Texture2d::from_web_image` and `SrgbTexture2d::from_web_image` build a texture of `U8U8U8U8` texels from an image, canvas or video element, an image bitmap, image data or an offscreen canvas, and `write_web_image` writes one into a texture, through the `WebImageSource` trait. `WebImageOptions` sets the conversions of the pixel store of WebGL for the write: the flip of the rows (`UNPACK_FLIP_Y_WEBGL`), the premultiplication by alpha and the conversion of the colors to the color space of the browser; they don't outlive the write, and `restore_state` resets them when other code changed them. The browser refuses the images of another origin without the permission of their server, and the images that haven't loaded: `UploadError::ImageRefused` carries its message. The `web_image` tests write image data and canvases with each option and read them back.

## Tests in browsers

The test suite runs in headless Firefox, through `wasm-bindgen-test-runner` and `geckodriver`, in two groups: with every extension of the browser, and with the extensions of the table hidden (`SAFEGL_DISABLE_EXTENSIONS`, read at compile time in browsers), which runs the paths of browsers that lack them. Each test creates its own canvas and display, loses its context when it ends so that the page doesn't run out of contexts, and checks that WebGL raised no error. The tests that need native contexts (window systems, SPIR-V files read from the disk, unwinding panics) are left out of the browser build.

```sh
cargo test --target wasm32-unknown-unknown --no-default-features --features webgl --tests
```

with `wasm-bindgen-cli` of the version in `Cargo.lock` and `geckodriver` installed, and `.cargo/config.toml` naming `wasm-bindgen-test-runner` as the runner of the target. The continuous integration runs the same job in Firefox.

What these tests don't verify:

- **Chrome and Safari.** Only Firefox runs the tests. Chrome draws through ANGLE on every platform, and Safari through its own ANGLE on Metal; both have extensions that Firefox doesn't (and the reverse), whose paths only the hidden-extension group exercises.
- **Real GPUs in browsers.** The tests run on the GPU of one machine; other GPUs, drivers and their workarounds in the browsers, and mobile browsers, aren't covered. The extensions of the table that the test browser doesn't have (such as `WEBGL_compressed_texture_pvrtc` and `WEBGL_compressed_texture_astc` on desktop GPUs) are only checked for their errors.
- **Multiview on NVIDIA.** Firefox on NVIDIA drivers has `OVR_multiview2`, but translates the shaders to GLSL 1.50, for which the driver refuses the extension: the multiview tests skip there after checking the errors that SafeGL returns itself, and run in Firefox drawing with Mesa (`__EGL_VENDOR_LIBRARY_FILENAMES` set to Mesa's vendor file), and natively on both drivers.
- **Losing the context for real.** The tests lose the context with `WEBGL_lose_context`; a GPU reset goes through the same events, but isn't reproduced.
