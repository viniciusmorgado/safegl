# OpenGL 4.6 coverage

This chapter maps every command of the OpenGL 4.6 core profile, 658 of them, to SafeGL. SafeGL covers the functionality of OpenGL safely rather than its commands one by one:

- 334 commands are called by SafeGL, which gives their functionality through the API that each row names;
- 324 commands aren't called, because another command gives the same result. These are another form of the same command (scalar or vector, signed or unsigned, single or array), the direct state access and multi-bind forms of commands that SafeGL calls on bound objects, and queries of state that SafeGL sets itself.

The level that compiles each API is in the table of levels of the README, and the functionality that only exists in shaders (GLSL 4.60) is tested by the `glsl_coverage` tests of the repository.

## Context, state and errors

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glDisable` | 1.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glEnable` | 1.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glFinish` | 1.0 | yes | `Context::finish`, `Context::flush` |
| `glFlush` | 1.0 | yes | `Context::finish`, `Context::flush` |
| `glGetBooleanv` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetDoublev` | 1.0 | no | State queries: SafeGL tracks the state it sets (`GlState`), and reads the limits once (`Capabilities`, `Limits`) |
| `glGetError` | 1.0 | yes | `Context::assert_no_error`, and the checks after linking programs |
| `glGetFloatv` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetIntegerv` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetString` | 1.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glIsEnabled` | 1.0 | no | State queries: SafeGL tracks the state it sets (`GlState`) and sets it again after `restore_state` |
| `glGetPointerv` | 4.3 | no | State queries: SafeGL tracks the state it sets (`GlState`), and reads the limits once (`Capabilities`, `Limits`) |
| `glGetBooleani_v` | 3.0 | no | State queries: SafeGL tracks the state it sets (`GlState`), and reads the limits once (`Capabilities`, `Limits`) |
| `glGetIntegeri_v` | 3.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glEnablei` | 3.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glDisablei` | 3.0 | yes | The state of `DrawParameters`, synchronized for each draw command |
| `glIsEnabledi` | 3.0 | no | State queries: SafeGL tracks the state it sets (`GlState`) and sets it again after `restore_state` |
| `glGetStringi` | 3.0 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetInteger64v` | 3.2 | yes | Context creation: the version, extensions, `Capabilities` and `Limits` |
| `glGetInteger64i_v` | 3.2 | no | State queries: SafeGL tracks the state it sets (`GlState`), and reads the limits once (`Capabilities`, `Limits`) |
| `glReleaseShaderCompiler` | 4.1 | yes | `Context::release_shader_compiler` |
| `glGetFloati_v` | 4.1 | no | State queries: SafeGL tracks the state it sets (`GlState`), and reads the limits once (`Capabilities`, `Limits`) |
| `glGetDoublei_v` | 4.1 | no | State queries: SafeGL tracks the state it sets (`GlState`), and reads the limits once (`Capabilities`, `Limits`) |
| `glGetPointerv` | 4.3 | no | State queries: SafeGL tracks the state it sets (`GlState`), and reads the limits once (`Capabilities`, `Limits`) |
| `glClipControl` | 4.5 | yes | `DrawParameters::clip_control` |
| `glGetGraphicsResetStatus` | 4.5 | yes | `Context::graphics_reset_status`, `Context::is_context_lost` |

