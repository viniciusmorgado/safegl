/*!

Memory and semaphores shared with other APIs, like Vulkan, through file descriptors
(`GL_EXT_memory_object`, `GL_EXT_memory_object_fd`, `GL_EXT_semaphore`, `GL_EXT_semaphore_fd`):
buffers and textures whose memory another API allocated, and semaphores that order the work of
both APIs on the GPU.

Another API exports the memory as an opaque file descriptor of the same device (for Vulkan,
`VK_KHR_external_memory_fd` with `VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT`), with its size,
and SafeGL imports it. The work of the APIs is ordered by semaphores, or by waiting for it to
finish; the layouts given to semaphores are the ones of Vulkan images.

*/

use std::error::Error;
use std::fmt;
use std::os::fd::{IntoRawFd, OwnedFd};
use std::rc::Rc;

use crate::backend::Facade;
use crate::buffer::{Alloc, AnyBitPattern, Buffer, BufferType};
use crate::context::Context;
use crate::gl;
use crate::image_format::{self, RequestType, TextureFormatRequest};
use crate::texture::{
    Dimensions, MipmapsOption, Texture2d, TextureAny, UncompressedFloatFormat,
};
use crate::version::is_webgl;
use crate::{CapabilitiesSource, ContextExt, GlObject, TextureExt};

use glow::HasContext;

/// Returns true if the backend imports memory from file descriptors (`GL_EXT_memory_object`
/// and `GL_EXT_memory_object_fd`). WebGL doesn't.
pub fn is_memory_import_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    let extensions = context.get_extensions();
    !is_webgl() && extensions.gl_ext_memory_object && extensions.gl_ext_memory_object_fd
}

/// Returns true if the backend imports semaphores from file descriptors (`GL_EXT_semaphore` and
/// `GL_EXT_semaphore_fd`). WebGL doesn't.
pub fn is_semaphore_import_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    let extensions = context.get_extensions();
    !is_webgl() && extensions.gl_ext_semaphore && extensions.gl_ext_semaphore_fd
}

/// Returns the UUIDs of the device and of the driver of the context (`GL_DEVICE_UUID_EXT` and
/// `GL_DRIVER_UUID_EXT`), the `deviceUUID` and `driverUUID` of the
/// `VkPhysicalDeviceIDProperties` of the Vulkan device that shares its memory and semaphores
/// with it. `None` without `GL_EXT_memory_object` nor `GL_EXT_semaphore`.
pub fn device_uuids<F: ?Sized + Facade>(facade: &F) -> Option<([u8; 16], [u8; 16])> {
    let context = facade.get_context();
    let extensions = context.get_extensions();
    if is_webgl() || !(extensions.gl_ext_memory_object || extensions.gl_ext_semaphore) {
        return None;
    }
    let ctxt = context.make_current();
    let (mut device, mut driver) = ([0; 16], [0; 16]);
    unsafe {
        // the first device of the context, the only one outside of device groups
        ctxt.extra
            .GetUnsignedBytei_vEXT(gl::DEVICE_UUID_EXT, 0, device.as_mut_ptr());
        ctxt.extra
            .GetUnsignedBytevEXT(gl::DRIVER_UUID_EXT, driver.as_mut_ptr());
    }
    Some((device, driver))
}

/// Error while importing memory or a semaphore, or creating an object in imported memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InteropError {
    /// The backend doesn't have the extensions.
    NotSupported,
    /// The context is lost.
    ContextLost,
    /// The driver refused the file descriptor, which isn't of its device.
    ImportRefused,
    /// The object doesn't fit in the memory from its offset, or the driver refused its format.
    DoesNotFit,
    /// The format of the texture isn't supported.
    FormatNotSupported,
    /// A texture belongs to another context than the semaphore's.
    ContextMismatch,
}

impl fmt::Display for InteropError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(match self {
            InteropError::NotSupported => "The backend doesn't import memory or semaphores",
            InteropError::ContextLost => "The context is lost",
            InteropError::ImportRefused => "The driver refused the file descriptor",
            InteropError::DoesNotFit => "The object doesn't fit in the imported memory",
            InteropError::FormatNotSupported => "The format of the texture isn't supported",
            InteropError::ContextMismatch => "A texture belongs to another context",
        })
    }
}

