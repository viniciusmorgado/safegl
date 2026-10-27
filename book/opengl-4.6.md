# OpenGL 4.6 coverage

This chapter maps every command of the OpenGL 4.6 core profile, 658 of them, to SafeGL. SafeGL covers the functionality of OpenGL safely rather than its commands one by one:

- 334 commands are called by SafeGL, behind the API that each row names;
- 324 commands aren't called by SafeGL, because another command gives the same result. These are another form of the same command (scalar or vector, signed or unsigned, single or array), the direct state access and multi-bind forms of commands that SafeGL calls on bound objects, and queries of state that SafeGL sets itself.

Every row tells how to use the functionality of its command in SafeGL. For a command that SafeGL doesn't call, it names the API that gives the same result: to set a uniform of a program pipeline, which OpenGL does with `glProgramUniform*`, give the value with `uniform!` to a draw command of the pipeline; to copy pixels of a framebuffer into a texture, which OpenGL does with `glCopyTexSubImage2D`, blit them to the surface of the texture. The rows that say *Not needed* are queries of what SafeGL already knows and gives through its types and the parameters of its calls.

The level that compiles each API is in the table of levels of the README, and the functionality that only exists in shaders (GLSL 4.60) is tested by the `glsl_coverage` tests of the repository.

## Context, state and errors

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glDisable` | 1.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glEnable` | 1.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glFinish` | 1.0 | yes | `Context::finish`, `Context::flush` |
| `glFlush` | 1.0 | yes | `Context::finish`, `Context::flush` |
| `glGetBooleanv` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetDoublev` | 1.0 | no | Use what SafeGL gives: `Capabilities`, `Limits`, `Context::get_opengl_version` and the getters of each object; the state of draw commands is the `DrawParameters` that you give |
| `glGetError` | 1.0 | yes | `Context::assert_no_error`, and the checks after linking programs |
| `glGetFloatv` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetIntegerv` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetString` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glIsEnabled` | 1.0 | no | Not needed: the state of each draw command is the `DrawParameters` given to it, and `Capabilities` tells what the context supports |
| `glGetPointerv` | 4.3 | no | Use what SafeGL gives: `Capabilities`, `Limits`, `Context::get_opengl_version` and the getters of each object; the state of draw commands is the `DrawParameters` that you give |
| `glGetBooleani_v` | 3.0 | no | Use what SafeGL gives: `Capabilities`, `Limits`, `Context::get_opengl_version` and the getters of each object; the state of draw commands is the `DrawParameters` that you give |
| `glGetIntegeri_v` | 3.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glEnablei` | 3.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glDisablei` | 3.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glIsEnabledi` | 3.0 | no | Not needed: the state of each draw command is the `DrawParameters` given to it, and `Capabilities` tells what the context supports |
| `glGetStringi` | 3.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetInteger64v` | 3.2 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetInteger64i_v` | 3.2 | no | Use what SafeGL gives: `Capabilities`, `Limits`, `Context::get_opengl_version` and the getters of each object; the state of draw commands is the `DrawParameters` that you give |
| `glReleaseShaderCompiler` | 4.1 | yes | `Context::release_shader_compiler` |
| `glGetFloati_v` | 4.1 | no | Use what SafeGL gives: `Capabilities`, `Limits`, `Context::get_opengl_version` and the getters of each object; the state of draw commands is the `DrawParameters` that you give |
| `glGetDoublei_v` | 4.1 | no | Use what SafeGL gives: `Capabilities`, `Limits`, `Context::get_opengl_version` and the getters of each object; the state of draw commands is the `DrawParameters` that you give |
| `glGetPointerv` | 4.3 | no | Use what SafeGL gives: `Capabilities`, `Limits`, `Context::get_opengl_version` and the getters of each object; the state of draw commands is the `DrawParameters` that you give |
| `glClipControl` | 4.5 | yes | `DrawParameters::clip_control` |
| `glGetGraphicsResetStatus` | 4.5 | yes | `Context::graphics_reset_status`, `Context::is_context_lost` |

