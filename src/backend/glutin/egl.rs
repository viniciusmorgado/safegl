/*!

EGL images and fences, which exchange images and synchronization with other APIs and processes:
camera frames and decoded video as dma-bufs on Linux (`EGL_EXT_image_dma_buf_import`), textures
exported to them (`EGL_KHR_gl_texture_2D_image`, `EGL_MESA_image_dma_buf_export`), and fences
that other APIs wait for or signal (`EGL_KHR_fence_sync`, `EGL_ANDROID_native_fence_sync`).

They need a display whose context is an EGL context: the displays without window of `headless`,
the displays of Android, of GBM surfaces, and of windows created with EGL.

*/

use std::collections::HashSet;
use std::error::Error;
use std::ffi::{c_void, CStr};
use std::fmt;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::rc::Rc;
use std::time::Duration;

use glutin::context::{AsRawContext, PossiblyCurrentContext, RawContext};
use glutin::display::{AsRawDisplay, GetDisplayExtensions, GlDisplay, RawDisplay};
use glutin::display::GetGlDisplay;
use glutin::surface::SurfaceTypeTrait;

use super::Display;
use crate::backend::Facade;
use crate::texture::{
    Dimensions, ExternalTexture, MipmapsOption, Texture2d, TextureCreationError,
    UncompressedFloatFormat,
};
use crate::{gl, CapabilitiesSource, ContextExt, GlObject, TextureExt};

use glow::HasContext;

type EglDisplay = *const c_void;
type EglContext = *const c_void;
type EglImageHandle = *const c_void;
type EglSync = *const c_void;
type EglInt = i32;

const EGL_NONE: EglInt = 0x3038;
const EGL_WIDTH: EglInt = 0x3057;
const EGL_HEIGHT: EglInt = 0x3056;
const EGL_GL_TEXTURE_2D_KHR: u32 = 0x30B1;
const EGL_GL_TEXTURE_LEVEL_KHR: EglInt = 0x30BC;
const EGL_IMAGE_PRESERVED_KHR: EglInt = 0x30D2;
const EGL_TRUE: EglInt = 1;
const EGL_LINUX_DMA_BUF_EXT: u32 = 0x3270;
const EGL_LINUX_DRM_FOURCC_EXT: EglInt = 0x3271;
const EGL_YUV_COLOR_SPACE_HINT_EXT: EglInt = 0x327B;
const EGL_SAMPLE_RANGE_HINT_EXT: EglInt = 0x327C;
const EGL_SYNC_FENCE_KHR: u32 = 0x30F9;
const EGL_SYNC_NATIVE_FENCE_ANDROID: u32 = 0x3144;
const EGL_SYNC_NATIVE_FENCE_FD_ANDROID: EglInt = 0x3145;
const EGL_SYNC_FLUSH_COMMANDS_BIT_KHR: EglInt = 0x0001;
const EGL_TIMEOUT_EXPIRED_KHR: EglInt = 0x30F5;
const EGL_CONDITION_SATISFIED_KHR: EglInt = 0x30F6;
const EGL_FALSE: EglInt = 0;

/// The attributes of the file descriptor, the offset and the pitch of the 4 planes of a dma-buf,
/// then the low and high halves of their modifier.
const PLANE_ATTRIBUTES: [[EglInt; 5]; 4] = [
    [0x3272, 0x3273, 0x3274, 0x3443, 0x3444],
    [0x3275, 0x3276, 0x3277, 0x3445, 0x3446],
    [0x3278, 0x3279, 0x327A, 0x3447, 0x3448],
    [0x3440, 0x3441, 0x3442, 0x3449, 0x344A],
];