impl Error for InteropError {}

/// Clears the error flags, so that the next `glGetError` gives the error of the next command.
unsafe fn clear_errors(ctxt: &crate::context::CommandContext<'_>) {
    for _ in 0..8 {
        if ctxt.glow.get_error() == gl::NO_ERROR {
            break;
        }
    }
}

/// Memory allocated by another API, imported from a file descriptor, in which buffers and
/// textures are created. They keep the memory when the memory object is dropped.
pub struct MemoryObject {
    context: Rc<Context>,
    id: u32,
    size: u64,
}

impl MemoryObject {
    /// Imports the memory of `fd`, of `size` bytes (`glImportMemoryFdEXT`). OpenGL takes the
    /// file descriptor. `dedicated` tells that the memory was allocated for a single image or
    /// buffer, as Vulkan's `VkMemoryDedicatedAllocateInfo` does, which the driver may need to
    /// know.
    ///
    /// # Safety
    ///
    /// `fd` must be an opaque file descriptor of memory of the device of the context, of at
    /// least `size` bytes, and the other API must not free the memory while the buffers and
    /// textures created in it are used.
    pub unsafe fn import_fd<F: ?Sized + Facade>(
        facade: &F,
        fd: OwnedFd,
        size: u64,
        dedicated: bool,
    ) -> Result<MemoryObject, InteropError> {
        let context = facade.get_context();
        if !is_memory_import_supported(&**context) {
            return Err(InteropError::NotSupported);
        }
        let ctxt = context.make_current();
        if ctxt.objects_lost() {
            return Err(InteropError::ContextLost);
        }
        let mut id = 0;
        ctxt.extra.CreateMemoryObjectsEXT(1, &mut id);
        if dedicated {
            let value: gl::types::GLint = 1;
            ctxt.extra
                .MemoryObjectParameterivEXT(id, gl::DEDICATED_MEMORY_OBJECT_EXT, &value);
        }
        clear_errors(&ctxt);
        // OpenGL owns the descriptor once it is imported
        let raw = fd.into_raw_fd();
        ctxt.extra
            .ImportMemoryFdEXT(id, size, gl::HANDLE_TYPE_OPAQUE_FD_EXT, raw);
        if ctxt.glow.get_error() != gl::NO_ERROR {
            drop(<OwnedFd as std::os::fd::FromRawFd>::from_raw_fd(raw));
            ctxt.extra.DeleteMemoryObjectsEXT(1, &id);
            return Err(InteropError::ImportRefused);
        }
        Ok(MemoryObject {
            context: context.clone(),
            id,
            size,
        })
    }

    /// The size of the memory, in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Creates a buffer of `len` elements whose storage is the memory from `offset`
    /// (`glBufferStorageMemEXT`). SafeGL writes and reads it through copies, since its storage
    /// can't be written by the client nor mapped.
    pub fn buffer<T>(
        &self,
        ty: BufferType,
        len: usize,
        offset: u64,
    ) -> Result<Buffer<[T]>, InteropError>
    where
        T: AnyBitPattern + Copy + 'static,
    {
        let size = len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(InteropError::DoesNotFit)?;
        if offset.saturating_add(size as u64) > self.size || size == 0 {
            return Err(InteropError::DoesNotFit);
        }
        let alloc = unsafe { Alloc::from_memory(&self.context, self.id, offset, size, ty) }
            .map_err(|error| match error {
                crate::buffer::BufferCreationError::ContextLost => InteropError::ContextLost,
                _ => InteropError::DoesNotFit,
            })?;
        Ok(Buffer::from_alloc(alloc))
    }

