//! OpenGL constants, type aliases, and the small table of functions that `glow` doesn't provide.
//!
//! Almost every OpenGL call goes through `glow`. This module re-exports glow's constants, so
//! `gl::FOO` keeps working, adds the handful of constants glow lacks, and loads the remaining
//! entry points (bindless textures, SPIR-V, multi-draw indirect, program introspection, ...)
//! through the backend's proc-address loader. Entries can be dropped once glow covers them.

#![allow(non_snake_case, non_upper_case_globals, dead_code, clippy::all)]

use std::mem;
use std::os::raw::c_void;
use std::ptr;

pub use glow::*;

/// OpenGL scalar type aliases.
pub mod types {
    use std::os::raw;

    pub type GLbitfield = raw::c_uint;
    pub type GLboolean = raw::c_uchar;
    pub type GLchar = raw::c_char;
    pub type GLclampd = raw::c_double;
    pub type GLclampf = raw::c_float;
    pub type GLdouble = raw::c_double;
    pub type GLenum = raw::c_uint;
    pub type GLfloat = raw::c_float;
    pub type GLint = raw::c_int;
    pub type GLint64 = i64;
    pub type GLintptr = isize;
    pub type GLsizei = raw::c_int;
    pub type GLsizeiptr = isize;
    pub type GLubyte = raw::c_uchar;
    pub type GLuint = raw::c_uint;
    pub type GLuint64 = u64;

    /// Opaque sync object.
    pub enum __GLsync {}
    pub type GLsync = *const __GLsync;

    pub type GLDEBUGPROC = Option<
        extern "system" fn(
            source: GLenum,
            gltype: GLenum,
            id: GLuint,
            severity: GLenum,
            length: GLsizei,
            message: *const GLchar,
            userParam: *mut raw::c_void,
        ),
    >;
    pub type GLDEBUGPROCARB = GLDEBUGPROC;
    pub type GLDEBUGPROCKHR = GLDEBUGPROC;
}

use self::types::*;

// Constants that glow doesn't define.
pub const CONTEXT_ROBUST_ACCESS: GLenum = 0x90F3;
pub const DEPTH_CLAMP_FAR_AMD: GLenum = 0x901F;
pub const DEPTH_CLAMP_NEAR_AMD: GLenum = 0x901E;
pub const GPU_MEMORY_INFO_CURRENT_AVAILABLE_VIDMEM_NVX: GLenum = 0x9049;
pub const INT64_NV: GLenum = 0x140E;
// GL_EXT_texture_sRGB_R8 and GL_EXT_texture_sRGB_RG8
pub const SR8_EXT: GLenum = 0x8FBD;
pub const SRG8_EXT: GLenum = 0x8FBE;
// GL_OVR_multiview
pub const MAX_VIEWS_OVR: GLenum = 0x9631;
// WebGL 2
pub const MAX_CLIENT_WAIT_TIMEOUT_WEBGL: GLenum = 0x9247;
pub const TEXTURE_FREE_MEMORY_ATI: GLenum = 0x87FC;
pub const UNSIGNED_INT64_NV: GLenum = 0x140F;
// the view classes of the ETC2 and EAC formats
pub const VIEW_CLASS_EAC_R11: GLenum = 0x9383;
pub const VIEW_CLASS_EAC_RG11: GLenum = 0x9384;
pub const VIEW_CLASS_ETC2_RGB: GLenum = 0x9385;
pub const VIEW_CLASS_ETC2_RGBA: GLenum = 0x9386;
pub const VIEW_CLASS_ETC2_EAC_RGBA: GLenum = 0x9387;
// GL_ARB_gpu_shader_int64
pub const INT64_ARB: GLenum = 0x140E;
pub const INT64_VEC2_ARB: GLenum = 0x8FE9;
pub const INT64_VEC3_ARB: GLenum = 0x8FEA;
pub const INT64_VEC4_ARB: GLenum = 0x8FEB;
pub const UNSIGNED_INT64_ARB: GLenum = 0x140F;
pub const UNSIGNED_INT64_VEC2_ARB: GLenum = 0x8FF5;
pub const UNSIGNED_INT64_VEC3_ARB: GLenum = 0x8FF6;
pub const UNSIGNED_INT64_VEC4_ARB: GLenum = 0x8FF7;
// GL_NV_gpu_shader5
pub const FLOAT16_NV: GLenum = 0x8FF8;
pub const FLOAT16_VEC2_NV: GLenum = 0x8FF9;
pub const FLOAT16_VEC3_NV: GLenum = 0x8FFA;
pub const FLOAT16_VEC4_NV: GLenum = 0x8FFB;