/// The functions of the EGL extensions, loaded by `eglGetProcAddress`.
struct EglFunctions {
    get_error: Option<unsafe extern "system" fn() -> EglInt>,
    create_image: Option<
        unsafe extern "system" fn(
            EglDisplay,
            EglContext,
            u32,
            *const c_void,
            *const EglInt,
        ) -> EglImageHandle,
    >,
    destroy_image: Option<unsafe extern "system" fn(EglDisplay, EglImageHandle) -> u32>,
    export_query: Option<
        unsafe extern "system" fn(
            EglDisplay,
            EglImageHandle,
            *mut i32,
            *mut i32,
            *mut u64,
        ) -> u32,
    >,
    export: Option<
        unsafe extern "system" fn(EglDisplay, EglImageHandle, *mut i32, *mut EglInt, *mut EglInt)
            -> u32,
    >,
    create_sync: Option<unsafe extern "system" fn(EglDisplay, u32, *const EglInt) -> EglSync>,
    destroy_sync: Option<unsafe extern "system" fn(EglDisplay, EglSync) -> u32>,
    client_wait_sync: Option<unsafe extern "system" fn(EglDisplay, EglSync, EglInt, u64) -> EglInt>,
    wait_sync: Option<unsafe extern "system" fn(EglDisplay, EglSync, EglInt) -> EglInt>,
    dup_native_fence: Option<unsafe extern "system" fn(EglDisplay, EglSync) -> EglInt>,
}

/// The EGL display of a SafeGL display, and its functions.
struct EglShared {
    display: glutin::api::egl::display::Display,
    raw_display: EglDisplay,
    raw_context: EglContext,
    functions: EglFunctions,
    context: Rc<crate::context::Context>,
}

impl EglShared {
    /// The last EGL error, as `EglError::Egl`.
    fn error(&self) -> EglError {
        let code = self.functions.get_error.map_or(0, |get_error| unsafe { get_error() });
        EglError::Egl(code)
    }
}

/// Error of EGL images and fences.
#[derive(Debug)]
#[non_exhaustive]
pub enum EglError {
    /// The context of the display isn't an EGL context: it uses GLX or WGL.
    NotEgl,
    /// The display doesn't have the EGL extension named.
    ExtensionMissing(&'static str),
    /// A dma-buf has no plane, more than 4, or dimensions or a format of 0.
    InvalidDmaBuf,
    /// The texture doesn't belong to the context of the display.
    ContextMismatch,
    /// EGL returned an error, with its code (`eglGetError`), like `EGL_BAD_MATCH` (0x3009) for a
    /// format or a modifier that the driver doesn't take.
    Egl(i32),
    /// The waiting for a fence failed, or a file descriptor couldn't be duplicated.
    Io(std::io::Error),
}

impl fmt::Display for EglError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EglError::NotEgl => fmt.write_str("The context of the display isn't an EGL context"),
            EglError::ExtensionMissing(name) => write!(fmt, "The display doesn't have {name}"),
            EglError::InvalidDmaBuf => fmt.write_str("The dma-buf has no plane or no size"),
            EglError::ContextMismatch => {
                fmt.write_str("The texture belongs to another context than the display's")
            }
            EglError::Egl(code) => write!(fmt, "EGL error {code:#x}"),
            EglError::Io(error) => write!(fmt, "{error}"),
        }
    }
}

impl Error for EglError {}

/// The YUV color space of a dma-buf, which the GPU converts to RGB with
/// (`EGL_YUV_COLOR_SPACE_HINT_EXT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YuvColorSpace {
    /// ITU-R BT.601, of standard definition video.
    Rec601,
    /// ITU-R BT.709, of high definition video.
    Rec709,
    /// ITU-R BT.2020, of ultra high definition video.
    Rec2020,
}

/// The range of the YUV values of a dma-buf (`EGL_SAMPLE_RANGE_HINT_EXT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YuvRange {
    /// From 0 to 255.
    Full,
    /// From 16 to 235, and to 240 for the chroma, as most video is.
    Narrow,
}

/// A plane of a dma-buf: the file descriptor of its memory, and where the plane is in it.
#[derive(Debug)]
pub struct DmaBufPlane {
    /// The file descriptor of the memory of the plane.
    pub fd: OwnedFd,
    /// Where the plane starts in that memory, in bytes.
    pub offset: u32,
    /// The bytes from a row of the plane to the next.
    pub stride: u32,
}