## Buffers

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glBindBuffer` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glDeleteBuffers` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGenBuffers` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glIsBuffer` | 1.5 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glBufferData` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBufferSubData` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferSubData` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glMapBuffer` | 1.5 | no | Use `map`, `map_read` or `map_write` on buffers |
| `glUnmapBuffer` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferParameteriv` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferPointerv` | 1.5 | no | Use `get_size` on buffers, and the mappings that `map` returns |
| `glBindBufferRange` | 3.0 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBindBufferBase` | 3.0 | no | Give the buffer to the block of a uniform of a draw command (`uniform!`): SafeGL binds it |
| `glMapBufferRange` | 3.0 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glFlushMappedBufferRange` | 3.0 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glCopyBufferSubData` | 3.1 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferParameteri64v` | 3.2 | no | Use `get_size` on buffers, and the mappings that `map` returns |
| `glClearBufferData` | 4.3 | no | Use `clear()` on the buffer |
| `glClearBufferSubData` | 4.3 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glInvalidateBufferSubData` | 4.3 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glInvalidateBufferData` | 4.3 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBufferStorage` | 4.4 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBindBuffersBase` | 4.4 | no | Give the buffers to the blocks of the uniforms of a draw command (`uniform!`): SafeGL binds each one |
| `glBindBuffersRange` | 4.4 | no | Give the buffers to the blocks of the uniforms of a draw command (`uniform!`): SafeGL binds each one |
| `glCreateBuffers` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glNamedBufferStorage` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glNamedBufferData` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glNamedBufferSubData` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glCopyNamedBufferSubData` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glClearNamedBufferData` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glClearNamedBufferSubData` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glMapNamedBuffer` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glMapNamedBufferRange` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glUnmapNamedBuffer` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glFlushMappedNamedBufferRange` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glGetNamedBufferParameteriv` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glGetNamedBufferParameteri64v` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glGetNamedBufferPointerv` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |
| `glGetNamedBufferSubData` | 4.5 | no | Use `Buffer` and its slices: `write`, `read`, `map`, `map_read`, `map_write`, `copy_to`, `clear`, `invalidate`, `get_size` |

## Textures and samplers

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glTexParameterf` | 1.0 | no | Use `uniforms::Sampler` or `SamplerBehavior` for the sampling parameters, border colors included, and the setters of `TextureAny` (`set_swizzle`, `set_mipmap_range`, `set_depth_stencil_mode`) |
| `glTexParameterfv` | 1.0 | no | Use `uniforms::Sampler` or `SamplerBehavior` for the sampling parameters, border colors included, and the setters of `TextureAny` (`set_swizzle`, `set_mipmap_range`, `set_depth_stencil_mode`) |
| `glTexParameteri` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexParameteriv` | 1.0 | no | Use `uniforms::Sampler` or `SamplerBehavior` for the sampling parameters, border colors included, and the setters of `TextureAny` (`set_swizzle`, `set_mipmap_range`, `set_depth_stencil_mode`) |
| `glTexImage1D` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexImage2D` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glPixelStoref` | 1.0 | no | Use `uniforms::Sampler` or `SamplerBehavior` for the sampling parameters, border colors included, and the setters of `TextureAny` (`set_swizzle`, `set_mipmap_range`, `set_depth_stencil_mode`) |
| `glPixelStorei` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetTexImage` | 1.0 | no | Use `read` on textures, `read_box` and `read_volume` on their mipmap levels, and `read_compressed_box` for compressed textures |
| `glGetTexParameterfv` | 1.0 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGetTexParameteriv` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetTexLevelParameterfv` | 1.0 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGetTexLevelParameteriv` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCopyTexImage1D` | 1.1 | no | Use `blit_color`, `blit_whole_color_to` or `fill` from a framebuffer to the surface of a texture (`as_surface()`), or `texture::copy_image` between textures |
| `glCopyTexImage2D` | 1.1 | no | Use `blit_color`, `blit_whole_color_to` or `fill` from a framebuffer to the surface of a texture (`as_surface()`), or `texture::copy_image` between textures |
| `glCopyTexSubImage1D` | 1.1 | no | Use `blit_color`, `blit_whole_color_to` or `fill` from a framebuffer to the surface of a texture (`as_surface()`), or `texture::copy_image` between textures |
| `glCopyTexSubImage2D` | 1.1 | no | Use `blit_color`, `blit_whole_color_to` or `fill` from a framebuffer to the surface of a texture (`as_surface()`), or `texture::copy_image` between textures |
| `glTexSubImage1D` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexSubImage2D` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glBindTexture` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glDeleteTextures` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGenTextures` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glIsTexture` | 1.1 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glTexImage3D` | 1.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexSubImage3D` | 1.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCopyTexSubImage3D` | 1.2 | no | Use `blit_color`, `blit_whole_color_to` or `fill` from a framebuffer to the surface of a texture (`as_surface()`), or `texture::copy_image` between textures |
| `glActiveTexture` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexImage3D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexImage2D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexImage1D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexSubImage3D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexSubImage2D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexSubImage1D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetCompressedTexImage` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexParameterIiv` | 3.0 | no | Use `uniforms::Sampler` or `SamplerBehavior` for the sampling parameters, border colors included, and the setters of `TextureAny` (`set_swizzle`, `set_mipmap_range`, `set_depth_stencil_mode`) |
| `glTexParameterIuiv` | 3.0 | no | Use `uniforms::Sampler` or `SamplerBehavior` for the sampling parameters, border colors included, and the setters of `TextureAny` (`set_swizzle`, `set_mipmap_range`, `set_depth_stencil_mode`) |
| `glGetTexParameterIiv` | 3.0 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGetTexParameterIuiv` | 3.0 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGenerateMipmap` | 3.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexBuffer` | 3.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexImage2DMultisample` | 3.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexImage3DMultisample` | 3.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGenSamplers` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glDeleteSamplers` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glIsSampler` | 3.3 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glBindSampler` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameteri` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameteriv` | 3.3 | no | Use `uniforms::Sampler` or `SamplerBehavior` for the sampling parameters, border colors included, and the setters of `TextureAny` (`set_swizzle`, `set_mipmap_range`, `set_depth_stencil_mode`) |
| `glSamplerParameterf` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameterfv` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameterIiv` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameterIuiv` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glGetSamplerParameteriv` | 3.3 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGetSamplerParameterIiv` | 3.3 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGetSamplerParameterfv` | 3.3 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGetSamplerParameterIuiv` | 3.3 | no | Use the getters of `TextureAny` (`swizzle`, `mipmap_range`, `depth_stencil_mode`, dimensions and format), and the `SamplerBehavior` that you give |
| `glGetInternalformativ` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glBindImageTexture` | 4.2 | yes | `uniforms::ImageUnit`: images of shaders |
| `glTexStorage1D` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage2D` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage3D` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCopyImageSubData` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetInternalformati64v` | 4.3 | no | Use `Context::format_info` |
| `glInvalidateTexSubImage` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glInvalidateTexImage` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexBufferRange` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage2DMultisample` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage3DMultisample` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTextureView` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glClearTexImage` | 4.4 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glClearTexSubImage` | 4.4 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glBindTextures` | 4.4 | no | Give the textures and samplers to the uniforms of a draw command (`uniform!`), arrays of samplers included: SafeGL binds each one |
| `glBindSamplers` | 4.4 | no | Give the textures and samplers to the uniforms of a draw command (`uniform!`), arrays of samplers included: SafeGL binds each one |
| `glBindImageTextures` | 4.4 | no | Give `ImageUnit`s to the image uniforms of a draw command: SafeGL binds each one |
| `glCreateTextures` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureBuffer` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureBufferRange` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureStorage1D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureStorage2D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureStorage3D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureStorage2DMultisample` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureStorage3DMultisample` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureSubImage1D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureSubImage2D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureSubImage3D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glCompressedTextureSubImage1D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glCompressedTextureSubImage2D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glCompressedTextureSubImage3D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glCopyTextureSubImage1D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glCopyTextureSubImage2D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glCopyTextureSubImage3D` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureParameterf` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureParameterfv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureParameteri` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTextureParameterIiv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureParameterIuiv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glTextureParameteriv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGenerateTextureMipmap` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glBindTextureUnit` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGetTextureImage` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGetCompressedTextureImage` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGetTextureLevelParameterfv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGetTextureLevelParameteriv` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetTextureParameterfv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGetTextureParameterIiv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGetTextureParameterIuiv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glGetTextureParameteriv` | 4.5 | no | Use the texture types (`Texture2d`, ...): their constructors, `write`, `read`, `read_box`, `generate_mipmaps`, `set_swizzle` and the other setters and getters of `TextureAny`, and `as_surface()` for drawing and blits |
| `glCreateSamplers` | 4.5 | no | Use `uniforms::Sampler` or a `SamplerBehavior` with the texture given to a uniform |
| `glGetTextureSubImage` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetCompressedTextureSubImage` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetnCompressedTexImage` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetnTexImage` | 4.5 | no | Use `read` on textures, `read_box` and `read_volume` on their mipmap levels, and `read_compressed_box` for compressed textures |
| `glTextureBarrier` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |

## Framebuffers

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glDrawBuffer` | 1.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClear` | 1.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearColor` | 1.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearStencil` | 1.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearDepth` | 1.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glReadBuffer` | 1.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glReadPixels` | 1.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glDrawBuffers` | 2.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearBufferiv` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearBufferuiv` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearBufferfv` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearBufferfi` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glIsRenderbuffer` | 3.0 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glBindRenderbuffer` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glDeleteRenderbuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGenRenderbuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glRenderbufferStorage` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGetRenderbufferParameteriv` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glIsFramebuffer` | 3.0 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glBindFramebuffer` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glDeleteFramebuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGenFramebuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glCheckFramebufferStatus` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glFramebufferTexture1D` | 3.0 | no | Use one-dimensional textures as attachments of the framebuffer types |
| `glFramebufferTexture2D` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glFramebufferTexture3D` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glFramebufferRenderbuffer` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGetFramebufferAttachmentParameteriv` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glBlitFramebuffer` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glRenderbufferStorageMultisample` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glFramebufferTextureLayer` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glFramebufferTexture` | 3.2 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glClearDepthf` | 4.1 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glFramebufferParameteri` | 4.3 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGetFramebufferParameteriv` | 4.3 | no | Use `get_dimensions` on framebuffers, and the dimensions given to `EmptyFrameBuffer::new` |
| `glInvalidateFramebuffer` | 4.3 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glInvalidateSubFramebuffer` | 4.3 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glCreateFramebuffers` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferRenderbuffer` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferParameteri` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glNamedFramebufferTexture` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferTextureLayer` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferDrawBuffer` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glNamedFramebufferDrawBuffers` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferReadBuffer` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glInvalidateNamedFramebufferData` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glInvalidateNamedFramebufferSubData` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glClearNamedFramebufferiv` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glClearNamedFramebufferuiv` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glClearNamedFramebufferfv` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glClearNamedFramebufferfi` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glBlitNamedFramebuffer` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glCheckNamedFramebufferStatus` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGetNamedFramebufferParameteriv` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glGetNamedFramebufferAttachmentParameteriv` | 4.5 | no | Use the framebuffer types (`SimpleFrameBuffer`, `MultiOutputFrameBuffer`, `LayeredFrameBuffer`, `EmptyFrameBuffer`, `DefaultFramebuffer`) and `Surface`: clears, blits, `read`, `invalidate`, `get_dimensions` |
| `glCreateRenderbuffers` | 4.5 | no | Use the render buffer types of `framebuffer` (`RenderBuffer`, `DepthRenderBuffer`, ...): their constructors and `get_dimensions` |
| `glNamedRenderbufferStorage` | 4.5 | no | Use the render buffer types of `framebuffer` (`RenderBuffer`, `DepthRenderBuffer`, ...): their constructors and `get_dimensions` |
| `glNamedRenderbufferStorageMultisample` | 4.5 | no | Use the render buffer types of `framebuffer` (`RenderBuffer`, `DepthRenderBuffer`, ...): their constructors and `get_dimensions` |
| `glGetNamedRenderbufferParameteriv` | 4.5 | no | Use the render buffer types of `framebuffer` (`RenderBuffer`, `DepthRenderBuffer`, ...): their constructors and `get_dimensions` |
| `glReadnPixels` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |

## Shaders and programs

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glAttachShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glBindAttribLocation` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCompileShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCreateProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCreateShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDeleteProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDeleteShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDetachShader` | 2.0 | no | Not needed: SafeGL creates the shaders of each program and deletes them once it is linked |
| `glGetActiveAttrib` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveUniform` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetAttachedShaders` | 2.0 | no | Not needed: SafeGL creates the shaders of each program and deletes them once it is linked |
| `glGetAttribLocation` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramiv` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramInfoLog` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetShaderiv` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetShaderInfoLog` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetShaderSource` | 2.0 | no | Not needed: SafeGL creates the shaders of each program and deletes them once it is linked |
| `glGetUniformLocation` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glIsProgram` | 2.0 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glIsShader` | 2.0 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glLinkProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glShaderSource` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glUseProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glValidateProgram` | 2.0 | no | Not needed: draw commands return a `DrawError` for what validation would report, and `ProgramPipeline::new` validates pipelines |
| `glBindFragDataLocation` | 3.0 | no | Use `program::LinkLocations::fragment_outputs` with the index 0 |
| `glGetFragDataLocation` | 3.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveUniformsiv` | 3.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveUniformName` | 3.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveUniformBlockiv` | 3.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveUniformBlockName` | 3.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glBindFragDataLocationIndexed` | 3.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetFragDataIndex` | 3.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetSubroutineUniformLocation` | 4.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveSubroutineUniformiv` | 4.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveSubroutineUniformName` | 4.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveSubroutineName` | 4.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramStageiv` | 4.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glShaderBinary` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetShaderPrecisionFormat` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramBinary` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glProgramBinary` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glProgramParameteri` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glUseProgramStages` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glActiveShaderProgram` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline` |
| `glCreateShaderProgramv` | 4.1 | no | Use `program::SeparableProgram::from_source` |
| `glBindProgramPipeline` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDeleteProgramPipelines` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGenProgramPipelines` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glIsProgramPipeline` | 4.1 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glGetProgramPipelineiv` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glValidateProgramPipeline` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramPipelineInfoLog` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveAtomicCounterBufferiv` | 4.2 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramInterfaceiv` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramResourceIndex` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramResourceName` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramResourceiv` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glShaderStorageBlockBinding` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCreateProgramPipelines` | 4.5 | no | Use `program::ProgramPipeline::new` |
| `glSpecializeShader` | 4.6 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |

## Uniforms

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glGetUniformfv` | 2.0 | no | Not needed: uniforms have the values that draw commands give them, or keep with `uniforms::Keep` |
| `glGetUniformiv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform1f` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2f` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform3f` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform4f` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform1i` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2i` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform3i` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform4i` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform1fv` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform2fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform3fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform4fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform1iv` | 2.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform2iv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform3iv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform4iv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix2fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix3fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix4fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix2x3fv` | 2.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix3x2fv` | 2.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix2x4fv` | 2.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix4x2fv` | 2.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix3x4fv` | 2.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix4x3fv` | 2.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glGetUniformuiv` | 3.0 | no | Not needed: uniforms have the values that draw commands give them, or keep with `uniforms::Keep` |
| `glUniform1ui` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2ui` | 3.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform3ui` | 3.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform4ui` | 3.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform1uiv` | 3.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform2uiv` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform3uiv` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform4uiv` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glGetUniformIndices` | 3.1 | no | Use the reflection of programs: `Program::get_uniform`, `uniforms`, `get_uniform_blocks`, `get_shader_storage_blocks`, `get_subroutine_uniforms`, `attributes`, `get_frag_data_location` |
| `glGetUniformBlockIndex` | 3.1 | no | Use the reflection of programs: `Program::get_uniform`, `uniforms`, `get_uniform_blocks`, `get_shader_storage_blocks`, `get_subroutine_uniforms`, `attributes`, `get_frag_data_location` |
| `glUniformBlockBinding` | 3.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform1d` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2d` | 4.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform3d` | 4.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform4d` | 4.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform1dv` | 4.0 | no | Use `uniform!` with a value of the type of the uniform: SafeGL sets it with another form of the command of the same type |
| `glUniform2dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform3dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform4dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix2dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix3dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix4dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix2x3dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix2x4dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix3x2dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix3x4dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix4x2dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniformMatrix4x3dv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glGetUniformdv` | 4.0 | no | Not needed: uniforms have the values that draw commands give them, or keep with `uniforms::Keep` |
| `glGetSubroutineIndex` | 4.0 | no | Use the reflection of programs: `Program::get_uniform`, `uniforms`, `get_uniform_blocks`, `get_shader_storage_blocks`, `get_subroutine_uniforms`, `attributes`, `get_frag_data_location` |
| `glUniformSubroutinesuiv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glGetUniformSubroutineuiv` | 4.0 | no | Not needed: draw commands give the subroutine uniforms (`UniformValue::Subroutine`) |
| `glGetQueryIndexediv` | 4.0 | no | Use the reflection of programs: `Program::get_uniform`, `uniforms`, `get_uniform_blocks`, `get_shader_storage_blocks`, `get_subroutine_uniforms`, `attributes`, `get_frag_data_location` |
| `glProgramUniform1i` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform1iv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform1f` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform1fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform1d` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform1dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform1ui` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform1uiv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2i` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2iv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2f` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2d` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2ui` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform2uiv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3i` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3iv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3f` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3d` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3ui` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform3uiv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4i` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4iv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4f` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4d` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4ui` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniform4uiv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix2fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix3fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix4fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix2dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix3dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix4dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix2x3fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix3x2fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix2x4fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix4x2fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix3x4fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix4x3fv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix2x3dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix3x2dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix2x4dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix4x2dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix3x4dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glProgramUniformMatrix4x3dv` | 4.1 | no | Use `uniform!` with a draw command whose program is a `ProgramPipeline`: the values are given to every program of the pipeline that has the uniform |
| `glGetProgramResourceLocation` | 4.3 | no | Use the reflection of programs: `Program::get_uniform`, `uniforms`, `get_uniform_blocks`, `get_shader_storage_blocks`, `get_subroutine_uniforms`, `attributes`, `get_frag_data_location` |
| `glGetProgramResourceLocationIndex` | 4.3 | no | Use the reflection of programs: `Program::get_uniform`, `uniforms`, `get_uniform_blocks`, `get_shader_storage_blocks`, `get_subroutine_uniforms`, `attributes`, `get_frag_data_location` |
| `glGetnUniformdv` | 4.5 | no | Not needed: uniforms have the values that draw commands give them, or keep with `uniforms::Keep` |
| `glGetnUniformfv` | 4.5 | no | Not needed: uniforms have the values that draw commands give them, or keep with `uniforms::Keep` |
| `glGetnUniformiv` | 4.5 | no | Not needed: uniforms have the values that draw commands give them, or keep with `uniforms::Keep` |
| `glGetnUniformuiv` | 4.5 | no | Not needed: uniforms have the values that draw commands give them, or keep with `uniforms::Keep` |

## Vertex input

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glDisableVertexAttribArray` | 2.0 | no | Not needed: SafeGL enables the attribute arrays that the program reads, and leaves the others disabled |
| `glEnableVertexAttribArray` | 2.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGetVertexAttribdv` | 2.0 | no | Not needed: the attributes are the vertex sources given to the draw command, and `Program::attributes` lists the ones of the program |
| `glGetVertexAttribfv` | 2.0 | no | Not needed: the attributes are the vertex sources given to the draw command, and `Program::attributes` lists the ones of the program |
| `glGetVertexAttribiv` | 2.0 | no | Not needed: the attributes are the vertex sources given to the draw command, and `Program::attributes` lists the ones of the program |
| `glGetVertexAttribPointerv` | 2.0 | no | Not needed: the attributes are the vertex sources given to the draw command, and `Program::attributes` lists the ones of the program |
| `glVertexAttrib1d` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib1dv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib1f` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib1fv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib1s` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib1sv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib2d` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib2dv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib2f` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib2fv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib2s` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib2sv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib3d` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib3dv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib3f` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib3fv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib3s` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib3sv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4Nbv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4Niv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4Nsv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4Nub` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4Nubv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4Nuiv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4Nusv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4bv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4d` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4dv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4f` | 2.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttrib4fv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4iv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4s` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4sv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4ubv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4uiv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttrib4usv` | 2.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribPointer` | 2.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribIPointer` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGetVertexAttribIiv` | 3.0 | no | Not needed: the attributes are the vertex sources given to the draw command, and `Program::attributes` lists the ones of the program |
| `glGetVertexAttribIuiv` | 3.0 | no | Not needed: the attributes are the vertex sources given to the draw command, and `Program::attributes` lists the ones of the program |
| `glVertexAttribI1i` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI2i` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI3i` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4i` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribI1ui` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI2ui` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI3ui` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4ui` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribI1iv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI2iv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI3iv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4iv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI1uiv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI2uiv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI3uiv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4uiv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4bv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4sv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4ubv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribI4usv` | 3.0 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glBindVertexArray` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glDeleteVertexArrays` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGenVertexArrays` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glIsVertexArray` | 3.0 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glPrimitiveRestartIndex` | 3.1 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribDivisor` | 3.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribP1ui` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribP1uiv` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribP2ui` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribP2uiv` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribP3ui` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribP3uiv` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribP4ui` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribP4uiv` | 3.3 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribL1d` | 4.1 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribL2d` | 4.1 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribL3d` | 4.1 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribL4d` | 4.1 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribL1dv` | 4.1 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribL2dv` | 4.1 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribL3dv` | 4.1 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribL4dv` | 4.1 | no | Use `vertex::ConstantAttribute`, which sets all four components: give the values of the shorter, normalized and packed forms as their four-component floats, integers or doubles |
| `glVertexAttribLPointer` | 4.1 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGetVertexAttribLdv` | 4.1 | no | Not needed: the attributes are the vertex sources given to the draw command, and `Program::attributes` lists the ones of the program |
| `glBindVertexBuffer` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribFormat` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribIFormat` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribLFormat` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribBinding` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexBindingDivisor` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glBindVertexBuffers` | 4.4 | no | Give several vertex buffers to `Surface::draw`, as a tuple or a slice of vertex sources: SafeGL binds each one |
| `glCreateVertexArrays` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glDisableVertexArrayAttrib` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glEnableVertexArrayAttrib` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayElementBuffer` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayVertexBuffer` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayVertexBuffers` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayAttribBinding` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayAttribFormat` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayAttribIFormat` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayAttribLFormat` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glVertexArrayBindingDivisor` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glGetVertexArrayiv` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glGetVertexArrayIndexediv` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |
| `glGetVertexArrayIndexed64iv` | 4.5 | no | Use vertex buffers of `implement_vertex!` types, `InstanceRate` and index buffers given to `Surface::draw`: SafeGL builds and caches the vertex array objects |