## Buffers

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glBindBuffer` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glDeleteBuffers` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGenBuffers` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glIsBuffer` | 1.5 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glBufferData` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBufferSubData` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferSubData` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glMapBuffer` | 1.5 | no | `glMapBufferRange`, with the access of the mapping |
| `glUnmapBuffer` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferParameteriv` | 1.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferPointerv` | 1.5 | no | Buffer queries: SafeGL creates the buffers with their sizes and keeps their mappings |
| `glBindBufferRange` | 3.0 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBindBufferBase` | 3.0 | no | `glBindBufferRange` over the whole buffer |
| `glMapBufferRange` | 3.0 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glFlushMappedBufferRange` | 3.0 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glCopyBufferSubData` | 3.1 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glGetBufferParameteri64v` | 3.2 | no | Buffer queries: SafeGL creates the buffers with their sizes and keeps their mappings |
| `glClearBufferData` | 4.3 | no | `glClearBufferSubData` over the whole buffer (`clear()` on buffers) |
| `glClearBufferSubData` | 4.3 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glInvalidateBufferSubData` | 4.3 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glInvalidateBufferData` | 4.3 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBufferStorage` | 4.4 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glBindBuffersBase` | 4.4 | no | Multi-bind: SafeGL binds each object with the single-object command, with the same result |
| `glBindBuffersRange` | 4.4 | no | Multi-bind: SafeGL binds each object with the single-object command, with the same result |
| `glCreateBuffers` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glNamedBufferStorage` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glNamedBufferData` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glNamedBufferSubData` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glCopyNamedBufferSubData` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glClearNamedBufferData` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glClearNamedBufferSubData` | 4.5 | yes | `buffer::Buffer` and its slices: creation, `write`, `read`, `map`, `copy_to`, `invalidate`, `clear` |
| `glMapNamedBuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glMapNamedBufferRange` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glUnmapNamedBuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glFlushMappedNamedBufferRange` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetNamedBufferParameteriv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetNamedBufferParameteri64v` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetNamedBufferPointerv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetNamedBufferSubData` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |

## Textures and samplers

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glTexParameterf` | 1.0 | no | Another form of the same parameter; the sampling parameters, border colors included, are set on sampler objects |
| `glTexParameterfv` | 1.0 | no | Another form of the same parameter; the sampling parameters, border colors included, are set on sampler objects |
| `glTexParameteri` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexParameteriv` | 1.0 | no | Another form of the same parameter; the sampling parameters, border colors included, are set on sampler objects |
| `glTexImage1D` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexImage2D` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glPixelStoref` | 1.0 | no | Another form of the same parameter; the sampling parameters, border colors included, are set on sampler objects |
| `glPixelStorei` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetTexImage` | 1.0 | no | Texture reads: through framebuffers (`glReadPixels`, `glReadnPixels`), `glGetTextureSubImage` (`read_box`) and `glGetCompressedTexImage` for compressed images |
| `glGetTexParameterfv` | 1.0 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGetTexParameteriv` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetTexLevelParameterfv` | 1.0 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGetTexLevelParameteriv` | 1.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCopyTexImage1D` | 1.1 | no | Copies from framebuffers into textures: blits from framebuffers to the surfaces of textures (`Surface::blit_*`), and `texture::copy_image` between textures |
| `glCopyTexImage2D` | 1.1 | no | Copies from framebuffers into textures: blits from framebuffers to the surfaces of textures (`Surface::blit_*`), and `texture::copy_image` between textures |
| `glCopyTexSubImage1D` | 1.1 | no | Copies from framebuffers into textures: blits from framebuffers to the surfaces of textures (`Surface::blit_*`), and `texture::copy_image` between textures |
| `glCopyTexSubImage2D` | 1.1 | no | Copies from framebuffers into textures: blits from framebuffers to the surfaces of textures (`Surface::blit_*`), and `texture::copy_image` between textures |
| `glTexSubImage1D` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexSubImage2D` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glBindTexture` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glDeleteTextures` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGenTextures` | 1.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glIsTexture` | 1.1 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glTexImage3D` | 1.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexSubImage3D` | 1.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCopyTexSubImage3D` | 1.2 | no | Copies from framebuffers into textures: blits from framebuffers to the surfaces of textures (`Surface::blit_*`), and `texture::copy_image` between textures |
| `glActiveTexture` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexImage3D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexImage2D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexImage1D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexSubImage3D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexSubImage2D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCompressedTexSubImage1D` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetCompressedTexImage` | 1.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexParameterIiv` | 3.0 | no | Another form of the same parameter; the sampling parameters, border colors included, are set on sampler objects |
| `glTexParameterIuiv` | 3.0 | no | Another form of the same parameter; the sampling parameters, border colors included, are set on sampler objects |
| `glGetTexParameterIiv` | 3.0 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGetTexParameterIuiv` | 3.0 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGenerateMipmap` | 3.0 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexBuffer` | 3.1 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexImage2DMultisample` | 3.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexImage3DMultisample` | 3.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGenSamplers` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glDeleteSamplers` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glIsSampler` | 3.3 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glBindSampler` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameteri` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameteriv` | 3.3 | no | Another form of the same parameter; the sampling parameters, border colors included, are set on sampler objects |
| `glSamplerParameterf` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameterfv` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameterIiv` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glSamplerParameterIuiv` | 3.3 | yes | `uniforms::Sampler`, `SamplerBehavior`: sampler objects bound with the textures of draw commands |
| `glGetSamplerParameteriv` | 3.3 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGetSamplerParameterIiv` | 3.3 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGetSamplerParameterfv` | 3.3 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGetSamplerParameterIuiv` | 3.3 | no | Parameter queries: SafeGL sets the parameters and keeps them (`TextureAny` caches, `SamplerBehavior`) |
| `glGetInternalformativ` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glBindImageTexture` | 4.2 | yes | `uniforms::ImageUnit`: images of shaders |
| `glTexStorage1D` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage2D` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage3D` | 4.2 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glCopyImageSubData` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetInternalformati64v` | 4.3 | no | `glGetInternalformativ` (`Context::format_info`), whose values fit in 32 bits |
| `glInvalidateTexSubImage` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glInvalidateTexImage` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexBufferRange` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage2DMultisample` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTexStorage3DMultisample` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTextureView` | 4.3 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glClearTexImage` | 4.4 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glClearTexSubImage` | 4.4 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glBindTextures` | 4.4 | no | Multi-bind: SafeGL binds each object with the single-object command, with the same result |
| `glBindSamplers` | 4.4 | no | Multi-bind: SafeGL binds each object with the single-object command, with the same result |
| `glBindImageTextures` | 4.4 | no | Multi-bind: SafeGL binds each object with the single-object command, with the same result |
| `glCreateTextures` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureBuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureBufferRange` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureStorage1D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureStorage2D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureStorage3D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureStorage2DMultisample` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureStorage3DMultisample` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureSubImage1D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureSubImage2D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureSubImage3D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCompressedTextureSubImage1D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCompressedTextureSubImage2D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCompressedTextureSubImage3D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCopyTextureSubImage1D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCopyTextureSubImage2D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCopyTextureSubImage3D` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureParameterf` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureParameterfv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureParameteri` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glTextureParameterIiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureParameterIuiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTextureParameteriv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGenerateTextureMipmap` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glBindTextureUnit` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTextureImage` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetCompressedTextureImage` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTextureLevelParameterfv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTextureLevelParameteriv` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetTextureParameterfv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTextureParameterIiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTextureParameterIuiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTextureParameteriv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCreateSamplers` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTextureSubImage` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetCompressedTextureSubImage` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetnCompressedTexImage` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |
| `glGetnTexImage` | 4.5 | no | Texture reads: through framebuffers (`glReadPixels`, `glReadnPixels`), `glGetTextureSubImage` (`read_box`) and `glGetCompressedTexImage` for compressed images |
| `glTextureBarrier` | 4.5 | yes | The `texture` module: texture types, uploads, `read`/`read_box`, mipmaps, `set_swizzle` and the other parameters, buffer textures, views, `invalidate`, `clear`, `copy_image`, `texture_barrier`, `Context::format_info` |