/// An image in the memory of the Linux kernel, shared between devices and processes: a camera
/// frame (V4L2), a decoded video frame, or the buffer of another API.
#[derive(Debug)]
pub struct DmaBuf {
    /// The width of the image, in pixels.
    pub width: u32,
    /// The height of the image, in pixels.
    pub height: u32,
    /// The DRM format of the image (`DRM_FORMAT_*` of `drm_fourcc.h`), like `AB24` for 8-bit RGBA
    /// or `NV12` for YUV 4:2:0.
    pub fourcc: u32,
    /// The DRM modifier of the layout of the planes, `None` for the layout of the driver, or the
    /// linear layout (`EGL_EXT_image_dma_buf_import_modifiers`).
    pub modifier: Option<u64>,
    /// The planes, from 1 to 4.
    pub planes: Vec<DmaBufPlane>,
    /// The color space of YUV images.
    pub color_space: Option<YuvColorSpace>,
    /// The range of the values of YUV images.
    pub range: Option<YuvRange>,
}

impl DmaBuf {
    /// The DRM format code of four characters, like `b"AB24"`.
    pub const fn fourcc(code: &[u8; 4]) -> u32 {
        u32::from_le_bytes(*code)
    }
}

/// The EGL display of a SafeGL display, which creates EGL images and fences.
#[derive(Clone)]
pub struct Egl(Rc<EglShared>);

impl Egl {
    /// The EGL display of `display`, whose context must be an EGL context.
    pub fn new<T: SurfaceTypeTrait>(display: &Display<T>) -> Result<Egl, EglError> {
        let gl_context = display.gl_context();
        #[allow(irrefutable_let_patterns)]
        let PossiblyCurrentContext::Egl(context) = &*gl_context
        else {
            return Err(EglError::NotEgl);
        };
        let egl_display = context.display();
        #[allow(irrefutable_let_patterns)]
        let RawDisplay::Egl(raw_display) = egl_display.raw_display()
        else {
            return Err(EglError::NotEgl);
        };
        #[allow(irrefutable_let_patterns)]
        let RawContext::Egl(raw_context) = context.raw_context()
        else {
            return Err(EglError::NotEgl);
        };
        let load = |name: &CStr| egl_display.get_proc_address(name);
        // the functions are null when the library doesn't have them
        macro_rules! function {
            ($name:literal) => {{
                let pointer = load($name);
                (!pointer.is_null()).then(|| unsafe { std::mem::transmute(pointer) })
            }};
        }
        let functions = EglFunctions {
            get_error: function!(c"eglGetError"),
            create_image: function!(c"eglCreateImageKHR"),
            destroy_image: function!(c"eglDestroyImageKHR"),
            export_query: function!(c"eglExportDMABUFImageQueryMESA"),
            export: function!(c"eglExportDMABUFImageMESA"),
            create_sync: function!(c"eglCreateSyncKHR"),
            destroy_sync: function!(c"eglDestroySyncKHR"),
            client_wait_sync: function!(c"eglClientWaitSyncKHR"),
            wait_sync: function!(c"eglWaitSyncKHR"),
            dup_native_fence: function!(c"eglDupNativeFenceFDANDROID"),
        };
        Ok(Egl(Rc::new(EglShared {
            display: egl_display,
            raw_display,
            raw_context,
            functions,
            context: display.get_context().clone(),
        })))
    }