## Drawing

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glDrawArrays` | 1.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElements` | 1.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawRangeElements` | 1.2 | no | Use `Surface::draw` with an `IndexBuffer` or a slice of it: the range is only a hint that doesn't change what is drawn |
| `glMultiDrawArrays` | 1.4 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElements` | 1.4 | no | Use `IndexBuffer::ranges` with `Surface::draw` |
| `glDrawArraysInstanced` | 3.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsInstanced` | 3.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsBaseVertex` | 3.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawRangeElementsBaseVertex` | 3.2 | no | Use `Surface::draw` with an `IndexBuffer` or a slice of it: the range is only a hint that doesn't change what is drawn |
| `glDrawElementsInstancedBaseVertex` | 3.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElementsBaseVertex` | 3.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawArraysIndirect` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsIndirect` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedback` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedbackStream` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawArraysInstancedBaseInstance` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsInstancedBaseInstance` | 4.2 | no | Use `DrawParameters::base_instance` with an instanced draw of an index buffer |
| `glDrawElementsInstancedBaseVertexBaseInstance` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedbackInstanced` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedbackStreamInstanced` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawArraysIndirect` | 4.3 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElementsIndirect` | 4.3 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawArraysIndirectCount` | 4.6 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElementsIndirectCount` | 4.6 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |

## Rasterization and fragments

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glCullFace` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glFrontFace` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glHint` | 1.0 | yes | `DrawParameters::smooth`: the quality of smoothed lines and polygons |
| `glLineWidth` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPointSize` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPolygonMode` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glScissor` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glStencilMask` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glColorMask` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glDepthMask` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glBlendFunc` | 1.0 | no | Use `DrawParameters::blend`, or `DrawParameters::outputs` for each output, with the same factors for the color and the alpha |
| `glLogicOp` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glStencilFunc` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glStencilOp` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glDepthFunc` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glDepthRange` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glViewport` | 1.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPolygonOffset` | 1.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glSampleCoverage` | 1.3 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glBlendFuncSeparate` | 1.4 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPointParameterf` | 1.4 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPointParameterfv` | 1.4 | no | Use `DrawParameters::point_fade_threshold` and `DrawParameters::point_coord_origin` |
| `glPointParameteri` | 1.4 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPointParameteriv` | 1.4 | no | Use `DrawParameters::point_fade_threshold` and `DrawParameters::point_coord_origin` |
| `glBlendColor` | 1.4 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glBlendEquation` | 1.4 | no | Use `DrawParameters::blend`, or `DrawParameters::outputs` for each output, with the same factors for the color and the alpha |
| `glBlendEquationSeparate` | 2.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glStencilOpSeparate` | 2.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glStencilFuncSeparate` | 2.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glStencilMaskSeparate` | 2.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glColorMaski` | 3.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glClampColor` | 3.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glProvokingVertex` | 3.2 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glGetMultisamplefv` | 3.2 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glSampleMaski` | 3.2 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glMinSampleShading` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glBlendEquationi` | 4.0 | no | Use `DrawParameters::blend`, or `DrawParameters::outputs` for each output, with the same factors for the color and the alpha |
| `glBlendEquationSeparatei` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glBlendFunci` | 4.0 | no | Use `DrawParameters::blend`, or `DrawParameters::outputs` for each output, with the same factors for the color and the alpha |
| `glBlendFuncSeparatei` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPatchParameteri` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPatchParameterfv` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glDepthRangef` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glViewportArrayv` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glViewportIndexedf` | 4.1 | no | Use `DrawParameters::viewports`, with one `ViewportState` per index |
| `glViewportIndexedfv` | 4.1 | no | Use `DrawParameters::viewports`, with one `ViewportState` per index |
| `glScissorArrayv` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glScissorIndexed` | 4.1 | no | Use `DrawParameters::viewports`, with one `ViewportState` per index |
| `glScissorIndexedv` | 4.1 | no | Use `DrawParameters::viewports`, with one `ViewportState` per index |
| `glDepthRangeArrayv` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glDepthRangeIndexed` | 4.1 | no | Use `DrawParameters::viewports`, with one `ViewportState` per index |
| `glMemoryBarrier` | 4.2 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glMemoryBarrierByRegion` | 4.5 | no | Not needed: SafeGL adds the memory barriers that draw commands need |
| `glPolygonOffsetClamp` | 4.6 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |

## Transform feedback

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glBeginTransformFeedback` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glEndTransformFeedback` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glTransformFeedbackVaryings` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glGetTransformFeedbackVarying` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glBindTransformFeedback` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glDeleteTransformFeedbacks` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glGenTransformFeedbacks` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glIsTransformFeedback` | 4.0 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glPauseTransformFeedback` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glResumeTransformFeedback` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glCreateTransformFeedbacks` | 4.5 | no | Use `vertex::TransformFeedback::new` and `with_buffer`, given to `DrawParameters::transform_feedback` |
| `glTransformFeedbackBufferBase` | 4.5 | no | Use `vertex::TransformFeedback::new` and `with_buffer`, given to `DrawParameters::transform_feedback` |
| `glTransformFeedbackBufferRange` | 4.5 | no | Use `vertex::TransformFeedback::new` and `with_buffer`, given to `DrawParameters::transform_feedback` |
| `glGetTransformFeedbackiv` | 4.5 | no | Use `vertex::TransformFeedback::new` and `with_buffer`, given to `DrawParameters::transform_feedback` |
| `glGetTransformFeedbacki_v` | 4.5 | no | Use `vertex::TransformFeedback::new` and `with_buffer`, given to `DrawParameters::transform_feedback` |
| `glGetTransformFeedbacki64_v` | 4.5 | no | Use `vertex::TransformFeedback::new` and `with_buffer`, given to `DrawParameters::transform_feedback` |

