# OpenGL ES 3.0 to 3.2 devices

The `gles30`, `gles31` and `gles32` levels compile SafeGL for OpenGL ES: the GPUs of phones, of single-board computers and of embedded boards, and the OpenGL ES contexts of desktop drivers. This chapter tells:

- which devices fit each level;
- every OpenGL ES extension that SafeGL uses, with its API, the check, and what happens without it;
- the platforms of embedded devices: displays without window system, DRM/KMS with GBM, and Android;
- the behaviors of OpenGL ES that desktop drivers hide;
- what was verified, on which drivers, and what wasn't.

**The embedded devices aren't tested like desktop OpenGL and WebGL.** Every functionality of this chapter runs in the test suite on the OpenGL ES drivers of desktop GPUs (Mesa's llvmpipe and NVIDIA), and compiles for the architectures of embedded devices, but no phone or board has run the tests yet. Runs on real devices are welcome: [Running the tests on a device](#running-the-tests-on-a-device) tells how.

## The levels and the devices

A level is a ceiling: `gles31` compiles the API of OpenGL ES 3.0 and 3.1, and SafeGL checks at runtime what the driver has. Pick the level of the least capable device that the application targets.

| Level | Devices |
|-------|---------|
| `gles30` | Every OpenGL ES 3 device: older Mali (T6xx), Adreno 3xx, PowerVR Series 6, the Raspberry Pi 3 with Mesa, WebGL 2 browsers through `webgl` |
| `gles31` | Raspberry Pi 4 and 5 (V3D), Vivante GC7000 (i.MX 8), Mali-T8xx, most Android 7 devices |
| `gles32` | Mali-G, Adreno 5xx and later, PowerVR Rogue and later, NVIDIA Jetson, the Android devices with the Android extension pack |

On desktop GPUs, `gles30`, `gles31` and `gles32` run on the OpenGL ES contexts of Mesa and NVIDIA, which the test suite uses (`SAFEGL_GL_VERSION='OpenGL ES 3.0'`, `MESA_GLES_VERSION_OVERRIDE=3.0`).

## The extensions of OpenGL ES

Each extension is checked at runtime: without it, the API returns its error, never a panic nor a GL error. The tiers of the last column are:

- **A**: runs in the test suite on a driver that has the extension (Mesa's llvmpipe in OpenGL ES 3.0 and 3.2, NVIDIA in OpenGL ES 3.1 and 3.2), and with the extension hidden (`SAFEGL_DISABLE_EXTENSIONS`);
- **C**: no driver of the tests has it: the tests check the error without it, and the paths with it are written from the specification, unverified.

### Functionalities of later versions

| Functionality | Extensions below its version | Without them | Tier |
|---------------|------------------------------|--------------|------|
| `copy_image`, `copy_to` (core in 3.2) | `GL_EXT_copy_image`, `GL_OES_copy_image` | `CopyImageError::NotSupported` | A |
| `DrawParameters::outputs`, per-output blending and masks (3.2) | `GL_EXT_draw_buffers_indexed`, `GL_OES_draw_buffers_indexed` | `DrawError::PerOutputStateNotSupported` | A |
| Base vertices (3.2) | `GL_EXT_draw_elements_base_vertex`, `GL_OES_draw_elements_base_vertex` | vertex attributes bound at an offset | A |
| Ranges drawn with one call (`NoIndicesRanges`, `IndexBuffer::ranges`) | `GL_EXT_multi_draw_arrays` | drawn one by one | A |
| Texture views, at `gles31` | `GL_OES_texture_view`, `GL_EXT_texture_view` | `TextureViewError::NotSupported` | A |
| Viewport arrays, at `gles32` | `GL_OES_viewport_array` | `DrawError::ViewportArrayNotSupported` | A |
| Object labels, debug groups, inserted messages | `GL_KHR_debug` (core in 3.2), `GL_EXT_debug_label`, `GL_EXT_debug_marker` | `DebugError::NotSupported` | A |
| `Context::graphics_reset_status` | `GL_EXT_robustness`, `GL_KHR_robustness` (core in 3.2) | `None` | A |

### Textures

| Functionality | Extension | Without it | Tier |
|---------------|-----------|------------|------|
| Regions of ETC1 levels | `GL_EXT_compressed_ETC1_RGB8_sub_texture` | `UploadError::NotSupported`: levels are written whole | A |
| Samplers that don't decode sRGB (`srgb_decode(false)`) | `GL_EXT_texture_sRGB_decode` | `SamplerNotSupportedError::SrgbDecode` | A |
| `SrgbFormat::U8`, `SrgbFormat::U8U8` | `GL_EXT_texture_sRGB_R8`, `GL_EXT_texture_sRGB_RG8` | `TextureCreationError::FormatNotSupported` | A |
| `TextureAny::set_srgb_override` | `GL_EXT_texture_format_sRGB_override` | `SrgbOverrideError::NotSupported` | C |
| ASTC 2D (`CompressedFormat::Astc`) | `GL_KHR_texture_compression_astc_ldr`, `_hdr` | `FormatNotSupported` | A |
| ASTC slices of 3D textures | `GL_KHR_texture_compression_astc_sliced_3d`, `_hdr` | `FormatNotSupported` | A |
| ASTC 3D blocks (`Astc3d`) | `GL_OES_texture_compression_astc` | `FormatNotSupported` | C |
| `TextureAny::set_astc_decode_precision` | `GL_EXT_texture_compression_astc_decode_mode`, `_rgb9e5` | `AstcDecodePrecisionError::NotSupported` | C |
| ATC (`CompressedFormat::Atc*`), older Adreno | `GL_AMD_compressed_ATC_texture` | `FormatNotSupported` | C |
| PVRTC 1 and 2, sRGB PVRTC | `GL_IMG_texture_compression_pvrtc`, `pvrtc2`, `GL_EXT_pvrtc_sRGB` | `FormatNotSupported` | C |

### Tile-based GPUs

Most embedded GPUs render in tiles, in the memory of the chip, and write the tiles to memory at the end of each render pass. Writing them costs bandwidth and power, which these functionalities avoid:

| Functionality | Extension | Without it | Tier |
|---------------|-----------|------------|------|
| `invalidate` of framebuffers at the end of frames and passes (core) | | | A |
| Multisampled rendering into single-sample textures, resolved on the chip: `TextureAnyImage::multisampled`, `new_multisample_resolved` render buffers | `GL_EXT_multisampled_render_to_texture`, `2` for the attachments other than the first color one | `ValidationError::MultisampledRenderToTextureNotSupported` | A (NVIDIA) |
| Framebuffer reads in fragment shaders (`inout` outputs, `gl_LastFragColorARM`, `gl_LastFragDepthARM`), with barriers for `layout(noncoherent)` | `GL_EXT_shader_framebuffer_fetch` and `_non_coherent`, `GL_ARM_shader_framebuffer_fetch` and `_depth_stencil` | shaders don't compile | A (Mesa) |
| `framebuffer::PixelLocalStorage` | `GL_EXT_shader_pixel_local_storage`, `2` for its size and `clear` | `TilePassError::NotSupported` | C |
| `framebuffer::TiledRendering` | `GL_QCOM_tiled_rendering` | `TilePassError::NotSupported` | C |
| `BlendingFunction::Advanced` (core in 3.2) | `GL_KHR_blend_equation_advanced`, `_coherent` | `DrawError::AdvancedBlendingNotSupported` | A |

The passes of pixel local storage and tiled rendering borrow their surface. SafeGL ends them in OpenGL while another framebuffer is bound for drawing, which the extensions require, and starts them again when their surface is drawn to: a read or a draw elsewhere in the middle of a pass loses the pixel local storage, and the tile is written to memory, but no GL error happens.

### Shaders

The extensions of GLSL ES that SafeGL doesn't call are listed in `ExtensionsList`, so that applications choose their shaders: `GL_EXT_gpu_shader5`, `GL_EXT_shader_io_blocks`, `GL_OES_shader_image_atomic`, `GL_EXT_shader_non_constant_global_initializers`, `GL_NV_shader_noperspective_interpolation`, `GL_EXT_geometry_point_size`, `GL_EXT_tessellation_point_size`, `GL_EXT_shader_group_vote`, `GL_EXT_shader_integer_mix`, `GL_EXT_texture_shadow_lod`, and their `OES` variants. The `glsl_es_extensions` tests compile and draw with each.

### External images

Cameras, video decoders, other processes and other APIs share their images and their synchronization with OpenGL through EGL and the Linux kernel:

| Functionality | Extension | Without it | Tier |
|---------------|-----------|------------|------|
| `texture::ExternalTexture`, sampled by `samplerExternalOES`, YUV converted to RGB | `GL_OES_EGL_image_external`, `_essl3` | `TextureCreationError::TypeNotSupported` | A |
| Writes of YUV into external textures (`layout(yuv) out`) | `GL_EXT_YUV_target` | `ValidationError::FormatNotRenderable` | C |
| Dma-bufs imported as EGL images, with modifiers and YUV color spaces (`egl::Egl::import_dma_buf`) | `EGL_EXT_image_dma_buf_import`, `_modifiers` | `EglError::ExtensionMissing` | A |
| Images of textures, exported as dma-bufs | `EGL_KHR_gl_texture_2D_image`, `EGL_MESA_image_dma_buf_export` | `EglError::ExtensionMissing` | A |
| 2D textures whose storage is an image (`EglImage::texture_2d`) | `GL_EXT_EGL_image_storage` | `TextureCreationError::TypeNotSupported` | A |
| EGL fences, and native fences as file descriptors | `EGL_KHR_fence_sync`, `EGL_KHR_wait_sync`, `EGL_ANDROID_native_fence_sync` | `EglError::ExtensionMissing` | A |
| Memory and semaphores of Vulkan (`interop::MemoryObject`, `interop::Semaphore`) | `GL_EXT_memory_object_fd`, `GL_EXT_semaphore_fd` | `InteropError::NotSupported` | A (NVIDIA) |

## Platforms

### Displays without window system

`backend::glutin::headless` creates displays from the EGL display of a device (`EGL_EXT_platform_device`): a GPU, or the software renderer of Mesa (`EGL_MESA_device_software`). Their frames go to a pbuffer of a fixed size, and nothing needs X11 nor Wayland, so they run on servers, in CI and on boards without a window system:

```rust,ignore
use safegl::backend::glutin::headless::Headless;

let headless = Headless::first()?;
let display = headless.display(1280, 720, Default::default())?;
```

The [`headless`](../examples/standalone/headless.rs) example draws a triangle and saves it as a PNG file. NVIDIA's driver (version 615) fails to make an EGL context current while a GLX context is made current on another thread: a program that has both creates them from one thread, or one after the other. The tests run on such displays with `SAFEGL_TEST_DISPLAY=headless` at build time, on the device whose name, vendor or DRM node contains `SAFEGL_EGL_DEVICE` (`software` for Mesa's), or on the first device that creates a context of the level.

### DRM/KMS and GBM

Kiosks and embedded boards draw fullscreen without a compositor: the frames are drawn on a GBM surface and shown with the page flips of KMS. glutin creates the EGL display from the GBM device and the surface from the GBM surface, as it does from a window, so the display is an ordinary `Display<WindowSurface>`. The [`kiosk`](../examples/standalone/kiosk.rs) example does it, from a text console.

### Android

Android destroys the window of a paused application and gives it a new one when it is resumed. `Display::remove_surface`, before the window goes (`Event::Suspended` of winit), keeps the context and every object of SafeGL, the context being current without surface (`EGL_KHR_surfaceless_context`); `Display::set_surface` gives it the surface of the new window. Between the two, frames return `FrameError::SurfaceLost`, but textures, buffers and programs can still be created and drawn to as framebuffers. The [`android`](../examples/standalone/android.rs) example builds as the shared library of a native activity, with `cargo-apk` or `xbuild`.

## What desktop drivers hide

**Precision.** Desktop drivers compute `mediump` and `lowp` as `highp`; mobile GPUs compute `mediump` with 16-bit floats, 10 bits of precision: enough for colors, not for the coordinates of pixels beyond 1024 or for positions. Vertex shaders are `highp` by default; fragment shaders need a declaration, and `highp` there is guaranteed from OpenGL ES 3.0. `Context::shader_precision_format` tells the precision of each qualifier, and the `precision_of_every_qualifier` test prints them on each device.

**Uploads.** OpenGL ES only uploads the data whose format and type the internal format takes (table 3.2 of OpenGL ES 3.0): RGBA bytes into a texture of one component, or floats into bytes, are errors there, where desktop OpenGL converts them. SafeGL converts such data on the CPU the way desktop OpenGL does, and returns `FormatNotSupported` when no exact conversion exists: floats into integer textures, and uploads from pixel buffers, whose data stays on the GPU.

**Limits.** Devices often have the minimums of their version: textures of 2048 pixels, 4 samples, 4 draw buffers, a single binding of atomic counter buffers in OpenGL ES 3.1, debug messages of a single byte. SafeGL checks each limit before calling OpenGL, and debug builds lower the limits to the minimums with `SAFEGL_CLAMP_LIMITS=es30`, `es31` or `es32`, so that tests run as on such devices; the whole suite passes so.

**Program binaries.** Their formats are the driver's, and some drivers have none (`GetBinaryError::NoFormats`): a binary saved on a device is only for that device and driver version, and `Program::new` with a binary of another one returns an error, after which the program is built from source.

**Reading back.** OpenGL ES reads textures through framebuffers, so the formats that can't be attached to them can't be read (`ReadError::NotRenderable`).

## Verification

**Tier A, on this repository's machines.** The whole test suite runs on:

- Mesa's llvmpipe in OpenGL ES 3.0 and 3.2, with windows and without window system;
- NVIDIA in OpenGL ES 3.1 and 3.2;
- OpenGL ES 3.0 with the extensions of this chapter hidden;
- the three levels with `SAFEGL_CLAMP_LIMITS`.

**Tier B, other architectures.**

- Every level compiles for aarch64, armv7, riscv64 and Android (`tests/cross/check_targets.lua`, and the cross builds of CI).
- The whole suite runs as an i686 program, on 32-bit Mesa (the i686 job of CI).
- QEMU runs the suite of aarch64 with a sysroot of aarch64 Mesa (`check_targets.lua test-aarch64 <sysroot>`).

**Tier C, unverified.** No real device has run the tests:

- the GPUs of phones and boards (Mali, Adreno, PowerVR, V3D, Vivante), whose drivers have their own bugs;
- the real precision of `mediump` and `lowp`;
- the real behavior of tiles (pixel local storage, tiled rendering, the resolve of multisampled rendering);
- Android, which only compiles, and GBM, which needs the screen of a text console;
- the extensions of tier C above.

## Running the tests on a device

On a board with Linux, Rust and the OpenGL ES driver of its GPU, from the root of the repository:

```sh
lua tests/cross/embedded_device_tests.lua
```

The script finds the OpenGL ES versions that the driver creates, runs the suite at each level on a display without window system (or a window, with `--window`), and writes a report of the driver, its extensions, its limits, the precision of its shaders and the failures. Sharing this report in an issue is the most useful contribution to the embedded support: it tells which drivers work, and which need fixes.