/// Some drivers return small sentinel values instead of null for unsupported functions.
fn is_valid_function_pointer(ptr: *const c_void) -> bool {
    !matches!(ptr as isize, -1 | 0 | 1 | 2 | 3)
}

#[cold]
#[inline(never)]
fn function_not_loaded(name: &str) -> ! {
    panic!("OpenGL function {} was called but is not supported by the driver", name)
}

macro_rules! extra_functions {
    ($(fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty;)+) => {
        /// OpenGL entry points that `glow` doesn't expose, loaded through the backend.
        ///
        /// On WebGL, nothing is loaded: these functions are only reached on code paths that
        /// check for desktop OpenGL or OpenGL ES versions and extensions WebGL doesn't have.
        pub struct Extra {
            $($name: *const c_void,)+
        }

        impl Extra {
            /// Loads every function with `loader`. Unsupported functions are left unloaded and
            /// panic if called.
            pub fn load<F>(mut loader: F) -> Extra
            where
                F: FnMut(&str) -> *const c_void,
            {
                Extra {
                    $($name: {
                        let ptr = loader(concat!("gl", stringify!($name)));
                        if is_valid_function_pointer(ptr) { ptr } else { ptr::null() }
                    },)+
                }
            }

            /// A table where no function is loaded.
            pub fn unloaded() -> Extra {
                Extra {
                    $($name: ptr::null(),)+
                }
            }

            $(
                #[inline]
                pub unsafe fn $name(&self, $($arg: $ty),*) -> $ret {
                    if self.$name.is_null() {
                        function_not_loaded(stringify!($name));
                    }
                    mem::transmute::<*const c_void, extern "system" fn($($ty),*) -> $ret>(
                        self.$name,
                    )($($arg),*)
                }
            )+
        }
    };
}