    /// The extensions of the EGL display.
    pub fn extensions(&self) -> &HashSet<&'static str> {
        self.0.display.extensions()
    }

    fn require(&self, extension: &'static str) -> Result<(), EglError> {
        if self.extensions().contains(extension) {
            Ok(())
        } else {
            Err(EglError::ExtensionMissing(extension))
        }
    }

    fn image(
        &self,
        context: EglContext,
        target: u32,
        buffer: *const c_void,
        attributes: &[EglInt],
        size: (u32, u32),
    ) -> Result<EglImage, EglError> {
        let create = self
            .0
            .functions
            .create_image
            .ok_or(EglError::ExtensionMissing("EGL_KHR_image_base"))?;
        let image = unsafe {
            create(self.0.raw_display, context, target, buffer, attributes.as_ptr())
        };
        if image.is_null() {
            return Err(self.0.error());
        }
        Ok(EglImage {
            egl: self.clone(),
            image,
            width: size.0,
            height: size.1,
        })
    }

    /// Creates an EGL image of a dma-buf (`EGL_EXT_image_dma_buf_import`, and
    /// `EGL_EXT_image_dma_buf_import_modifiers` with a modifier). EGL keeps its own references
    /// to the memory: the file descriptors can be closed afterwards.
    pub fn import_dma_buf(&self, dma_buf: &DmaBuf) -> Result<EglImage, EglError> {
        self.require("EGL_EXT_image_dma_buf_import")?;
        if dma_buf.modifier.is_some() {
            self.require("EGL_EXT_image_dma_buf_import_modifiers")?;
        }
        let planes = dma_buf.planes.len();
        if planes == 0 || planes > 4 || dma_buf.width == 0 || dma_buf.height == 0 {
            return Err(EglError::InvalidDmaBuf);
        }
        if dma_buf.fourcc == 0 {
            return Err(EglError::InvalidDmaBuf);
        }
        let mut attributes = vec![
            EGL_WIDTH,
            dma_buf.width as EglInt,
            EGL_HEIGHT,
            dma_buf.height as EglInt,
            EGL_LINUX_DRM_FOURCC_EXT,
            dma_buf.fourcc as EglInt,
        ];
        for (plane, names) in dma_buf.planes.iter().zip(PLANE_ATTRIBUTES) {
            attributes.extend([
                names[0],
                plane.fd.as_raw_fd(),
                names[1],
                plane.offset as EglInt,
                names[2],
                plane.stride as EglInt,
            ]);
            if let Some(modifier) = dma_buf.modifier {
                attributes.extend([
                    names[3],
                    modifier as u32 as EglInt,
                    names[4],
                    (modifier >> 32) as u32 as EglInt,
                ]);
            }
        }
        if let Some(color_space) = dma_buf.color_space {
            let value = match color_space {
                YuvColorSpace::Rec601 => 0x327F,
                YuvColorSpace::Rec709 => 0x3280,
                YuvColorSpace::Rec2020 => 0x3281,
            };
            attributes.extend([EGL_YUV_COLOR_SPACE_HINT_EXT, value]);
        }
        if let Some(range) = dma_buf.range {
            let value = match range {
                YuvRange::Full => 0x3282,
                YuvRange::Narrow => 0x3283,
            };
            attributes.extend([EGL_SAMPLE_RANGE_HINT_EXT, value]);
        }
        // the content of an image is undefined after its creation otherwise
        attributes.extend([EGL_IMAGE_PRESERVED_KHR, EGL_TRUE, EGL_NONE]);
        // dma-bufs belong to no context
        self.image(
            std::ptr::null(),
            EGL_LINUX_DMA_BUF_EXT,
            std::ptr::null(),
            &attributes,
            (dma_buf.width, dma_buf.height),
        )
    }

    /// Creates an EGL image of the level `level` of `texture` (`EGL_KHR_gl_texture_2D_image`),
    /// which shares its memory: other APIs see what SafeGL draws into the texture, and the
    /// texture what they write into the image.
    pub fn image_of_texture(&self, texture: &Texture2d, level: u32) -> Result<EglImage, EglError> {
        self.require("EGL_KHR_gl_texture_2D_image")?;
        if !Rc::ptr_eq(texture.get_context(), &self.0.context) {
            return Err(EglError::ContextMismatch);
        }
        let Some(mipmap) = texture.mipmap(level) else {
            return Err(EglError::Egl(0x300C)); // EGL_BAD_PARAMETER
        };
        let size = (mipmap.width(), mipmap.height());
        // the commands that wrote the texture go before the image is used elsewhere
        self.0.context.flush();
        // the content of the texture is undefined after the creation of the image otherwise
        let attributes = [
            EGL_GL_TEXTURE_LEVEL_KHR,
            level as EglInt,
            EGL_IMAGE_PRESERVED_KHR,
            EGL_TRUE,
            EGL_NONE,
        ];
        let id = texture.get_id().0.get() as usize;
        self.image(
            self.0.raw_context,
            EGL_GL_TEXTURE_2D_KHR,
            id as *const c_void,
            &attributes,
            size,
        )
    }

    /// Inserts a fence into the commands of the context (`EGL_KHR_fence_sync`), which is
    /// signaled when the GPU has executed the commands before it.
    pub fn fence(&self) -> Result<EglFence, EglError> {
        self.require("EGL_KHR_fence_sync")?;
        self.sync(EGL_SYNC_FENCE_KHR, &[EGL_NONE])
    }

    /// Inserts a fence that is also a file descriptor of the Linux kernel
    /// (`EGL_ANDROID_native_fence_sync`), which other APIs and processes wait for: Vulkan,
    /// the display server, a video encoder.
    pub fn native_fence(&self) -> Result<EglFence, EglError> {
        self.require("EGL_ANDROID_native_fence_sync")?;
        self.sync(EGL_SYNC_NATIVE_FENCE_ANDROID, &[EGL_NONE])
    }

    /// Imports the native fence `fd` of another API or process
    /// (`EGL_ANDROID_native_fence_sync`), for the GPU to wait for it (`EglFence::wait_on_gpu`).
    /// EGL takes the file descriptor.
    pub fn import_native_fence(&self, fd: OwnedFd) -> Result<EglFence, EglError> {
        self.require("EGL_ANDROID_native_fence_sync")?;
        use std::os::fd::IntoRawFd;
        let raw = fd.into_raw_fd();
        let attributes = [EGL_SYNC_NATIVE_FENCE_FD_ANDROID, raw, EGL_NONE];
        let fence = self.sync(EGL_SYNC_NATIVE_FENCE_ANDROID, &attributes);
        if fence.is_err() {
            // EGL took the descriptor only if it created the fence
            drop(unsafe { OwnedFd::from_raw_fd(raw) });
        }
        fence
    }

    fn sync(&self, kind: u32, attributes: &[EglInt]) -> Result<EglFence, EglError> {
        let create = self
            .0
            .functions
            .create_sync
            .ok_or(EglError::ExtensionMissing("EGL_KHR_fence_sync"))?;
        // the fence goes into the commands of the context of SafeGL
        let _ctxt = self.0.context.make_current();
        let sync = unsafe { create(self.0.raw_display, kind, attributes.as_ptr()) };
        if sync.is_null() {
            return Err(self.0.error());
        }
        Ok(EglFence {
            egl: self.clone(),
            sync,
        })
    }
}