## Framebuffers

| Command | Since | Called | SafeGL |
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
| `glIsRenderbuffer` | 3.0 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glBindRenderbuffer` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glDeleteRenderbuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGenRenderbuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glRenderbufferStorage` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGetRenderbufferParameteriv` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glIsFramebuffer` | 3.0 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glBindFramebuffer` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glDeleteFramebuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGenFramebuffers` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glCheckFramebufferStatus` | 3.0 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glFramebufferTexture1D` | 3.0 | no | One-dimensional textures are attached with `glFramebufferTexture` and `glFramebufferTextureLayer` |
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
| `glGetFramebufferParameteriv` | 4.3 | no | Framebuffer queries: SafeGL sets the default dimensions of framebuffers without attachments and keeps them |
| `glInvalidateFramebuffer` | 4.3 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glInvalidateSubFramebuffer` | 4.3 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glCreateFramebuffers` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferRenderbuffer` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferParameteri` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glNamedFramebufferTexture` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferTextureLayer` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferDrawBuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glNamedFramebufferDrawBuffers` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glNamedFramebufferReadBuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glInvalidateNamedFramebufferData` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glInvalidateNamedFramebufferSubData` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glClearNamedFramebufferiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glClearNamedFramebufferuiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glClearNamedFramebufferfv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glClearNamedFramebufferfi` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glBlitNamedFramebuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCheckNamedFramebufferStatus` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |
| `glGetNamedFramebufferParameteriv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetNamedFramebufferAttachmentParameteriv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glCreateRenderbuffers` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glNamedRenderbufferStorage` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glNamedRenderbufferStorageMultisample` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetNamedRenderbufferParameteriv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glReadnPixels` | 4.5 | yes | The `framebuffer` module: framebuffers and render buffers, clears, `read`, blits, `invalidate`, `DefaultFramebuffer` |