## Queries

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glGenQueries` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glDeleteQueries` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glIsQuery` | 1.5 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glBeginQuery` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glEndQuery` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glGetQueryiv` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glGetQueryObjectiv` | 1.5 | no | Use `get()` on queries, or `to_buffer` with a buffer of signed integers |
| `glGetQueryObjectuiv` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glBeginConditionalRender` | 3.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glEndConditionalRender` | 3.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glQueryCounter` | 3.3 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glGetQueryObjecti64v` | 3.3 | no | Use `get()` on queries, or `to_buffer` with a buffer of signed integers |
| `glGetQueryObjectui64v` | 3.3 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glBeginQueryIndexed` | 4.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glEndQueryIndexed` | 4.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glCreateQueries` | 4.5 | no | Use the query types (`SamplesPassedQuery`, `TimeElapsedQuery`, ...): `get()`, `is_ready()` and `to_buffer` |
| `glGetQueryBufferObjecti64v` | 4.5 | no | Use the query types (`SamplesPassedQuery`, `TimeElapsedQuery`, ...): `get()`, `is_ready()` and `to_buffer` |
| `glGetQueryBufferObjectiv` | 4.5 | no | Use the query types (`SamplesPassedQuery`, `TimeElapsedQuery`, ...): `get()`, `is_ready()` and `to_buffer` |
| `glGetQueryBufferObjectui64v` | 4.5 | no | Use the query types (`SamplesPassedQuery`, `TimeElapsedQuery`, ...): `get()`, `is_ready()` and `to_buffer` |
| `glGetQueryBufferObjectuiv` | 4.5 | no | Use the query types (`SamplesPassedQuery`, `TimeElapsedQuery`, ...): `get()`, `is_ready()` and `to_buffer` |