/// An EGL image, which shares its memory with other APIs and processes. It is destroyed when
/// dropped; the textures created from it keep the memory.
pub struct EglImage {
    egl: Egl,
    image: EglImageHandle,
    width: u32,
    height: u32,
}

impl EglImage {
    /// The width of the image.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height of the image.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The raw `EGLImage`.
    pub fn raw(&self) -> *const c_void {
        self.image
    }

    /// Creates the external texture of the image (`GL_OES_EGL_image_external`), which shaders
    /// sample with `samplerExternalOES`, converting YUV images to RGB.
    pub fn external_texture<F: ?Sized + Facade>(
        &self,
        facade: &F,
    ) -> Result<ExternalTexture, TextureCreationError> {
        if !Rc::ptr_eq(facade.get_context(), &self.egl.0.context) {
            return Err(TextureCreationError::ContextLost);
        }
        unsafe { ExternalTexture::from_egl_image(facade, self.image, self.width, self.height) }
    }

    /// Creates a 2D texture whose immutable storage is the image (`GL_EXT_EGL_image_storage`), of
    /// RGB formats only: SafeGL draws into it, reads it and copies it like other textures.
    /// `format` is the format of the image, which SafeGL can't query.
    ///
    /// Returns `TextureCreationError::TypeNotSupported` without the extension, and
    /// `FormatNotSupported` if the driver can't use the image as the storage of a texture.
    pub fn texture_2d<F: ?Sized + Facade>(
        &self,
        facade: &F,
        format: UncompressedFloatFormat,
    ) -> Result<Texture2d, TextureCreationError> {
        let context = facade.get_context();
        if !Rc::ptr_eq(context, &self.egl.0.context) {
            return Err(TextureCreationError::ContextLost);
        }
        if !context.get_extensions().gl_ext_egl_image_storage {
            return Err(TextureCreationError::TypeNotSupported);
        }
        let mut ctxt = context.make_current();
        if ctxt.objects_lost() {
            return Err(TextureCreationError::ContextLost);
        }
        unsafe {
            let id = ctxt
                .glow
                .create_texture()
                .map_err(|_| TextureCreationError::ContextLost)?;
            let unit = ctxt.state.active_texture as usize;
            ctxt.glow.bind_texture(gl::TEXTURE_2D, Some(id));
            ctxt.state.texture_units[unit].texture = Some(id);
            for _ in 0..8 {
                if ctxt.glow.get_error() == gl::NO_ERROR {
                    break;
                }
            }
            ctxt.extra
                .EGLImageTargetTexStorageEXT(gl::TEXTURE_2D, self.image, std::ptr::null());
            if ctxt.glow.get_error() != gl::NO_ERROR {
                ctxt.glow.delete_texture(id);
                ctxt.state.texture_units[unit].texture = None;
                return Err(TextureCreationError::FormatNotSupported);
            }
            drop(ctxt);
            let texture = Texture2d::from_id(
                facade,
                format,
                id,
                true,
                MipmapsOption::NoMipmap,
                Dimensions::Texture2d {
                    width: self.width,
                    height: self.height,
                },
            );
            // NVIDIA reads zeros from them with `glGetTexImage`
            texture.set_image_storage();
            Ok(texture)
        }
    }