## Shaders and programs

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glAttachShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glBindAttribLocation` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCompileShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCreateProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCreateShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDeleteProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDeleteShader` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDetachShader` | 2.0 | no | The shaders of a program are owned by SafeGL during the link and deleted after it, which detaches them |
| `glGetActiveAttrib` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveUniform` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetAttachedShaders` | 2.0 | no | The shaders of a program are owned by SafeGL during the link and deleted after it, which detaches them |
| `glGetAttribLocation` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramiv` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramInfoLog` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetShaderiv` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetShaderInfoLog` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetShaderSource` | 2.0 | no | The shaders of a program are owned by SafeGL during the link and deleted after it, which detaches them |
| `glGetUniformLocation` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glIsProgram` | 2.0 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glIsShader` | 2.0 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glLinkProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glShaderSource` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glUseProgram` | 2.0 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glValidateProgram` | 2.0 | no | Draw commands check what validation checks and return a `DrawError`; pipelines are validated with `glValidateProgramPipeline` |
| `glBindFragDataLocation` | 3.0 | no | `glBindFragDataLocationIndexed` with the index 0 (`program::LinkLocations`) |
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
| `glActiveShaderProgram` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command |
| `glCreateShaderProgramv` | 4.1 | no | `program::SeparableProgram::from_source` compiles and links a separable program from one shader |
| `glBindProgramPipeline` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glDeleteProgramPipelines` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGenProgramPipelines` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glIsProgramPipeline` | 4.1 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glGetProgramPipelineiv` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glValidateProgramPipeline` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramPipelineInfoLog` | 4.1 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetActiveAtomicCounterBufferiv` | 4.2 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramInterfaceiv` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramResourceIndex` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramResourceName` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glGetProgramResourceiv` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glShaderStorageBlockBinding` | 4.3 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |
| `glCreateProgramPipelines` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glSpecializeShader` | 4.6 | yes | The `program` module: `Program`, `ComputeShader`, `SeparableProgram`, `ProgramPipeline`, binaries, SPIR-V, reflection, `LinkLocations` |