## Synchronization

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glFenceSync` | 3.2 | yes | `SyncFence` |
| `glIsSync` | 3.2 | no | Not needed: the Rust types of SafeGL (`Buffer`, `Texture2d`, `Program`, ...) tell what each object is |
| `glDeleteSync` | 3.2 | yes | `SyncFence` |
| `glClientWaitSync` | 3.2 | yes | `SyncFence` |
| `glWaitSync` | 3.2 | yes | `SyncFence` |
| `glGetSynciv` | 3.2 | no | Use `SyncFence::is_signaled` |

## Debug output

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glDebugMessageControl` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glDebugMessageInsert` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glDebugMessageCallback` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glGetDebugMessageLog` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glPushDebugGroup` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glPopDebugGroup` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glObjectLabel` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glGetObjectLabel` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glObjectPtrLabel` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |
| `glGetObjectPtrLabel` | 4.3 | yes | `debug` module: debug callback and log, `Context::push_debug_group`, `debug::ObjectLabel`, `insert_debug_message` |

## Compute

| Command | Since | Called by SafeGL | How to use it in SafeGL |
|---------|-------|--------|--------|
| `glDispatchCompute` | 4.3 | yes | `program::ComputeShader::execute`, `execute_indirect` |
| `glDispatchComputeIndirect` | 4.3 | yes | `program::ComputeShader::execute`, `execute_indirect` |