    /// Exports the image as a dma-buf (`EGL_MESA_image_dma_buf_export`), for other devices and
    /// processes: a video encoder, the display server, Vulkan.
    pub fn export_dma_buf(&self) -> Result<DmaBuf, EglError> {
        self.egl.require("EGL_MESA_image_dma_buf_export")?;
        let functions = &self.egl.0.functions;
        let missing = EglError::ExtensionMissing("EGL_MESA_image_dma_buf_export");
        let (query, export) = (
            functions.export_query.ok_or(missing)?,
            functions
                .export
                .ok_or(EglError::ExtensionMissing("EGL_MESA_image_dma_buf_export"))?,
        );
        let display = self.egl.0.raw_display;
        let (mut fourcc, mut planes, mut modifier) = (0, 0, 0u64);
        if unsafe { query(display, self.image, &mut fourcc, &mut planes, &mut modifier) } == 0 {
            return Err(self.egl.0.error());
        }
        let planes = planes.clamp(0, 4) as usize;
        let (mut fds, mut strides, mut offsets) = ([-1; 4], [0; 4], [0; 4]);
        let exported = unsafe {
            export(
                display,
                self.image,
                fds.as_mut_ptr(),
                strides.as_mut_ptr(),
                offsets.as_mut_ptr(),
            )
        };
        // the descriptors are ours from here, even on a failure
        let mut owned = Vec::with_capacity(planes);
        for &fd in fds.iter().filter(|&&fd| fd >= 0) {
            owned.push(unsafe { OwnedFd::from_raw_fd(fd) });
        }
        if exported == 0 {
            return Err(self.egl.0.error());
        }
        // planes may share a descriptor, which EGL then gives once
        let planes = (0..planes)
            .map(|plane| {
                let fd = match fds[plane] {
                    fd if fd >= 0 => fd,
                    _ => fds[0],
                };
                let fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(fd) }
                    .try_clone_to_owned()
                    .map_err(EglError::Io)?;
                Ok(DmaBufPlane {
                    fd,
                    offset: offsets[plane] as u32,
                    stride: strides[plane] as u32,
                })
            })
            .collect::<Result<Vec<_>, EglError>>()?;
        drop(owned);
        Ok(DmaBuf {
            width: self.width,
            height: self.height,
            fourcc: fourcc as u32,
            // the invalid modifier of DRM means the layout of the driver
            modifier: (modifier != 0x00ff_ffff_ffff_ffff).then_some(modifier),
            planes,
            color_space: None,
            range: None,
        })
    }
}