## Uniforms

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glGetUniformfv` | 2.0 | no | Uniform values: every draw command gives them, or keeps them with `uniforms::Keep`, so SafeGL never reads them back |
| `glGetUniformiv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform1f` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2f` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform3f` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform4f` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform1i` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2i` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform3i` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform4i` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform1fv` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform2fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform3fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform4fv` | 2.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform1iv` | 2.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
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
| `glGetUniformuiv` | 3.0 | no | Uniform values: every draw command gives them, or keeps them with `uniforms::Keep`, so SafeGL never reads them back |
| `glUniform1ui` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2ui` | 3.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform3ui` | 3.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform4ui` | 3.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform1uiv` | 3.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform2uiv` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform3uiv` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform4uiv` | 3.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glGetUniformIndices` | 3.1 | no | Reflection: SafeGL lists the resources of programs once they are linked, by index, with the other reflection commands |
| `glGetUniformBlockIndex` | 3.1 | no | Reflection: SafeGL lists the resources of programs once they are linked, by index, with the other reflection commands |
| `glUniformBlockBinding` | 3.1 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform1d` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glUniform2d` | 4.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform3d` | 4.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform4d` | 4.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
| `glUniform1dv` | 4.0 | no | Uniform values: SafeGL sets each type with another form of the command of the same type (`glUniform*v` or the scalar form) |
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
| `glGetUniformdv` | 4.0 | no | Uniform values: every draw command gives them, or keeps them with `uniforms::Keep`, so SafeGL never reads them back |
| `glGetSubroutineIndex` | 4.0 | no | Reflection: SafeGL lists the resources of programs once they are linked, by index, with the other reflection commands |
| `glUniformSubroutinesuiv` | 4.0 | yes | `uniform!`, `uniforms::UniformValue`, uniform blocks and storage blocks given to draw commands |
| `glGetUniformSubroutineuiv` | 4.0 | no | Subroutine uniforms: every draw command gives them (`UniformValue::Subroutine`) |
| `glGetQueryIndexediv` | 4.0 | no | Reflection: SafeGL lists the resources of programs once they are linked, by index, with the other reflection commands |
| `glProgramUniform1i` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform1iv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform1f` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform1fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform1d` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform1dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform1ui` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform1uiv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2i` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2iv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2f` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2d` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2ui` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform2uiv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3i` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3iv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3f` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3d` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3ui` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform3uiv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4i` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4iv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4f` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4d` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4ui` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniform4uiv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix2fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix3fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix4fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix2dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix3dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix4dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix2x3fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix3x2fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix2x4fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix4x2fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix3x4fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix4x3fv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix2x3dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix3x2dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix2x4dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix4x2dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix3x4dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glProgramUniformMatrix4x3dv` | 4.1 | no | Uniforms of the programs of pipelines are set while the program is bound for the draw command (`glUniform*`), which gives the same values |
| `glGetProgramResourceLocation` | 4.3 | no | Reflection: SafeGL lists the resources of programs once they are linked, by index, with the other reflection commands |
| `glGetProgramResourceLocationIndex` | 4.3 | no | Reflection: SafeGL lists the resources of programs once they are linked, by index, with the other reflection commands |
| `glGetnUniformdv` | 4.5 | no | Uniform values: every draw command gives them, or keeps them with `uniforms::Keep`, so SafeGL never reads them back |
| `glGetnUniformfv` | 4.5 | no | Uniform values: every draw command gives them, or keeps them with `uniforms::Keep`, so SafeGL never reads them back |
| `glGetnUniformiv` | 4.5 | no | Uniform values: every draw command gives them, or keeps them with `uniforms::Keep`, so SafeGL never reads them back |
| `glGetnUniformuiv` | 4.5 | no | Uniform values: every draw command gives them, or keeps them with `uniforms::Keep`, so SafeGL never reads them back |

## Vertex input

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glDisableVertexAttribArray` | 2.0 | no | Vertex array objects are built with the arrays that they read enabled, and the other arrays stay disabled |
| `glEnableVertexAttribArray` | 2.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGetVertexAttribdv` | 2.0 | no | Vertex state queries: SafeGL builds the vertex array objects and knows their attributes |
| `glGetVertexAttribfv` | 2.0 | no | Vertex state queries: SafeGL builds the vertex array objects and knows their attributes |
| `glGetVertexAttribiv` | 2.0 | no | Vertex state queries: SafeGL builds the vertex array objects and knows their attributes |
| `glGetVertexAttribPointerv` | 2.0 | no | Vertex state queries: SafeGL builds the vertex array objects and knows their attributes |
| `glVertexAttrib1d` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib1dv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib1f` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib1fv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib1s` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib1sv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib2d` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib2dv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib2f` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib2fv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib2s` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib2sv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib3d` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib3dv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib3f` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib3fv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib3s` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib3sv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4Nbv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4Niv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4Nsv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4Nub` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4Nubv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4Nuiv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4Nusv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4bv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4d` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4dv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4f` | 2.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttrib4fv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4iv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4s` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4sv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4ubv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4uiv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttrib4usv` | 2.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribPointer` | 2.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribIPointer` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGetVertexAttribIiv` | 3.0 | no | Vertex state queries: SafeGL builds the vertex array objects and knows their attributes |
| `glGetVertexAttribIuiv` | 3.0 | no | Vertex state queries: SafeGL builds the vertex array objects and knows their attributes |
| `glVertexAttribI1i` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI2i` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI3i` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4i` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribI1ui` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI2ui` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI3ui` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4ui` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribI1iv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI2iv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI3iv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4iv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI1uiv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI2uiv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI3uiv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4uiv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4bv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4sv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4ubv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribI4usv` | 3.0 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glBindVertexArray` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glDeleteVertexArrays` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGenVertexArrays` | 3.0 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glIsVertexArray` | 3.0 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glPrimitiveRestartIndex` | 3.1 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribDivisor` | 3.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribP1ui` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribP1uiv` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribP2ui` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribP2uiv` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribP3ui` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribP3uiv` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribP4ui` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribP4uiv` | 3.3 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribL1d` | 4.1 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribL2d` | 4.1 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribL3d` | 4.1 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribL4d` | 4.1 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribL1dv` | 4.1 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribL2dv` | 4.1 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribL3dv` | 4.1 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribL4dv` | 4.1 | no | Constant attributes: `vertex::ConstantAttribute` sets the four components (`glVertexAttrib4f`, `glVertexAttribI4i`, `glVertexAttribI4ui`, `glVertexAttribL4d`); the shorter, normalized and packed forms give the same values |
| `glVertexAttribLPointer` | 4.1 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glGetVertexAttribLdv` | 4.1 | no | Vertex state queries: SafeGL builds the vertex array objects and knows their attributes |
| `glBindVertexBuffer` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribFormat` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribIFormat` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribLFormat` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexAttribBinding` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glVertexBindingDivisor` | 4.3 | yes | `VertexBuffer`, `implement_vertex!`, `vertex::ConstantAttribute`, `InstanceRate`, `DrawParameters::primitive_restart` |
| `glBindVertexBuffers` | 4.4 | no | Multi-bind: SafeGL binds each object with the single-object command, with the same result |
| `glCreateVertexArrays` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glDisableVertexArrayAttrib` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glEnableVertexArrayAttrib` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayElementBuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayVertexBuffer` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayVertexBuffers` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayAttribBinding` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayAttribFormat` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayAttribIFormat` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayAttribLFormat` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glVertexArrayBindingDivisor` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetVertexArrayiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetVertexArrayIndexediv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetVertexArrayIndexed64iv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |

## Drawing

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glDrawArrays` | 1.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElements` | 1.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawRangeElements` | 1.2 | no | Hints of the range of the indices: SafeGL draws with `glDrawElements*`, and the hint doesn't change what is drawn |
| `glMultiDrawArrays` | 1.4 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElements` | 1.4 | no | `glMultiDrawElementsBaseVertex` with a base vertex of 0 (`IndexBufferRanges`) |
| `glDrawArraysInstanced` | 3.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsInstanced` | 3.1 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsBaseVertex` | 3.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawRangeElementsBaseVertex` | 3.2 | no | Hints of the range of the indices: SafeGL draws with `glDrawElements*`, and the hint doesn't change what is drawn |
| `glDrawElementsInstancedBaseVertex` | 3.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElementsBaseVertex` | 3.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawArraysIndirect` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsIndirect` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedback` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedbackStream` | 4.0 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawArraysInstancedBaseInstance` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawElementsInstancedBaseInstance` | 4.2 | no | `glDrawElementsInstancedBaseVertexBaseInstance` with a base vertex of 0 |
| `glDrawElementsInstancedBaseVertexBaseInstance` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedbackInstanced` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glDrawTransformFeedbackStreamInstanced` | 4.2 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawArraysIndirect` | 4.3 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElementsIndirect` | 4.3 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawArraysIndirectCount` | 4.6 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |
| `glMultiDrawElementsIndirectCount` | 4.6 | yes | `Surface::draw` with the index sources of the `index` module: lists, ranges, indirect and multidraw commands, counts from buffers, captured vertices |

## Rasterization and fragments

| Command | Since | Called | SafeGL |
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
| `glBlendFunc` | 1.0 | no | The `Separate` form with the same factors and equations (`Blend`, `DrawParameters::outputs`) |
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
| `glPointParameterfv` | 1.4 | no | The scalar forms `glPointParameterf` and `glPointParameteri` (`point_fade_threshold`, `point_coord_origin`) |
| `glPointParameteri` | 1.4 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPointParameteriv` | 1.4 | no | The scalar forms `glPointParameterf` and `glPointParameteri` (`point_fade_threshold`, `point_coord_origin`) |
| `glBlendColor` | 1.4 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glBlendEquation` | 1.4 | no | The `Separate` form with the same factors and equations (`Blend`, `DrawParameters::outputs`) |
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
| `glBlendEquationi` | 4.0 | no | The `Separate` form with the same factors and equations (`Blend`, `DrawParameters::outputs`) |
| `glBlendEquationSeparatei` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glBlendFunci` | 4.0 | no | The `Separate` form with the same factors and equations (`Blend`, `DrawParameters::outputs`) |
| `glBlendFuncSeparatei` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPatchParameteri` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glPatchParameterfv` | 4.0 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glDepthRangef` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glViewportArrayv` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glViewportIndexedf` | 4.1 | no | The array forms `glViewportArrayv`, `glScissorArrayv` and `glDepthRangeArrayv` (`DrawParameters::viewports`) |
| `glViewportIndexedfv` | 4.1 | no | The array forms `glViewportArrayv`, `glScissorArrayv` and `glDepthRangeArrayv` (`DrawParameters::viewports`) |
| `glScissorArrayv` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glScissorIndexed` | 4.1 | no | The array forms `glViewportArrayv`, `glScissorArrayv` and `glDepthRangeArrayv` (`DrawParameters::viewports`) |
| `glScissorIndexedv` | 4.1 | no | The array forms `glViewportArrayv`, `glScissorArrayv` and `glDepthRangeArrayv` (`DrawParameters::viewports`) |
| `glDepthRangeArrayv` | 4.1 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glDepthRangeIndexed` | 4.1 | no | The array forms `glViewportArrayv`, `glScissorArrayv` and `glDepthRangeArrayv` (`DrawParameters::viewports`) |
| `glMemoryBarrier` | 4.2 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |
| `glMemoryBarrierByRegion` | 4.5 | no | `glMemoryBarrier`, which covers the regions too |
| `glPolygonOffsetClamp` | 4.6 | yes | `DrawParameters`: culling, polygon modes and offsets, viewports, scissors, depth, stencil, blending, logic ops, multisampling, points, patches; barriers are added automatically |

## Transform feedback

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glBeginTransformFeedback` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glEndTransformFeedback` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glTransformFeedbackVaryings` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glGetTransformFeedbackVarying` | 3.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glBindTransformFeedback` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glDeleteTransformFeedbacks` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glGenTransformFeedbacks` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glIsTransformFeedback` | 4.0 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glPauseTransformFeedback` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glResumeTransformFeedback` | 4.0 | yes | `vertex::TransformFeedback`, `CapturedVertices` |
| `glCreateTransformFeedbacks` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTransformFeedbackBufferBase` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glTransformFeedbackBufferRange` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTransformFeedbackiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTransformFeedbacki_v` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |
| `glGetTransformFeedbacki64_v` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result |