## Functionality of shaders

The functionality of OpenGL 4.0 to 4.6 that only exists in GLSL needs no API: the tests of the repository compile shaders that use it, when the context has the version of GLSL, and check what they compute.

| Functionality | Since | Test |
|---------------|-------|------|
| Shader group vote (`anyInvocation`, `allInvocations`, `allInvocationsEqual`) | 4.6 | `glsl_coverage::shader_group_vote` |
| Atomic counter operations (`atomicCounterAdd`, `atomicCounterSubtract`, `atomicCounterMax`, ...) | 4.6 | `glsl_coverage::atomic_counter_operations` |
| Draw parameters (`gl_DrawID`, `gl_BaseVertex`, `gl_BaseInstance`) | 4.6 | `multi_draw` |
| Derivative control (`dFdxFine`, `dFdyCoarse`, `fwidthFine`, ...) | 4.5 | `glsl_coverage::derivative_control` |
| `gl_HelperInvocation` | 4.5 | `glsl_coverage::helper_invocations` |
| Texture queries (`textureQueryLevels`, `textureSamples`, `textureSize`) | 4.3, 4.5 | `glsl_coverage::texture_queries` |
| `gl_CullDistance` | 4.5 | `rasterization` |
| Explicit uniform locations | 4.3 | `glsl_coverage::explicit_uniform_locations` |
| Arrays of arrays | 4.3 | `uniform_arrays::arrays_of_arrays` |
| `layout(component)` and enhanced layouts | 4.4 | `attribute_components`, `transform_feedback` |
| `imageAtomicExchange` on `r32f` images | 4.2 | `glsl_coverage::atomic_exchange_on_float_images` |
| Conservative depth (`layout(depth_greater)`) | 4.2 | `glsl_coverage::conservative_depth` |
| Early fragment tests (`layout(early_fragment_tests)`) | 4.2 | `glsl_coverage::early_fragment_tests` |
| `textureGatherOffsets` | 4.0 | `glsl_coverage::gather_with_offsets` |
| Interpolation functions (`interpolateAtCentroid`, `interpolateAtSample`, `interpolateAtOffset`) | 4.0 | `glsl_coverage::interpolation_functions` |
| Sample built-ins (`gl_SampleID`, `gl_SamplePosition`, `gl_SampleMaskIn`) | 4.0 | `glsl_coverage::sample_built_ins` |
| Instanced geometry shaders (`invocations`) | 4.0 | `glsl_coverage::instanced_geometry_shaders` |
| Double precision (`double`, `dvec`, `dmat`) | 4.0 | `glsl_coverage::double_precision`, `uniforms` |
| Subroutines | 4.0 | `subroutines`, `program_pipeline` |
| Tessellation and geometry stages, vertex streams | 4.0 | `tessellation`, `geometry_shader`, `transform_feedback` |