    /// Creates a 2D texture of `levels` levels, whose storage is the memory from `offset`
    /// (`glTexStorageMem2DEXT`). The other API lays the texture out like an image of the same
    /// format, size and levels, with the optimal tiling, as the driver expects.
    pub fn texture_2d(
        &self,
        format: UncompressedFloatFormat,
        width: u32,
        height: u32,
        levels: u32,
        offset: u64,
    ) -> Result<Texture2d, InteropError> {
        if offset >= self.size || levels == 0 || width == 0 || height == 0 {
            return Err(InteropError::DoesNotFit);
        }
        let internal_format = image_format::format_request_to_glenum(
            &self.context,
            TextureFormatRequest::Specific(format.to_texture_format()),
            RequestType::TexStorage(None),
        )
        .map_err(|_| InteropError::FormatNotSupported)?;
        let mut ctxt = self.context.make_current();
        if ctxt.objects_lost() {
            return Err(InteropError::ContextLost);
        }
        unsafe {
            let id = ctxt
                .glow
                .create_texture()
                .map_err(|_| InteropError::ContextLost)?;
            let unit = ctxt.state.active_texture as usize;
            ctxt.glow.bind_texture(gl::TEXTURE_2D, Some(id));
            ctxt.state.texture_units[unit].texture = Some(id);
            clear_errors(&ctxt);
            ctxt.extra.TexStorageMem2DEXT(
                gl::TEXTURE_2D,
                levels as gl::types::GLsizei,
                internal_format,
                width as gl::types::GLsizei,
                height as gl::types::GLsizei,
                self.id,
                offset,
            );
            if ctxt.glow.get_error() != gl::NO_ERROR {
                ctxt.glow.delete_texture(id);
                ctxt.state.texture_units[unit].texture = None;
                return Err(InteropError::DoesNotFit);
            }
            drop(ctxt);
            Ok(Texture2d::from_id(
                &self.context,
                format,
                id,
                true,
                MipmapsOption::EmptyMipmapsMax(levels - 1),
                Dimensions::Texture2d { width, height },
            ))
        }
    }
}

impl Drop for MemoryObject {
    fn drop(&mut self) {
        let ctxt = self.context.make_current();
        if !ctxt.objects_lost() {
            unsafe { ctxt.extra.DeleteMemoryObjectsEXT(1, &self.id) };
        }
    }
}

impl fmt::Debug for MemoryObject {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "MemoryObject({} bytes)", self.size)
    }
}

/// The layout of an image when a semaphore is signaled or waited for, which tells the other API
/// how the texture is laid out (the `VkImageLayout` of Vulkan).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageLayout {
    /// `VK_IMAGE_LAYOUT_UNDEFINED`: the content isn't kept.
    Undefined,
    /// `VK_IMAGE_LAYOUT_GENERAL`.
    General,
    /// `VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL`.
    ColorAttachment,
    /// `VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL`.
    DepthStencilAttachment,
    /// `VK_IMAGE_LAYOUT_DEPTH_STENCIL_READ_ONLY_OPTIMAL`.
    DepthStencilReadOnly,
    /// `VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL`.
    ShaderReadOnly,
    /// `VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL`.
    TransferSource,
    /// `VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL`.
    TransferDestination,
    /// `VK_IMAGE_LAYOUT_DEPTH_READ_ONLY_STENCIL_ATTACHMENT_OPTIMAL`.
    DepthReadOnlyStencilAttachment,
    /// `VK_IMAGE_LAYOUT_DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL`.
    DepthAttachmentStencilReadOnly,
}

impl ImageLayout {
    fn to_glenum(self) -> gl::types::GLenum {
        match self {
            ImageLayout::Undefined => gl::NONE,
            ImageLayout::General => 0x958D,
            ImageLayout::ColorAttachment => 0x958E,
            ImageLayout::DepthStencilAttachment => 0x958F,
            ImageLayout::DepthStencilReadOnly => 0x9590,
            ImageLayout::ShaderReadOnly => 0x9591,
            ImageLayout::TransferSource => 0x9592,
            ImageLayout::TransferDestination => 0x9593,
            ImageLayout::DepthReadOnlyStencilAttachment => 0x9530,
            ImageLayout::DepthAttachmentStencilReadOnly => 0x9531,
        }
    }
}

/// A semaphore of another API, imported from a file descriptor, which orders the work of both
/// APIs on the GPU: OpenGL waits for it before its next commands, or signals it after its
/// previous ones.
pub struct Semaphore {
    context: Rc<Context>,
    id: u32,
}