extra_functions! {
    fn BeginConditionalRender(id: GLuint, mode: GLenum) -> ();
    fn BeginConditionalRenderNV(id: GLuint, mode: GLenum) -> ();
    fn BindProgramPipeline(pipeline: GLuint) -> ();
    fn BindFragDataLocationIndexed(program: GLuint, colorNumber: GLuint, index: GLuint, name: *const GLchar) -> ();
    fn BindFragDataLocationIndexedEXT(program: GLuint, colorNumber: GLuint, index: GLuint, name: *const GLchar) -> ();
    fn ClampColor(target: GLenum, clamp: GLenum) -> ();
    fn ClientWaitSync(sync: GLsync, flags: GLbitfield, timeout: GLuint64) -> GLenum;
    fn ClearTexImage(texture: GLuint, level: GLint, format: GLenum, type_: GLenum, data: *const c_void) -> ();
    fn ClearTexSubImage(texture: GLuint, level: GLint, xoffset: GLint, yoffset: GLint, zoffset: GLint, width: GLsizei, height: GLsizei, depth: GLsizei, format: GLenum, type_: GLenum, data: *const c_void) -> ();
    fn ClearTexImageEXT(texture: GLuint, level: GLint, format: GLenum, type_: GLenum, data: *const c_void) -> ();
    fn ClearTexSubImageEXT(texture: GLuint, level: GLint, xoffset: GLint, yoffset: GLint, zoffset: GLint, width: GLsizei, height: GLsizei, depth: GLsizei, format: GLenum, type_: GLenum, data: *const c_void) -> ();
    fn ClipControl(origin: GLenum, depth: GLenum) -> ();
    fn ClipControlEXT(origin: GLenum, depth: GLenum) -> ();
    fn CopyImageSubData(srcName: GLuint, srcTarget: GLenum, srcLevel: GLint, srcX: GLint, srcY: GLint, srcZ: GLint, dstName: GLuint, dstTarget: GLenum, dstLevel: GLint, dstX: GLint, dstY: GLint, dstZ: GLint, srcWidth: GLsizei, srcHeight: GLsizei, srcDepth: GLsizei) -> ();
    fn CopyImageSubDataEXT(srcName: GLuint, srcTarget: GLenum, srcLevel: GLint, srcX: GLint, srcY: GLint, srcZ: GLint, dstName: GLuint, dstTarget: GLenum, dstLevel: GLint, dstX: GLint, dstY: GLint, dstZ: GLint, srcWidth: GLsizei, srcHeight: GLsizei, srcDepth: GLsizei) -> ();
    fn CopyImageSubDataOES(srcName: GLuint, srcTarget: GLenum, srcLevel: GLint, srcX: GLint, srcY: GLint, srcZ: GLint, dstName: GLuint, dstTarget: GLenum, dstLevel: GLint, dstX: GLint, dstY: GLint, dstZ: GLint, srcWidth: GLsizei, srcHeight: GLsizei, srcDepth: GLsizei) -> ();
    fn CompressedTexSubImage1D(target: GLenum, level: GLint, xoffset: GLint, width: GLsizei, format: GLenum, imageSize: GLsizei, data: *const c_void) -> ();
    fn DebugMessageCallback(callback: GLDEBUGPROC, userParam: *const c_void) -> ();
    fn DebugMessageCallbackARB(callback: GLDEBUGPROCARB, userParam: *const c_void) -> ();
    fn DebugMessageCallbackKHR(callback: GLDEBUGPROCKHR, userParam: *const c_void) -> ();
    fn DebugMessageControl(source: GLenum, type_: GLenum, severity: GLenum, count: GLsizei, ids: *const GLuint, enabled: GLboolean) -> ();
    fn DebugMessageControlARB(source: GLenum, type_: GLenum, severity: GLenum, count: GLsizei, ids: *const GLuint, enabled: GLboolean) -> ();
    fn DebugMessageControlKHR(source: GLenum, type_: GLenum, severity: GLenum, count: GLsizei, ids: *const GLuint, enabled: GLboolean) -> ();
    fn DrawElementsBaseVertexOES(mode: GLenum, count: GLsizei, type_: GLenum, indices: *const c_void, basevertex: GLint) -> ();
    fn DrawElementsInstancedBaseVertexOES(mode: GLenum, count: GLsizei, type_: GLenum, indices: *const c_void, instancecount: GLsizei, basevertex: GLint) -> ();
    fn DrawArraysInstancedBaseInstanceEXT(mode: GLenum, first: GLint, count: GLsizei, instancecount: GLsizei, baseinstance: GLuint) -> ();
    fn DrawElementsInstancedBaseInstanceEXT(mode: GLenum, count: GLsizei, type_: GLenum, indices: *const c_void, instancecount: GLsizei, baseinstance: GLuint) -> ();
    fn DrawElementsInstancedBaseVertexBaseInstanceEXT(mode: GLenum, count: GLsizei, type_: GLenum, indices: *const c_void, instancecount: GLsizei, basevertex: GLint, baseinstance: GLuint) -> ();
    fn EndConditionalRender() -> ();
    fn EnableiEXT(target: GLenum, index: GLuint) -> ();
    fn DisableiEXT(target: GLenum, index: GLuint) -> ();
    fn BlendEquationSeparateiEXT(buf: GLuint, modeRGB: GLenum, modeAlpha: GLenum) -> ();
    fn BlendFuncSeparateiEXT(buf: GLuint, srcRGB: GLenum, dstRGB: GLenum, srcAlpha: GLenum, dstAlpha: GLenum) -> ();
    fn ColorMaskiEXT(index: GLuint, r: GLboolean, g: GLboolean, b: GLboolean, a: GLboolean) -> ();
    fn EnableiOES(target: GLenum, index: GLuint) -> ();
    fn DisableiOES(target: GLenum, index: GLuint) -> ();
    fn BlendEquationSeparateiOES(buf: GLuint, modeRGB: GLenum, modeAlpha: GLenum) -> ();
    fn BlendFuncSeparateiOES(buf: GLuint, srcRGB: GLenum, dstRGB: GLenum, srcAlpha: GLenum, dstAlpha: GLenum) -> ();
    fn ColorMaskiOES(index: GLuint, r: GLboolean, g: GLboolean, b: GLboolean, a: GLboolean) -> ();
    fn EndConditionalRenderNV() -> ();
    fn FramebufferParameteri(target: GLenum, pname: GLenum, param: GLint) -> ();
    fn FramebufferTextureEXT(target: GLenum, attachment: GLenum, texture: GLuint, level: GLint) -> ();
    fn FramebufferTextureMultiviewOVR(target: GLenum, attachment: GLenum, texture: GLuint, level: GLint, baseViewIndex: GLint, numViews: GLsizei) -> ();
    fn GetActiveAtomicCounterBufferiv(program: GLuint, bufferIndex: GLuint, pname: GLenum, params: *mut GLint) -> ();
    fn GetActiveSubroutineName(program: GLuint, shadertype: GLenum, index: GLuint, bufsize: GLsizei, length: *mut GLsizei, name: *mut GLchar) -> ();
    fn GetActiveSubroutineUniformName(program: GLuint, shadertype: GLenum, index: GLuint, bufsize: GLsizei, length: *mut GLsizei, name: *mut GLchar) -> ();
    fn GetActiveSubroutineUniformiv(program: GLuint, shadertype: GLenum, index: GLuint, pname: GLenum, values: *mut GLint) -> ();
    fn GetActiveUniformName(program: GLuint, uniformIndex: GLuint, bufSize: GLsizei, length: *mut GLsizei, uniformName: *mut GLchar) -> ();
    fn GetDebugMessageLog(count: GLuint, bufSize: GLsizei, sources: *mut GLenum, types: *mut GLenum, ids: *mut GLuint, severities: *mut GLenum, lengths: *mut GLsizei, messageLog: *mut GLchar) -> GLuint;
    fn GetDebugMessageLogKHR(count: GLuint, bufSize: GLsizei, sources: *mut GLenum, types: *mut GLenum, ids: *mut GLuint, severities: *mut GLenum, lengths: *mut GLsizei, messageLog: *mut GLchar) -> GLuint;
    fn GetCompressedTexImage(target: GLenum, level: GLint, img: *mut c_void) -> ();
    fn GetCompressedTextureSubImage(texture: GLuint, level: GLint, xoffset: GLint, yoffset: GLint, zoffset: GLint, width: GLsizei, height: GLsizei, depth: GLsizei, bufSize: GLsizei, pixels: *mut c_void) -> ();
    fn GetTextureSubImage(texture: GLuint, level: GLint, xoffset: GLint, yoffset: GLint, zoffset: GLint, width: GLsizei, height: GLsizei, depth: GLsizei, format: GLenum, type_: GLenum, bufSize: GLsizei, pixels: *mut c_void) -> ();
    fn GetnCompressedTexImage(target: GLenum, lod: GLint, bufSize: GLsizei, pixels: *mut c_void) -> ();
    fn GetnCompressedTexImageARB(target: GLenum, lod: GLint, bufSize: GLsizei, img: *mut c_void) -> ();
    fn GetInternalformati64v(target: GLenum, internalformat: GLenum, pname: GLenum, count: GLsizei, params: *mut GLint64) -> ();
    fn GetnTexImageARB(target: GLenum, level: GLint, format: GLenum, type_: GLenum, bufSize: GLsizei, img: *mut c_void) -> ();
    fn GetFragDataIndex(program: GLuint, name: *const GLchar) -> GLint;
    fn GetFragDataIndexEXT(program: GLuint, name: *const GLchar) -> GLint;
    fn GetGraphicsResetStatus() -> GLenum;
    fn GetGraphicsResetStatusARB() -> GLenum;
    fn GetGraphicsResetStatusEXT() -> GLenum;
    fn GetGraphicsResetStatusKHR() -> GLenum;
    fn GetMultisamplefv(pname: GLenum, index: GLuint, val: *mut GLfloat) -> ();
    fn GetObjectLabel(identifier: GLenum, name: GLuint, bufSize: GLsizei, length: *mut GLsizei, label: *mut GLchar) -> ();
    fn GetObjectLabelKHR(identifier: GLenum, name: GLuint, bufSize: GLsizei, length: *mut GLsizei, label: *mut GLchar) -> ();
    fn GetObjectPtrLabel(ptr: *const c_void, bufSize: GLsizei, length: *mut GLsizei, label: *mut GLchar) -> ();
    fn GetObjectPtrLabelKHR(ptr: *const c_void, bufSize: GLsizei, length: *mut GLsizei, label: *mut GLchar) -> ();
    fn DeleteProgramPipelines(n: GLsizei, pipelines: *const GLuint) -> ();
    fn GenProgramPipelines(n: GLsizei, pipelines: *mut GLuint) -> ();
    fn GetProgramPipelineInfoLog(pipeline: GLuint, bufSize: GLsizei, length: *mut GLsizei, infoLog: *mut GLchar) -> ();
    fn GetProgramPipelineiv(pipeline: GLuint, pname: GLenum, params: *mut GLint) -> ();
    fn GetProgramiv(program: GLuint, pname: GLenum, params: *mut GLint) -> ();
    fn GetProgramInterfaceiv(program: GLuint, programInterface: GLenum, pname: GLenum, params: *mut GLint) -> ();
    fn GetProgramResourceIndex(program: GLuint, programInterface: GLenum, name: *const GLchar) -> GLuint;
    fn GetProgramResourceName(program: GLuint, programInterface: GLenum, index: GLuint, bufSize: GLsizei, length: *mut GLsizei, name: *mut GLchar) -> ();
    fn GetProgramResourceiv(program: GLuint, programInterface: GLenum, index: GLuint, propCount: GLsizei, props: *const GLenum, bufSize: GLsizei, length: *mut GLsizei, params: *mut GLint) -> ();
    fn GetProgramStageiv(program: GLuint, shadertype: GLenum, pname: GLenum, values: *mut GLint) -> ();
    fn GetQueryObjectiv(id: GLuint, pname: GLenum, params: *mut GLint) -> ();
    fn GetQueryObjecti64v(id: GLuint, pname: GLenum, params: *mut GLint64) -> ();
    fn GetQueryObjectuiv(id: GLuint, pname: GLenum, params: *mut GLuint) -> ();
    fn GetQueryObjectui64v(id: GLuint, pname: GLenum, params: *mut GLuint64) -> ();
    fn GetQueryiv(target: GLenum, pname: GLenum, params: *mut GLint) -> ();
    fn GetSubroutineUniformLocation(program: GLuint, shadertype: GLenum, name: *const GLchar) -> GLint;
    fn GetTextureHandleARB(texture: GLuint) -> GLuint64;
    fn GetTextureSamplerHandleARB(texture: GLuint, sampler: GLuint) -> GLuint64;
    fn UniformHandleui64ARB(location: GLint, value: GLuint64) -> ();
    fn InsertEventMarkerEXT(length: GLsizei, marker: *const GLchar) -> ();
    fn InvalidateBufferData(buffer: GLuint) -> ();
    fn InvalidateBufferSubData(buffer: GLuint, offset: GLintptr, length: GLsizeiptr) -> ();
    fn InvalidateTexImage(texture: GLuint, level: GLint) -> ();
    fn InvalidateTexSubImage(texture: GLuint, level: GLint, xoffset: GLint, yoffset: GLint, zoffset: GLint, width: GLsizei, height: GLsizei, depth: GLsizei) -> ();
    fn LogicOp(opcode: GLenum) -> ();
    fn BeginQueryIndexed(target: GLenum, index: GLuint, id: GLuint) -> ();
    fn EndQueryIndexed(target: GLenum, index: GLuint) -> ();
    fn ClearBufferSubData(target: GLenum, internalformat: GLenum, offset: GLintptr, size: GLsizeiptr, format: GLenum, type_: GLenum, data: *const c_void) -> ();
    fn ClearNamedBufferSubData(buffer: GLuint, internalformat: GLenum, offset: GLintptr, size: GLsizeiptr, format: GLenum, type_: GLenum, data: *const c_void) -> ();
    fn MakeImageHandleResidentARB(handle: GLuint64, access: GLenum) -> ();
    fn MakeTextureHandleNonResidentARB(handle: GLuint64) -> ();
    fn MakeTextureHandleResidentARB(handle: GLuint64) -> ();
    fn MinSampleShading(value: GLfloat) -> ();
    fn MinSampleShadingOES(value: GLfloat) -> ();
    fn MultiDrawArraysIndirect(mode: GLenum, indirect: *const c_void, drawcount: GLsizei, stride: GLsizei) -> ();
    fn MultiDrawElementsIndirect(mode: GLenum, type_: GLenum, indirect: *const c_void, drawcount: GLsizei, stride: GLsizei) -> ();
    fn MultiDrawArraysIndirectEXT(mode: GLenum, indirect: *const c_void, drawcount: GLsizei, stride: GLsizei) -> ();
    fn MultiDrawElementsIndirectEXT(mode: GLenum, type_: GLenum, indirect: *const c_void, drawcount: GLsizei, stride: GLsizei) -> ();
    fn MultiDrawArraysIndirectCount(mode: GLenum, indirect: *const c_void, drawcount: GLintptr, maxdrawcount: GLsizei, stride: GLsizei) -> ();
    fn MultiDrawElementsIndirectCount(mode: GLenum, type_: GLenum, indirect: *const c_void, drawcount: GLintptr, maxdrawcount: GLsizei, stride: GLsizei) -> ();
    fn MultiDrawArraysIndirectCountARB(mode: GLenum, indirect: *const c_void, drawcount: GLintptr, maxdrawcount: GLsizei, stride: GLsizei) -> ();
    fn MultiDrawElementsIndirectCountARB(mode: GLenum, type_: GLenum, indirect: *const c_void, drawcount: GLintptr, maxdrawcount: GLsizei, stride: GLsizei) -> ();
    fn PointSize(size: GLfloat) -> ();
    fn PointParameterf(pname: GLenum, param: GLfloat) -> ();
    fn PointParameteri(pname: GLenum, param: GLint) -> ();
    fn PolygonOffsetClamp(factor: GLfloat, units: GLfloat, clamp: GLfloat) -> ();
    fn PolygonOffsetClampEXT(factor: GLfloat, units: GLfloat, clamp: GLfloat) -> ();
    fn PatchParameteriEXT(pname: GLenum, value: GLint) -> ();
    fn PatchParameteriOES(pname: GLenum, value: GLint) -> ();
    fn PrimitiveBoundingBox(minX: GLfloat, minY: GLfloat, minZ: GLfloat, minW: GLfloat, maxX: GLfloat, maxY: GLfloat, maxZ: GLfloat, maxW: GLfloat) -> ();
    fn PrimitiveBoundingBoxARB(minX: GLfloat, minY: GLfloat, minZ: GLfloat, minW: GLfloat, maxX: GLfloat, maxY: GLfloat, maxZ: GLfloat, maxW: GLfloat) -> ();
    fn PrimitiveBoundingBoxEXT(minX: GLfloat, minY: GLfloat, minZ: GLfloat, minW: GLfloat, maxX: GLfloat, maxY: GLfloat, maxZ: GLfloat, maxW: GLfloat) -> ();
    fn PrimitiveBoundingBoxOES(minX: GLfloat, minY: GLfloat, minZ: GLfloat, minW: GLfloat, maxX: GLfloat, maxY: GLfloat, maxZ: GLfloat, maxW: GLfloat) -> ();
    fn VertexAttribL4d(index: GLuint, x: GLdouble, y: GLdouble, z: GLdouble, w: GLdouble) -> ();
    fn MultiDrawArrays(mode: GLenum, first: *const GLint, count: *const GLsizei, drawcount: GLsizei) -> ();
    fn MultiDrawElementsBaseVertex(mode: GLenum, count: *const GLsizei, type_: GLenum, indices: *const *const c_void, drawcount: GLsizei, basevertex: *const GLint) -> ();
    fn DrawTransformFeedback(mode: GLenum, id: GLuint) -> ();
    fn DrawTransformFeedbackInstanced(mode: GLenum, id: GLuint, instancecount: GLsizei) -> ();
    fn DrawTransformFeedbackStream(mode: GLenum, id: GLuint, stream: GLuint) -> ();
    fn DrawTransformFeedbackStreamInstanced(mode: GLenum, id: GLuint, stream: GLuint, instancecount: GLsizei) -> ();
    fn PatchParameterfv(pname: GLenum, values: *const GLfloat) -> ();
    fn PrimitiveRestartIndex(index: GLuint) -> ();
    fn UseProgramStages(pipeline: GLuint, stages: GLbitfield, program: GLuint) -> ();
    fn ValidateProgramPipeline(pipeline: GLuint) -> ();
    fn ProgramParameteri(program: GLuint, pname: GLenum, value: GLint) -> ();
    fn ProvokingVertex(mode: GLenum) -> ();
    fn ProvokingVertexEXT(mode: GLenum) -> ();
    fn ReadnPixels(x: GLint, y: GLint, width: GLsizei, height: GLsizei, format: GLenum, type_: GLenum, bufSize: GLsizei, data: *mut c_void) -> ();
    fn ReadnPixelsARB(x: GLint, y: GLint, width: GLsizei, height: GLsizei, format: GLenum, type_: GLenum, bufSize: GLsizei, data: *mut c_void) -> ();
    fn ReadnPixelsEXT(x: GLint, y: GLint, width: GLsizei, height: GLsizei, format: GLenum, type_: GLenum, bufSize: GLsizei, data: *mut c_void) -> ();
    fn ReadnPixelsKHR(x: GLint, y: GLint, width: GLsizei, height: GLsizei, format: GLenum, type_: GLenum, bufSize: GLsizei, data: *mut c_void) -> ();
    fn ReleaseShaderCompiler() -> ();
    fn SampleMaski(maskNumber: GLuint, mask: GLbitfield) -> ();
    fn SamplerParameterIiv(sampler: GLuint, pname: GLenum, param: *const GLint) -> ();
    fn SamplerParameterIivEXT(sampler: GLuint, pname: GLenum, param: *const GLint) -> ();
    fn SamplerParameterIivOES(sampler: GLuint, pname: GLenum, param: *const GLint) -> ();
    fn SamplerParameterIuiv(sampler: GLuint, pname: GLenum, param: *const GLuint) -> ();
    fn SamplerParameterIuivEXT(sampler: GLuint, pname: GLenum, param: *const GLuint) -> ();
    fn SamplerParameterIuivOES(sampler: GLuint, pname: GLenum, param: *const GLuint) -> ();
    fn ShaderBinary(count: GLsizei, shaders: *const GLuint, binaryformat: GLenum, binary: *const c_void, length: GLsizei) -> ();
    fn SpecializeShader(shader: GLuint, pEntryPoint: *const GLchar, numSpecializationConstants: GLuint, pConstantIndex: *const GLuint, pConstantValue: *const GLuint) -> ();
    fn SpecializeShaderARB(shader: GLuint, pEntryPoint: *const GLchar, numSpecializationConstants: GLuint, pConstantIndex: *const GLuint, pConstantValue: *const GLuint) -> ();
    fn TexBufferEXT(target: GLenum, internalformat: GLenum, buffer: GLuint) -> ();
    fn TexBufferRange(target: GLenum, internalformat: GLenum, buffer: GLuint, offset: GLintptr, size: GLsizeiptr) -> ();
    fn TexBufferRangeEXT(target: GLenum, internalformat: GLenum, buffer: GLuint, offset: GLintptr, size: GLsizeiptr) -> ();
    fn TexBufferRangeOES(target: GLenum, internalformat: GLenum, buffer: GLuint, offset: GLintptr, size: GLsizeiptr) -> ();
    fn TexBufferOES(target: GLenum, internalformat: GLenum, buffer: GLuint) -> ();
    fn TexImage3DMultisample(target: GLenum, samples: GLsizei, internalformat: GLenum, width: GLsizei, height: GLsizei, depth: GLsizei, fixedsamplelocations: GLboolean) -> ();
    fn TexStorage3DMultisample(target: GLenum, samples: GLsizei, internalformat: GLenum, width: GLsizei, height: GLsizei, depth: GLsizei, fixedsamplelocations: GLboolean) -> ();
    fn TexStorage3DMultisampleOES(target: GLenum, samples: GLsizei, internalformat: GLenum, width: GLsizei, height: GLsizei, depth: GLsizei, fixedsamplelocations: GLboolean) -> ();
    fn TexSubImage1D(target: GLenum, level: GLint, xoffset: GLint, width: GLsizei, format: GLenum, type_: GLenum, pixels: *const c_void) -> ();
    fn TextureBarrier() -> ();
    fn TextureView(texture: GLuint, target: GLenum, origtexture: GLuint, internalformat: GLenum, minlevel: GLuint, numlevels: GLuint, minlayer: GLuint, numlayers: GLuint) -> ();
    fn TextureBarrierNV() -> ();
    fn Uniform1d(location: GLint, x: GLdouble) -> ();
    fn Uniform1i64ARB(location: GLint, x: GLint64) -> ();
    fn Uniform1ui64ARB(location: GLint, x: GLuint64) -> ();
    fn Uniform2dv(location: GLint, count: GLsizei, value: *const GLdouble) -> ();
    fn Uniform2i64vARB(location: GLint, count: GLsizei, value: *const GLint64) -> ();
    fn Uniform2ui64vARB(location: GLint, count: GLsizei, value: *const GLuint64) -> ();
    fn Uniform3dv(location: GLint, count: GLsizei, value: *const GLdouble) -> ();
    fn Uniform3i64vARB(location: GLint, count: GLsizei, value: *const GLint64) -> ();
    fn Uniform3ui64vARB(location: GLint, count: GLsizei, value: *const GLuint64) -> ();
    fn Uniform4dv(location: GLint, count: GLsizei, value: *const GLdouble) -> ();
    fn Uniform4i64vARB(location: GLint, count: GLsizei, value: *const GLint64) -> ();
    fn Uniform4ui64vARB(location: GLint, count: GLsizei, value: *const GLuint64) -> ();
    fn UniformMatrix2dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix3dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix4dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix2x3dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix2x4dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix3x2dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix3x4dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix4x2dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformMatrix4x3dv(location: GLint, count: GLsizei, transpose: GLboolean, value: *const GLdouble) -> ();
    fn UniformSubroutinesuiv(shadertype: GLenum, count: GLsizei, indices: *const GLuint) -> ();
}