impl Drop for EglImage {
    fn drop(&mut self) {
        if let Some(destroy) = self.egl.0.functions.destroy_image {
            unsafe { destroy(self.egl.0.raw_display, self.image) };
        }
    }
}

impl fmt::Debug for EglImage {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "EglImage({}x{})", self.width, self.height)
    }
}

/// An EGL fence, signaled when the GPU has executed the commands before it, or by the API or
/// process that gave it.
pub struct EglFence {
    egl: Egl,
    sync: EglSync,
}

impl EglFence {
    /// Waits for the fence on the CPU, for at most `timeout`, and returns true if it is
    /// signaled (`eglClientWaitSyncKHR`). The commands before it are flushed first.
    pub fn wait(&self, timeout: Duration) -> Result<bool, EglError> {
        let wait = self
            .egl
            .0
            .functions
            .client_wait_sync
            .ok_or(EglError::ExtensionMissing("EGL_KHR_fence_sync"))?;
        let nanoseconds = timeout.as_nanos().min(u64::MAX as u128) as u64;
        let result = unsafe {
            wait(
                self.egl.0.raw_display,
                self.sync,
                EGL_SYNC_FLUSH_COMMANDS_BIT_KHR,
                nanoseconds,
            )
        };
        match result {
            EGL_CONDITION_SATISFIED_KHR => Ok(true),
            EGL_TIMEOUT_EXPIRED_KHR => Ok(false),
            _ => Err(self.egl.0.error()),
        }
    }

    /// Makes the GPU wait for the fence before the commands that come after
    /// (`eglWaitSyncKHR`, `EGL_KHR_wait_sync`), without blocking the CPU.
    pub fn wait_on_gpu(&self) -> Result<(), EglError> {
        self.egl.require("EGL_KHR_wait_sync")?;
        let wait = self
            .egl
            .0
            .functions
            .wait_sync
            .ok_or(EglError::ExtensionMissing("EGL_KHR_wait_sync"))?;
        let _ctxt = self.egl.0.context.make_current();
        if unsafe { wait(self.egl.0.raw_display, self.sync, 0) } == EGL_FALSE {
            return Err(self.egl.0.error());
        }
        Ok(())
    }

    /// Returns a new file descriptor of a native fence (`Egl::native_fence`), for other APIs
    /// and processes (`eglDupNativeFenceFDANDROID`). The commands before it are flushed first,
    /// which gives the fence its descriptor.
    pub fn native_fd(&self) -> Result<OwnedFd, EglError> {
        let dup = self
            .egl
            .0
            .functions
            .dup_native_fence
            .ok_or(EglError::ExtensionMissing("EGL_ANDROID_native_fence_sync"))?;
        self.egl.0.context.flush();
        let fd = unsafe { dup(self.egl.0.raw_display, self.sync) };
        if fd < 0 {
            return Err(self.egl.0.error());
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

impl Drop for EglFence {
    fn drop(&mut self) {
        if let Some(destroy) = self.egl.0.functions.destroy_sync {
            unsafe { destroy(self.egl.0.raw_display, self.sync) };
        }
    }
}

impl fmt::Debug for EglFence {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("EglFence")
    }
}