impl Semaphore {
    /// Imports the semaphore of `fd` (`glImportSemaphoreFdEXT`). OpenGL takes the file
    /// descriptor.
    ///
    /// # Safety
    ///
    /// `fd` must be an opaque file descriptor of a semaphore of the device of the context.
    pub unsafe fn import_fd<F: ?Sized + Facade>(
        facade: &F,
        fd: OwnedFd,
    ) -> Result<Semaphore, InteropError> {
        let context = facade.get_context();
        if !is_semaphore_import_supported(&**context) {
            return Err(InteropError::NotSupported);
        }
        let ctxt = context.make_current();
        if ctxt.objects_lost() {
            return Err(InteropError::ContextLost);
        }
        let mut id = 0;
        ctxt.extra.GenSemaphoresEXT(1, &mut id);
        clear_errors(&ctxt);
        let raw = fd.into_raw_fd();
        ctxt.extra
            .ImportSemaphoreFdEXT(id, gl::HANDLE_TYPE_OPAQUE_FD_EXT, raw);
        if ctxt.glow.get_error() != gl::NO_ERROR {
            drop(<OwnedFd as std::os::fd::FromRawFd>::from_raw_fd(raw));
            ctxt.extra.DeleteSemaphoresEXT(1, &id);
            return Err(InteropError::ImportRefused);
        }
        Ok(Semaphore {
            context: context.clone(),
            id,
        })
    }

    /// The names of the buffers and textures, after checking that they belong to the context.
    fn barriers(
        &self,
        buffers: &[&dyn GlObject<Id = glow::Buffer>],
        textures: &[(&TextureAny, ImageLayout)],
    ) -> Result<(Vec<u32>, Vec<u32>, Vec<gl::types::GLenum>), InteropError> {
        let others = textures
            .iter()
            .any(|(texture, _)| !Rc::ptr_eq(texture.get_context(), &self.context));
        if others {
            return Err(InteropError::ContextMismatch);
        }
        let buffers = buffers.iter().map(|buffer| buffer.get_id().0.get()).collect();
        let names = textures.iter().map(|(texture, _)| texture.get_id().0.get());
        let layouts = textures.iter().map(|(_, layout)| layout.to_glenum()).collect();
        Ok((buffers, names.collect(), layouts))
    }

    /// Makes the commands that OpenGL executes after this wait for the semaphore
    /// (`glWaitSemaphoreEXT`). The buffers and the textures are the ones that the other API
    /// wrote, with the layouts that it left the textures in.
    pub fn wait(
        &self,
        buffers: &[&dyn GlObject<Id = glow::Buffer>],
        textures: &[(&TextureAny, ImageLayout)],
    ) -> Result<(), InteropError> {
        let (buffers, textures, layouts) = self.barriers(buffers, textures)?;
        let ctxt = self.context.make_current();
        if ctxt.objects_lost() {
            return Err(InteropError::ContextLost);
        }
        unsafe {
            ctxt.extra.WaitSemaphoreEXT(
                self.id,
                buffers.len() as u32,
                buffers.as_ptr(),
                textures.len() as u32,
                textures.as_ptr(),
                layouts.as_ptr(),
            )
        };
        Ok(())
    }

    /// Signals the semaphore once OpenGL has executed the commands before this
    /// (`glSignalSemaphoreEXT`), and flushes them. The buffers and the textures are the ones that
    /// the other API reads next, with the layouts that it expects the textures in.
    pub fn signal(
        &self,
        buffers: &[&dyn GlObject<Id = glow::Buffer>],
        textures: &[(&TextureAny, ImageLayout)],
    ) -> Result<(), InteropError> {
        let (buffers, textures, layouts) = self.barriers(buffers, textures)?;
        let ctxt = self.context.make_current();
        if ctxt.objects_lost() {
            return Err(InteropError::ContextLost);
        }
        unsafe {
            ctxt.extra.SignalSemaphoreEXT(
                self.id,
                buffers.len() as u32,
                buffers.as_ptr(),
                textures.len() as u32,
                textures.as_ptr(),
                layouts.as_ptr(),
            );
            ctxt.glow.flush();
        }
        Ok(())
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        let ctxt = self.context.make_current();
        if !ctxt.objects_lost() {
            unsafe { ctxt.extra.DeleteSemaphoresEXT(1, &self.id) };
        }
    }
}

impl fmt::Debug for Semaphore {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Semaphore")
    }
}