## Queries

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glGenQueries` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glDeleteQueries` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glIsQuery` | 1.5 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glBeginQuery` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glEndQuery` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glGetQueryiv` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glGetQueryObjectiv` | 1.5 | no | The unsigned forms `glGetQueryObjectuiv` and `glGetQueryObjectui64v`; signed results are written to buffers |
| `glGetQueryObjectuiv` | 1.5 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glBeginConditionalRender` | 3.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glEndConditionalRender` | 3.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glQueryCounter` | 3.3 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glGetQueryObjecti64v` | 3.3 | no | The unsigned forms `glGetQueryObjectuiv` and `glGetQueryObjectui64v`; signed results are written to buffers |
| `glGetQueryObjectui64v` | 3.3 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glBeginQueryIndexed` | 4.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glEndQueryIndexed` | 4.0 | yes | The queries of `draw_parameters`, `debug::TimestampQuery`, `ConditionalRendering` |
| `glCreateQueries` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result; query results are written to buffers with `glGetQueryObject*` and a bound query buffer |
| `glGetQueryBufferObjecti64v` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result; query results are written to buffers with `glGetQueryObject*` and a bound query buffer |
| `glGetQueryBufferObjectiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result; query results are written to buffers with `glGetQueryObject*` and a bound query buffer |
| `glGetQueryBufferObjectui64v` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result; query results are written to buffers with `glGetQueryObject*` and a bound query buffer |
| `glGetQueryBufferObjectuiv` | 4.5 | no | Direct state access: SafeGL binds the object and uses the non-DSA command, with the same result; query results are written to buffers with `glGetQueryObject*` and a bound query buffer |

## Synchronization

| Command | Since | Called | SafeGL |
|---------|-------|--------|--------|
| `glFenceSync` | 3.2 | yes | `SyncFence` |
| `glIsSync` | 3.2 | no | Object type queries: SafeGL creates and owns its objects, whose Rust types tell what they are |
| `glDeleteSync` | 3.2 | yes | `SyncFence` |
| `glClientWaitSync` | 3.2 | yes | `SyncFence` |
| `glWaitSync` | 3.2 | yes | `SyncFence` |
| `glGetSynciv` | 3.2 | no | `glClientWaitSync` with a timeout of 0 tells whether a fence is signaled (`SyncFence::is_signaled`) |

## Debug output

| Command | Since | Called | SafeGL |
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

| Command | Since | Called | SafeGL |
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
