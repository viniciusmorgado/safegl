use crate::backend::Facade;
use crate::context::CommandContext;
use crate::context::Context;
use crate::gl;
use crate::handles::RawName;
use crate::version::Version;
use crate::CapabilitiesSource;
use crate::ContextExt;
use crate::GlObject;
use crate::TransformFeedbackSessionExt;
use glow::HasContext;
use std::cell::Cell;
use std::error::Error;
use std::ops::{Deref, DerefMut, Range};
use std::os::raw;
use std::rc::Rc;
use std::{fmt, mem, ptr};

use crate::buffer::{AnyBitPattern, BufferCreationError, BufferMode, BufferType, Content};
use crate::vertex::TransformFeedbackSession;
use crate::vertex_array_object::VertexAttributesSystem;

use crate::version::{is_webgl, Api};

/// Error that can happen when reading from a buffer.
#[derive(Debug, Copy, Clone)]
pub enum ReadError {
    /// The backend doesn't support reading from a buffer.
    NotSupported,

    /// The context has been lost. Reading from the buffer would return garbage data.
    ContextLost,
}

impl fmt::Display for ReadError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ReadError::*;
        let desc = match *self {
            NotSupported => "The backend doesn't support reading from a buffer",
            ContextLost => {
                "The context has been lost. Reading from the buffer would return garbage data"
            }
        };
        fmt.write_str(desc)
    }
}

impl Error for ReadError {}

/// Error that can happen when copying data between buffers.
#[derive(Debug, Copy, Clone)]
pub enum CopyError {
    /// The backend doesn't support copying between buffers.
    NotSupported,

    /// The target belongs to another context than the source. OpenGL objects can only be used
    /// with the context that created them.
    ContextMismatch,

    /// The target is smaller than the source.
    TargetTooSmall,
}

impl fmt::Display for CopyError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::CopyError::*;
        let desc = match *self {
            NotSupported => "The backend doesn't support copying between buffers",
            ContextMismatch => "The target belongs to another context than the source",
            TargetTooSmall => "The target is smaller than the source",
        };
        fmt.write_str(desc)
    }
}

impl Error for CopyError {}

/// A buffer in the graphics card's memory.
pub struct Alloc {
    context: Rc<Context>,

    /// OpenGL identifier ; can't be zero.
    id: glow::Buffer,

    /// Type of buffer.
    ty: BufferType,

    /// Size in bytes of the buffer.
    size: usize,

    /// A pointer to the persistent mapping of this buffer in memory, if there is one.
    persistent_mapping: Option<*mut raw::c_void>,

    /// If true, then this buffer can only be modified by calls to `glCopyBufferSubData` or through
    /// the persistent mapping.
    immutable: bool,

    /// If true, the buffer was created with the "dynamic" flag.
    creation_mode: BufferMode,

    /// If true, the buffer was created with `glBufferStorage`.
    created_with_buffer_storage: bool,

    /// True if the buffer is currently mapped with something else than persistent mapping.
    ///
    /// The purpose of this flag is to detect if the user mem::forgets the `Mapping` object.
    mapped: Cell<bool>,

    /// ID of the draw call where the buffer was last written as an SSBO.
    latest_shader_write: Cell<u64>,
}

impl Alloc {
    /// Builds a new buffer containing the given data. The size of the buffer is equal to the
    /// size of the data.
    pub fn new<D: ?Sized, F: ?Sized>(
        facade: &F,
        data: &D,
        ty: BufferType,
        mode: BufferMode,
    ) -> Result<Alloc, BufferCreationError>
    where
        D: Content,
        F: Facade,
    {
        let mut ctxt = facade.get_context().make_current();

        let size = mem::size_of_val(data);

        let (id, immutable, created_with_buffer_storage, persistent_mapping) =
            unsafe { create_buffer(&mut ctxt, size, Some(data), ty, mode) }?;

        Ok(Alloc {
            context: facade.get_context().clone(),
            id,
            ty,
            size,
            persistent_mapping,
            immutable,
            created_with_buffer_storage,
            creation_mode: mode,
            mapped: Cell::new(false),
            latest_shader_write: Cell::new(0),
        })
    }

    /// Builds a new empty buffer of the given size.
    pub fn empty<F: ?Sized>(
        facade: &F,
        ty: BufferType,
        size: usize,
        mode: BufferMode,
    ) -> Result<Alloc, BufferCreationError>
    where
        F: Facade,
    {
        let mut ctxt = facade.get_context().make_current();

        let (id, immutable, created_with_buffer_storage, persistent_mapping) =
            unsafe { create_buffer::<()>(&mut ctxt, size, None, ty, mode) }?;

        Ok(Alloc {
            context: facade.get_context().clone(),
            id,
            ty,
            size,
            persistent_mapping,
            immutable,
            created_with_buffer_storage,
            creation_mode: mode,
            mapped: Cell::new(false),
            latest_shader_write: Cell::new(0),
        })
    }

    /// Returns the context corresponding to this buffer.
    #[inline]
    pub fn get_context(&self) -> &Rc<Context> {
        &self.context
    }

    /// Returns the total size in bytes of this buffer.
    #[inline]
    pub fn get_size(&self) -> usize {
        self.size
    }

    /// Returns true if the buffer is persistently mapped in memory.
    #[inline]
    pub fn uses_persistent_mapping(&self) -> bool {
        self.persistent_mapping.is_some()
    }

    /// Changes the type of the buffer. Returns `Err` if this is forbidden.
    pub fn set_type(mut self, ty: BufferType) -> Result<Alloc, Alloc> {
        // FIXME: return Err for GLES2
        self.ty = ty;
        Ok(self)
    }

    /// Asserts that the buffer is not mapped and available for operations.
    /// No-op for persistent mapping.
    fn assert_unmapped(&self, ctxt: &mut CommandContext<'_>) {
        if self.mapped.get() {
            unsafe { unmap_buffer(ctxt, self.id, self.ty) };
            self.mapped.set(false);
        }
    }

    /// Ensures that the buffer isn't used by the transform feedback process.
    #[inline]
    fn assert_not_transform_feedback(&self, ctxt: &mut CommandContext<'_>) {
        TransformFeedbackSession::ensure_buffer_out_of_transform_feedback(ctxt, self.id);
    }

    /// Calls `glMemoryBarrier(GL_BUFFER_UPDATE_BARRIER_BIT)` if necessary.
    fn barrier_for_buffer_update(&self, ctxt: &mut CommandContext<'_>) {
        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_buffer_update {
            unsafe {
                ctxt.glow.memory_barrier(glow::BUFFER_UPDATE_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_buffer_update = ctxt.state.next_draw_call_id;
        }
    }

    /// Calls `glMemoryBarrier(GL_VERTEX_ATTRIB_ARRAY_BARRIER_BIT)` if necessary.
    pub fn prepare_for_vertex_attrib_array(&self, ctxt: &mut CommandContext<'_>) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_vertex_attrib_array {
            unsafe {
                ctxt.glow
                    .memory_barrier(glow::VERTEX_ATTRIB_ARRAY_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_vertex_attrib_array = ctxt.state.next_draw_call_id;
        }
    }

    /// Calls `glMemoryBarrier(ELEMENT_ARRAY_BARRIER_BIT)` if necessary.
    pub fn prepare_for_element_array(&self, ctxt: &mut CommandContext<'_>) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_element_array {
            unsafe {
                ctxt.glow.memory_barrier(glow::ELEMENT_ARRAY_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_element_array = ctxt.state.next_draw_call_id;
        }
    }

    /// Binds the buffer to `GL_ELEMENT_ARRAY_BUFFER` regardless of the current vertex array object.
    pub fn bind_to_element_array(&self, ctxt: &mut CommandContext<'_>) {
        unsafe {
            ctxt.glow
                .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.id));
        }
    }

    /// Makes sure that the buffer is bound to the `GL_PIXEL_PACK_BUFFER` and calls
    /// `glMemoryBarrier(GL_PIXEL_BUFFER_BARRIER_BIT)` if necessary.
    pub fn prepare_and_bind_for_pixel_pack(&self, ctxt: &mut CommandContext<'_>) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_pixel_buffer {
            unsafe {
                ctxt.glow.memory_barrier(glow::PIXEL_BUFFER_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_pixel_buffer = ctxt.state.next_draw_call_id;
        }

        unsafe {
            bind_buffer(ctxt, Some(self.id), BufferType::PixelPackBuffer);
        }
    }

    /// Makes sure that nothing is bound to `GL_PIXEL_PACK_BUFFER`.
    #[inline]
    pub fn unbind_pixel_pack(ctxt: &mut CommandContext<'_>) {
        unsafe {
            bind_buffer(ctxt, None, BufferType::PixelPackBuffer);
        }
    }

    /// Makes sure that the buffer is bound to the `GL_PIXEL_UNPACK_BUFFER` and calls
    /// `glMemoryBarrier(GL_PIXEL_BUFFER_BARRIER_BIT)` if necessary.
    pub fn prepare_and_bind_for_pixel_unpack(&self, ctxt: &mut CommandContext<'_>) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_pixel_buffer {
            unsafe {
                ctxt.glow.memory_barrier(glow::PIXEL_BUFFER_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_pixel_buffer = ctxt.state.next_draw_call_id;
        }

        unsafe {
            bind_buffer(ctxt, Some(self.id), BufferType::PixelUnpackBuffer);
        }
    }

    /// Makes sure that nothing is bound to `GL_PIXEL_UNPACK_BUFFER`.
    #[inline]
    pub fn unbind_pixel_unpack(ctxt: &mut CommandContext<'_>) {
        unsafe {
            bind_buffer(ctxt, None, BufferType::PixelUnpackBuffer);
        }
    }

    /// Makes sure that the buffer is bound to the `GL_QUERY_BUFFER` and calls
    /// `glMemoryBarrier(GL_QUERY_BUFFER_BARRIER_BIT)` if necessary.
    #[cfg(feature = "gl46")]
    pub fn prepare_and_bind_for_query(&self, ctxt: &mut CommandContext<'_>) {
        assert!(
            ctxt.version >= &Version(Api::Gl, 4, 4)
                || ctxt.extensions.gl_arb_query_buffer_object
                || ctxt.extensions.gl_amd_query_buffer_object
        );

        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_query_buffer {
            unsafe {
                ctxt.glow.memory_barrier(glow::QUERY_BUFFER_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_query_buffer = ctxt.state.next_draw_call_id;
        }

        unsafe {
            bind_buffer(ctxt, Some(self.id), BufferType::QueryBuffer);
        }
    }

    /// Makes sure that nothing is bound to `GL_QUERY_BUFFER`.
    #[cfg(feature = "gl46")]
    #[inline]
    pub fn unbind_query(ctxt: &mut CommandContext<'_>) {
        unsafe {
            bind_buffer(ctxt, None, BufferType::QueryBuffer);
        }
    }

    /// Makes sure that the buffer is bound to the `GL_DRAW_INDIRECT_BUFFER` and calls
    /// `glMemoryBarrier(GL_COMMAND_BARRIER_BIT)` if necessary.
    pub fn prepare_and_bind_for_draw_indirect(&self, ctxt: &mut CommandContext<'_>) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_command {
            unsafe {
                ctxt.glow.memory_barrier(glow::COMMAND_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_command = ctxt.state.next_draw_call_id;
        }

        unsafe {
            bind_buffer(ctxt, Some(self.id), BufferType::DrawIndirectBuffer);
        }
    }

    /// Makes sure that the buffer is bound to the `GL_DISPATCH_INDIRECT_BUFFER` and calls
    /// `glMemoryBarrier(GL_COMMAND_BARRIER_BIT)` if necessary.
    #[cfg(feature = "gl46")]
    pub fn prepare_and_bind_for_dispatch_indirect(&self, ctxt: &mut CommandContext<'_>) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_command {
            unsafe {
                ctxt.glow.memory_barrier(glow::COMMAND_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_command = ctxt.state.next_draw_call_id;
        }

        unsafe {
            bind_buffer(ctxt, Some(self.id), BufferType::DispatchIndirectBuffer);
        }
    }

    /// Makes sure that the buffer is bound to the indexed `GL_UNIFORM_BUFFER` point and calls
    /// `glMemoryBarrier(GL_UNIFORM_BARRIER_BIT)` if necessary.
    pub fn prepare_and_bind_for_uniform(
        &self,
        ctxt: &mut CommandContext<'_>,
        index: gl::types::GLuint,
        range: Range<usize>,
    ) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_uniform {
            unsafe {
                ctxt.glow.memory_barrier(glow::UNIFORM_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_uniform = ctxt.state.next_draw_call_id;
        }

        self.indexed_bind(ctxt, BufferType::UniformBuffer, index, range);
    }

    /// Makes sure that the buffer is bound to the indexed `GL_SHARED_STORAGE_BUFFER` point and calls
    /// `glMemoryBarrier(GL_SHADER_STORAGE_BARRIER_BIT)` if necessary.
    #[cfg(feature = "gl46")]
    pub fn prepare_and_bind_for_shared_storage(
        &self,
        ctxt: &mut CommandContext<'_>,
        index: gl::types::GLuint,
        range: Range<usize>,
    ) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_shader_storage {
            unsafe {
                ctxt.glow.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_shader_storage = ctxt.state.next_draw_call_id;
        }

        self.indexed_bind(ctxt, BufferType::ShaderStorageBuffer, index, range);

        self.latest_shader_write.set(ctxt.state.next_draw_call_id); // TODO: put this somewhere else
    }

    /// Makes sure that the buffer is bound to the indexed `GL_ATOMIC_COUNTER_BUFFER` point and calls
    /// `glMemoryBarrier(GL_ATOMIC_COUNTER_BARRIER_BIT)` if necessary.
    #[cfg(feature = "gl46")]
    pub fn prepare_and_bind_for_atomic_counter(
        &self,
        ctxt: &mut CommandContext<'_>,
        index: gl::types::GLuint,
        range: Range<usize>,
    ) {
        self.assert_unmapped(ctxt);
        self.assert_not_transform_feedback(ctxt);

        if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_atomic_counter {
            unsafe {
                ctxt.glow.memory_barrier(glow::ATOMIC_COUNTER_BARRIER_BIT);
            }
            ctxt.state.latest_memory_barrier_atomic_counter = ctxt.state.next_draw_call_id;
        }

        self.indexed_bind(ctxt, BufferType::AtomicCounterBuffer, index, range);

        self.latest_shader_write.set(ctxt.state.next_draw_call_id); // TODO: put this somewhere else
    }

    /// Binds the buffer to `GL_TRANSFORM_FEEDBACk_BUFFER` regardless of the current transform
    /// feedback object.
    #[inline]
    pub fn bind_to_transform_feedback(
        &self,
        ctxt: &mut CommandContext<'_>,
        index: gl::types::GLuint,
        range: Range<usize>,
    ) {
        self.indexed_bind(ctxt, BufferType::TransformFeedbackBuffer, index, range);
    }

    /// Makes sure that the buffer is bound to a specific bind point.
    ///
    /// The bind point is the value passed to `ty`.
    ///
    /// # Panic
    ///
    /// Panics if the backend doesn't allow binding this buffer to the specified point.
    #[inline]
    fn bind(&self, ctxt: &mut CommandContext<'_>, ty: BufferType) {
        self.assert_unmapped(ctxt);
        unsafe {
            bind_buffer(ctxt, Some(self.id), ty);
        }
    }

    /// Makes sure that the buffer is bound to a specific indexed bind point.
    ///
    /// The bind point is the value passed to `ty`.
    ///
    /// # Panic
    ///
    /// - Panics if `range` is out of range.
    /// - Panics if the backend doesn't allow binding this buffer to the specified point.
    /// - Panics if the bind point is not an indexed bind point.
    /// - Panics if the bind point is over the maximum value.
    #[inline]
    fn indexed_bind(
        &self,
        ctxt: &mut CommandContext<'_>,
        ty: BufferType,
        index: gl::types::GLuint,
        range: Range<usize>,
    ) {
        self.assert_unmapped(ctxt);
        unsafe {
            indexed_bind_buffer(ctxt, self.id, ty, index, range);
        }
    }

    /// Uploads data in the buffer.
    ///
    /// The data must fit inside the buffer.
    ///
    /// # Panic
    ///
    /// Panics if `offset_bytes` is out of range or the data is too large to fit in the buffer.
    ///
    /// # Unsafety
    ///
    /// If the buffer uses persistent mapping, the caller of this function must handle
    /// synchronization.
    ///
    pub unsafe fn upload<D: ?Sized>(&self, offset_bytes: usize, data: &D)
    where
        D: Content,
    {
        assert!(offset_bytes + mem::size_of_val(data) <= self.size);

        if self.persistent_mapping.is_some() {
            let mapping = Mapping {
                mapping: self.map_shared(
                    offset_bytes..offset_bytes + mem::size_of_val(data),
                    false,
                    true,
                ),
            };
            ptr::copy_nonoverlapping(
                data.to_void_ptr() as *const u8,
                <D as Content>::to_void_ptr(&mapping) as *mut u8,
                mem::size_of_val(data),
            );
        } else if self.immutable {
            let mut ctxt = self.context.make_current();
            self.barrier_for_buffer_update(&mut ctxt);

            self.assert_unmapped(&mut ctxt);
            self.assert_not_transform_feedback(&mut ctxt);

            let (tmp_buffer, _, _, _) = create_buffer(
                &mut ctxt,
                mem::size_of_val(data),
                Some(data),
                BufferType::CopyReadBuffer,
                BufferMode::Dynamic,
            )
            .unwrap();
            copy_buffer(
                &mut ctxt,
                tmp_buffer,
                0,
                self.id,
                offset_bytes,
                mem::size_of_val(data),
            )
            .unwrap();
            destroy_buffer(&mut ctxt, tmp_buffer);
        } else {
            assert!(offset_bytes < self.size);

            let mut ctxt = self.context.make_current();
            self.barrier_for_buffer_update(&mut ctxt);

            let invalidate_all = offset_bytes == 0 && mem::size_of_val(data) == self.size;

            self.assert_unmapped(&mut ctxt);
            self.assert_not_transform_feedback(&mut ctxt);

            if invalidate_all
                && (ctxt.version >= &Version(Api::Gl, 4, 3)
                    || ctxt.extensions.gl_arb_invalidate_subdata)
            {
                ctxt.extra.InvalidateBufferData(self.id.raw_name());
            }

            let data_bytes =
                std::slice::from_raw_parts(data.to_void_ptr() as *const u8, mem::size_of_val(data));

            if ctxt.version >= &Version(Api::Gl, 4, 5) {
                ctxt.glow
                    .named_buffer_sub_data_u8_slice(self.id, offset_bytes as i32, data_bytes)
            } else {
                let bind = bind_buffer(&mut ctxt, Some(self.id), self.ty);
                ctxt.glow
                    .buffer_sub_data_u8_slice(bind, offset_bytes as i32, data_bytes);
            }
        }
    }

    /// Invalidates the content of the buffer. The data becomes undefined.
    ///
    /// `offset` and `size` are both in bytes.
    ///
    /// # Panic
    ///
    /// Panics if out of range.
    ///
    pub fn invalidate(&self, offset: usize, size: usize) {
        assert!(offset + size <= self.size);

        let is_whole_buffer = offset == 0 && size == self.size;

        let mut ctxt = self.context.make_current();
        self.assert_unmapped(&mut ctxt);
        self.assert_not_transform_feedback(&mut ctxt);

        if self.persistent_mapping.is_none()
            && (ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.extensions.gl_arb_invalidate_subdata)
        {
            if is_whole_buffer {
                unsafe { ctxt.extra.InvalidateBufferData(self.id.raw_name()) };
            } else {
                unsafe {
                    ctxt.extra.InvalidateBufferSubData(
                        self.id.raw_name(),
                        offset as gl::types::GLintptr,
                        size as gl::types::GLsizeiptr,
                    )
                };
            }
        } else if !self.created_with_buffer_storage && is_whole_buffer {
            let flags = match self.creation_mode {
                BufferMode::Default | BufferMode::Immutable => gl::STATIC_DRAW,
                BufferMode::Persistent | BufferMode::Dynamic => gl::DYNAMIC_DRAW,
            };

            unsafe {
                let bind = bind_buffer(&mut ctxt, Some(self.id), self.ty);
                ctxt.glow.buffer_data_size(bind, size as i32, flags);
            }
        }
    }

    /// Returns a mapping in memory of the content of the buffer.
    ///
    /// There are two possibilities:
    ///
    ///  - If the buffer uses persistent mapping, it will simply return a wrapper around the
    ///    pointer to the existing mapping.
    ///  - If the buffer doesn't use persistent mapping, it will create a temporary buffer which
    ///    will be mapped. After the mapping is released, the temporary buffer will be copied
    ///    to the real buffer.
    ///
    /// In the second case, the changes will only be written when the mapping is released.
    /// Therefore this API is error-prone and shouldn't be exposed directly to the user. Instead
    /// `map` public functions should take a `&mut self` instead of a `&self` to prevent users
    /// from manipulating the buffer while it is "mapped".
    ///
    /// Contrary to `map_mut`, this function only requires a `&self`.
    ///
    /// # Panic
    ///
    /// Panics if the `bytes_range` is not aligned to a mappable slice.
    ///
    /// # Unsafety
    ///
    /// If the buffer uses persistent mapping, the caller of this function must handle
    /// synchronization.
    ///
    /// If you pass `false` for `read`, you **must not** read the returned buffer. If you pass
    /// `false` for `write`, you **must not** write the returned buffer.
    ///
    unsafe fn map_shared<D: ?Sized>(
        &self,
        bytes_range: Range<usize>,
        read: bool,
        write: bool,
    ) -> MappingImpl<'_, D>
    where
        D: Content,
    {
        if let Some(existing_mapping) = self.persistent_mapping {
            // TODO: optimize so that it's not always necessary to make the context current
            let mut ctxt = self.context.make_current();
            // shader writes must also be made visible to the persistent mapping that the CPU reads
            if self.latest_shader_write.get() >= ctxt.state.latest_memory_barrier_buffer_update {
                ctxt.glow.memory_barrier(
                    glow::BUFFER_UPDATE_BARRIER_BIT | glow::CLIENT_MAPPED_BUFFER_BARRIER_BIT,
                );
                ctxt.state.latest_memory_barrier_buffer_update = ctxt.state.next_draw_call_id;
            }

            let data = (existing_mapping as *mut u8).add(bytes_range.start);
            let data = Content::ref_from_ptr(data as *mut (), bytes_range.end - bytes_range.start)
                .unwrap();

            MappingImpl::PersistentMapping {
                buffer: self,
                offset_bytes: bytes_range.start,
                data,
                needs_flushing: write,
            }
        } else if is_webgl() {
            self.map_shadow_copy(bytes_range, read, write)
        } else {
            let size_bytes = bytes_range.end - bytes_range.start;

            let mut ctxt = self.context.make_current();

            // we have to construct a temporary buffer that we will map in memory
            // then after the Mapping is destroyed, we will copy from the temporary buffer to the
            // real one
            let temporary_buffer = {
                let (temporary_buffer, _, _, _) = create_buffer::<D>(
                    &mut ctxt,
                    size_bytes,
                    None,
                    BufferType::CopyWriteBuffer,
                    BufferMode::Dynamic,
                )
                .unwrap();
                temporary_buffer
            };

            let ptr = {
                self.assert_unmapped(&mut ctxt);
                self.assert_not_transform_feedback(&mut ctxt);

                if read {
                    copy_buffer(
                        &mut ctxt,
                        self.id,
                        bytes_range.start,
                        temporary_buffer,
                        0,
                        size_bytes,
                    )
                    .unwrap();
                }

                map_buffer(
                    &mut ctxt,
                    temporary_buffer,
                    self.ty,
                    0..size_bytes,
                    true,
                    true,
                )
                .expect("Buffer mapping is not supported by the backend")
            };

            let data = match Content::ref_from_ptr(ptr, bytes_range.end - bytes_range.start) {
                Some(data) => data,
                None => {
                    unmap_buffer(&mut ctxt, temporary_buffer, self.ty);
                    panic!("Wrong bytes range");
                }
            };

            MappingImpl::TemporaryBuffer {
                original_buffer: self,
                original_buffer_offset: bytes_range.start,
                temporary_buffer,
                temporary_buffer_data: data,
                needs_flushing: write,
            }
        }
    }

    /// Maps a range of the buffer by copying it to client memory. Used on WebGL, which has no
    /// `glMapBufferRange`.
    ///
    /// If `read` is true, the range is downloaded with `glGetBufferSubData`. If `write` is true,
    /// the content is uploaded back with `glBufferSubData` when the mapping is dropped, so, like
    /// the temporary-buffer mapping of `map_shared`, changes are only visible to OpenGL after
    /// the mapping is released.
    ///
    /// # Panic
    ///
    /// Panics if the `bytes_range` is not aligned to a mappable slice.
    ///
    unsafe fn map_shadow_copy<D: ?Sized>(
        &self,
        bytes_range: Range<usize>,
        read: bool,
        write: bool,
    ) -> MappingImpl<'_, D>
    where
        D: Content,
    {
        let size_bytes = bytes_range.end - bytes_range.start;
        assert!(bytes_range.end <= self.size);

        let chunk_size = mem::size_of::<ShadowChunk>();
        let mut storage = vec![ShadowChunk([0; 16]); size_bytes.div_ceil(chunk_size)];
        let ptr = storage.as_mut_ptr() as *mut u8;

        let data = Content::ref_from_ptr(ptr as *mut (), size_bytes).expect("Wrong bytes range");

        if read && size_bytes != 0 {
            let mut ctxt = self.context.make_current();
            self.assert_unmapped(&mut ctxt);
            self.assert_not_transform_feedback(&mut ctxt);
            self.barrier_for_buffer_update(&mut ctxt);

            let bind = bind_buffer(&mut ctxt, Some(self.id), self.ty);
            ctxt.glow.get_buffer_sub_data(
                bind,
                bytes_range.start as i32,
                std::slice::from_raw_parts_mut(ptr, size_bytes),
            );
        }

        MappingImpl::ShadowCopy {
            buffer: self,
            offset_bytes: bytes_range.start,
            _storage: storage,
            data,
            needs_flushing: write,
        }
    }

    /// Returns a mapping in memory of the content of the buffer.
    ///
    /// There are three possibilities:
    ///
    ///  - If the buffer uses persistent mapping, it will simply return a wrapper around the
    ///    pointer to the existing mapping.
    ///  - On WebGL, which can't map buffers, it will return a copy of the range in client
    ///    memory that is uploaded back when the mapping is released (see `map_shadow_copy`).
    ///  - Otherwise, it will map the buffer.
    ///
    /// Contrary to `map_shared`, this function requires a `&mut self`. It can only be used if
    /// you have exclusive access to the buffer.
    ///
    /// # Panic
    ///
    /// Panics if the `bytes_range` is not aligned to a mappable slice.
    ///
    /// # Unsafety
    ///
    /// If the buffer uses persistent mapping, the caller of this function must handle
    /// synchronization.
    ///
    /// If you pass `false` for `read`, you **must not** read the returned buffer. If you pass
    /// `false` for `write`, you **must not** write the returned buffer.
    ///
    unsafe fn map_impl<D: ?Sized>(
        &mut self,
        bytes_range: Range<usize>,
        read: bool,
        write: bool,
    ) -> MappingImpl<'_, D>
    where
        D: Content,
    {
        if self.persistent_mapping.is_some() || self.immutable || is_webgl() {
            self.map_shared(bytes_range, read, write)
        } else {
            let data = {
                let mut ctxt = self.context.make_current();

                let ptr = {
                    self.assert_unmapped(&mut ctxt);
                    self.assert_not_transform_feedback(&mut ctxt);
                    self.barrier_for_buffer_update(&mut ctxt);
                    let ptr = map_buffer(
                        &mut ctxt,
                        self.id,
                        self.ty,
                        bytes_range.clone(),
                        read,
                        write,
                    )
                    .expect("Buffer mapping is not supported by the backend");
                    self.mapped.set(true);
                    ptr
                };

                match Content::ref_from_ptr(ptr, bytes_range.end - bytes_range.start) {
                    Some(data) => data,
                    None => {
                        unmap_buffer(&mut ctxt, self.id, self.ty);
                        panic!("Wrong bytes range");
                    }
                }
            };

            MappingImpl::RegularMapping {
                buffer: self,
                data,
                needs_flushing: write,
            }
        }
    }

    /// Returns a read and write mapping in memory of the content of the buffer.
    ///
    /// # Panic
    ///
    /// Panics if the `bytes_range` is not aligned to a mappable slice.
    ///
    /// # Unsafety
    ///
    /// If the buffer uses persistent mapping, the caller of this function must handle
    /// synchronization.
    ///
    #[inline]
    pub unsafe fn map<D: ?Sized>(&mut self, bytes_range: Range<usize>) -> Mapping<'_, D>
    where
        D: Content,
    {
        Mapping {
            mapping: self.map_impl(bytes_range, true, true),
        }
    }

    /// Returns a read-only mapping in memory of the content of the buffer.
    ///
    /// # Panic
    ///
    /// Panics if the `bytes_range` is not aligned to a mappable slice.
    ///
    /// # Unsafety
    ///
    /// If the buffer uses persistent mapping, the caller of this function must handle
    /// synchronization.
    ///
    #[inline]
    pub unsafe fn map_read<D: ?Sized>(&mut self, bytes_range: Range<usize>) -> ReadMapping<'_, D>
    where
        D: Content,
    {
        ReadMapping {
            mapping: self.map_impl(bytes_range, true, false),
        }
    }

    /// Returns a write-only mapping in memory of the content of the buffer.
    ///
    /// # Panic
    ///
    /// Panics if the `bytes_range` is not aligned to a mappable slice.
    ///
    /// # Unsafety
    ///
    /// If the buffer uses persistent mapping, the caller of this function must handle
    /// synchronization.
    ///
    #[inline]
    pub unsafe fn map_write<D: ?Sized>(&mut self, bytes_range: Range<usize>) -> WriteMapping<'_, D>
    where
        D: Content,
    {
        WriteMapping {
            mapping: self.map_impl(bytes_range, false, true),
        }
    }

    /// Reads the content of the buffer.
    ///
    /// # Panic
    ///
    /// Panics if out of range.
    ///
    /// # Unsafety
    ///
    /// If the buffer uses persistent mapping, the caller of this function must handle
    /// synchronization.
    ///
    pub unsafe fn read<D: ?Sized>(&self, range: Range<usize>) -> Result<D::Owned, ReadError>
    where
        D: Content + AnyBitPattern,
    {
        let size_to_read = range.end - range.start;

        if self.persistent_mapping.is_some() {
            let mapping = ReadMapping {
                mapping: self.map_shared(range, true, false),
            };
            <D as Content>::read(size_to_read, |output| {
                ptr::copy_nonoverlapping(
                    <D as Content>::to_void_ptr(&mapping) as *const u8,
                    output as *mut D as *mut u8,
                    size_to_read,
                );
                Ok(())
            })
        } else {
            let mut ctxt = self.context.make_current();

            if ctxt.state.lost_context {
                return Err(ReadError::ContextLost);
            }

            self.assert_unmapped(&mut ctxt);
            self.barrier_for_buffer_update(&mut ctxt);

            <D as Content>::read(size_to_read, |output| {
                // `glGetBufferSubData` exists on desktop GL and WebGL 2, but not on OpenGL ES,
                // where the range is mapped for reading instead
                if ctxt.version.0 == Api::Gl || is_webgl() {
                    let bind = bind_buffer(&mut ctxt, Some(self.id), self.ty);
                    ctxt.glow.get_buffer_sub_data(
                        bind,
                        range.start as i32,
                        std::slice::from_raw_parts_mut(output as *mut D as *mut u8, size_to_read),
                    );
                } else if size_to_read != 0 {
                    let mapping =
                        map_buffer(&mut ctxt, self.id, self.ty, range.clone(), true, false)
                            .filter(|mapping| !mapping.is_null())
                            .ok_or(ReadError::NotSupported)?;
                    ptr::copy_nonoverlapping(
                        mapping as *const u8,
                        output as *mut D as *mut u8,
                        size_to_read,
                    );
                    unmap_buffer(&mut ctxt, self.id, self.ty);
                }

                Ok(())
            })
        }
    }

    /// Copies data from this buffer to another one.
    ///
    /// With persistent-mapped buffers you must create a sync fence *after* this operation.
    ///
    /// # Panic
    ///
    /// Panics if the range is out of the source.
    ///
    pub fn copy_to(
        &self,
        range: Range<usize>,
        target: &Alloc,
        dest_offset: usize,
    ) -> Result<(), CopyError> {
        // OpenGL objects can only be used with the context that created them
        if !Rc::ptr_eq(&self.context, &target.context) {
            return Err(CopyError::ContextMismatch);
        }

        // the ranges come from slices, which are always inside their buffer
        assert!(range.end >= range.start);
        assert!(range.end <= self.size);
        if dest_offset + range.end - range.start > target.size {
            return Err(CopyError::TargetTooSmall);
        }

        let mut ctxt = self.context.make_current();

        unsafe {
            copy_buffer(
                &mut ctxt,
                self.id,
                range.start,
                target.id,
                dest_offset,
                range.end - range.start,
            )
        }
    }
}

impl fmt::Debug for Alloc {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "Buffer {:?} (size: {} bytes)", self.id, self.size)
    }
}

impl Drop for Alloc {
    fn drop(&mut self) {
        unsafe {
            let mut ctxt = self.context.make_current();
            self.assert_unmapped(&mut ctxt);
            self.assert_not_transform_feedback(&mut ctxt);
            VertexAttributesSystem::purge_buffer(&mut ctxt, self.id);
            destroy_buffer(&mut ctxt, self.id);
        }
    }
}

impl GlObject for Alloc {
    type Id = glow::Buffer;

    #[inline]
    fn get_id(&self) -> glow::Buffer {
        self.id
    }
}

/// A mapping of a buffer. Private object.
enum MappingImpl<'b, D: ?Sized> {
    PersistentMapping {
        buffer: &'b Alloc,
        offset_bytes: usize,
        data: *mut D,
        needs_flushing: bool,
    },

    TemporaryBuffer {
        original_buffer: &'b Alloc,
        original_buffer_offset: usize,
        temporary_buffer: glow::Buffer,
        temporary_buffer_data: *mut D,
        needs_flushing: bool,
    },

    RegularMapping {
        buffer: &'b mut Alloc,
        data: *mut D,
        needs_flushing: bool,
    },

    /// WebGL only: a copy of the range in client memory, see `Alloc::map_shadow_copy`.
    ShadowCopy {
        buffer: &'b Alloc,
        offset_bytes: usize,
        /// Owns the memory `data` points to.
        _storage: Vec<ShadowChunk>,
        data: *mut D,
        needs_flushing: bool,
    },
}

/// Unit of storage of a `ShadowCopy` mapping. The alignment makes the storage suitable for any
/// buffer content type.
#[derive(Copy, Clone)]
#[repr(C, align(16))]
struct ShadowChunk([u8; 16]);

impl<'b, D: ?Sized> MappingImpl<'b, D> {
    /// Returns the pointer to the mapped data.
    #[inline]
    fn data_ptr(&self) -> *mut D {
        match *self {
            MappingImpl::PersistentMapping { data, .. } => data,
            MappingImpl::TemporaryBuffer {
                temporary_buffer_data,
                ..
            } => temporary_buffer_data,
            MappingImpl::RegularMapping { data, .. } => data,
            MappingImpl::ShadowCopy { data, .. } => data,
        }
    }
}

unsafe impl<'a, D: ?Sized> Sync for MappingImpl<'a, D> where D: Send + Sync {}

impl<'a, D: ?Sized> Drop for MappingImpl<'a, D> {
    fn drop(&mut self) {
        match *self {
            MappingImpl::PersistentMapping {
                buffer,
                offset_bytes,
                data,
                needs_flushing,
            } => {
                let mut ctxt = buffer.context.make_current();
                unsafe {
                    if needs_flushing {
                        flush_range(
                            &mut ctxt,
                            buffer.id,
                            buffer.ty,
                            offset_bytes..offset_bytes + mem::size_of_val(&*data),
                        );
                    }
                }
            }

            MappingImpl::TemporaryBuffer {
                original_buffer,
                original_buffer_offset,
                temporary_buffer,
                temporary_buffer_data,
                needs_flushing,
            } => {
                let mut ctxt = original_buffer.context.make_current();
                original_buffer.barrier_for_buffer_update(&mut ctxt);

                unsafe {
                    if needs_flushing {
                        flush_range(
                            &mut ctxt,
                            temporary_buffer,
                            original_buffer.ty,
                            0..mem::size_of_val(&*temporary_buffer_data),
                        );
                    }
                    unmap_buffer(&mut ctxt, temporary_buffer, original_buffer.ty);
                    if needs_flushing {
                        copy_buffer(
                            &mut ctxt,
                            temporary_buffer,
                            0,
                            original_buffer.id,
                            original_buffer_offset,
                            mem::size_of_val(&*temporary_buffer_data),
                        )
                        .unwrap();
                    }

                    destroy_buffer(&mut ctxt, temporary_buffer);
                }
            }

            MappingImpl::RegularMapping {
                ref mut buffer,
                data,
                needs_flushing,
            } => {
                let mut ctxt = buffer.context.make_current();

                unsafe {
                    if needs_flushing {
                        flush_range(&mut ctxt, buffer.id, buffer.ty, 0..mem::size_of_val(&*data));
                    }
                    unmap_buffer(&mut ctxt, buffer.id, buffer.ty);
                }

                buffer.mapped.set(false);
            }

            MappingImpl::ShadowCopy {
                buffer,
                offset_bytes,
                data,
                needs_flushing,
                ..
            } => unsafe {
                let size_bytes = mem::size_of_val(&*data);
                if needs_flushing && size_bytes != 0 {
                    let mut ctxt = buffer.context.make_current();
                    buffer.assert_not_transform_feedback(&mut ctxt);
                    buffer.barrier_for_buffer_update(&mut ctxt);

                    let bind = bind_buffer(&mut ctxt, Some(buffer.id), buffer.ty);
                    ctxt.glow.buffer_sub_data_u8_slice(
                        bind,
                        offset_bytes as i32,
                        std::slice::from_raw_parts(data as *const u8, size_bytes),
                    );
                }
            },
        }
    }
}

/// A mapping of a buffer for reading and writing.
pub struct Mapping<'b, D: ?Sized>
where
    D: Content,
{
    mapping: MappingImpl<'b, D>,
}

impl<'a, D: ?Sized> Deref for Mapping<'a, D>
where
    D: Content,
{
    type Target = D;

    #[inline]
    fn deref(&self) -> &D {
        unsafe { &*self.mapping.data_ptr() }
    }
}

impl<'a, D: ?Sized> DerefMut for Mapping<'a, D>
where
    D: Content,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut D {
        unsafe { &mut *self.mapping.data_ptr() }
    }
}

/// A mapping of a buffer for reading.
pub struct ReadMapping<'b, D: ?Sized>
where
    D: Content,
{
    mapping: MappingImpl<'b, D>,
}

impl<'a, D: ?Sized> Deref for ReadMapping<'a, D>
where
    D: Content,
{
    type Target = D;

    #[inline]
    fn deref(&self) -> &D {
        unsafe { &*self.mapping.data_ptr() }
    }
}

/// A mapping of a buffer for write only.
pub struct WriteMapping<'b, D: ?Sized>
where
    D: Content,
{
    mapping: MappingImpl<'b, D>,
}

impl<'b, D: ?Sized> WriteMapping<'b, D>
where
    D: Content,
{
    #[inline]
    fn get_slice(&mut self) -> &mut D {
        unsafe { &mut *self.mapping.data_ptr() }
    }
}

impl<'b, D> WriteMapping<'b, D>
where
    D: Content + Copy,
{
    /// Writes the whole content.
    #[inline]
    pub fn write(&mut self, value: D) {
        let slice = self.get_slice();
        *slice = value;
    }
}

impl<'b, D> WriteMapping<'b, [D]>
where
    [D]: Content,
    D: Copy,
{
    /// Returns the length of the mapping.
    #[inline]
    pub fn len(&self) -> usize {
        unsafe { (&*self.mapping.data_ptr()).len() }
    }

    /// Changes an element of the mapping.
    ///
    /// # Panic
    ///
    /// Panics if out of range.
    ///
    #[inline]
    pub fn set(&mut self, index: usize, value: D) {
        let slice = self.get_slice();
        slice[index] = value;
    }
}

/// Returns true if reading from a buffer is supported by the backend.
pub fn is_buffer_read_supported<C: ?Sized>(ctxt: &C) -> bool
where
    C: CapabilitiesSource,
{
    // `glGetBufferSubData` is available on desktop GL and WebGL 2, but not on OpenGL ES
    ctxt.get_version().0 == Api::Gl || is_webgl()
}

/// Creates a new buffer.
///
/// # Panic
///
/// Panics if `mem::size_of_val(&data) != size`.
unsafe fn create_buffer<D: ?Sized>(
    mut ctxt: &mut CommandContext<'_>,
    size: usize,
    data: Option<&D>,
    ty: BufferType,
    mode: BufferMode,
) -> Result<(glow::Buffer, bool, bool, Option<*mut raw::c_void>), BufferCreationError>
where
    D: Content,
{
    if !is_buffer_type_supported(ctxt, ty) {
        return Err(BufferCreationError::BufferTypeNotSupported);
    }

    if let Some(data) = data {
        assert!(mem::size_of_val(data) == size);
    }

    // creating the id of the buffer
    let id =
        if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access {
            ctxt.glow.create_named_buffer()
        } else {
            ctxt.glow.create_buffer()
        }
        .expect("glGenBuffers failed");

    // raw pointer to data
    let data_ptr = if let Some(data) = data {
        if size == 0 {
            // if the size is `0` we pass `1` instead (see below),
            ptr::null() // so it's important to have `null` here
        } else {
            data.to_void_ptr()
        }
    } else {
        ptr::null()
    };

    // if the `size` is 0 bytes then we use 1 instead, otherwise nvidia drivers complain
    // note that according to SafeGL the size of the buffer will remain 0
    let size = match size {
        0 => 1,
        a => a,
    };

    // the data as a slice of bytes, for the functions that take one
    let data_bytes = if data_ptr.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(data_ptr as *const u8, size))
    };

    // the flags to use in the case where only `glBufferData` is supported
    let mutable_storage_flags = match mode {
        BufferMode::Persistent | BufferMode::Dynamic => gl::DYNAMIC_DRAW,
        BufferMode::Default | BufferMode::Immutable => gl::STATIC_DRAW,
    };

    // the flags to use if `glBufferStorage` is supported
    let immutable_storage_flags = match mode {
        BufferMode::Default => gl::DYNAMIC_STORAGE_BIT | gl::MAP_READ_BIT | gl::MAP_WRITE_BIT,
        BufferMode::Dynamic => {
            gl::DYNAMIC_STORAGE_BIT | gl::CLIENT_STORAGE_BIT | gl::MAP_READ_BIT | gl::MAP_WRITE_BIT
        }
        BufferMode::Persistent => gl::MAP_PERSISTENT_BIT | gl::MAP_READ_BIT | gl::MAP_WRITE_BIT,
        BufferMode::Immutable => 0,
    };

    // if true, there is a possibility that the buffer won't be modifiable with regular OpenGL
    // function calls
    let could_be_immutable = match mode {
        BufferMode::Default | BufferMode::Dynamic => false,
        BufferMode::Immutable | BufferMode::Persistent => true,
    };

    // will store the actual size of the buffer so that we can compare it with the expected size
    let obtained_size: gl::types::GLint;

    // the value of `immutable` is determined below
    // if true, the buffer won't be modifiable with regular OpenGL function calls
    let immutable: bool;

    // whether the buffer was created with `glBufferStorage`
    let created_with_buffer_storage: bool;

    if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access {
        ctxt.glow
            .named_buffer_storage(id, size as i32, data_bytes, immutable_storage_flags);
        let bind = bind_buffer(&mut ctxt, Some(id), ty);
        obtained_size = ctxt.glow.get_buffer_parameter_i32(bind, glow::BUFFER_SIZE);
        immutable = could_be_immutable;
        created_with_buffer_storage = true;
    } else if ctxt.version >= &Version(Api::Gl, 4, 4)
        || ctxt.extensions.gl_arb_buffer_storage
        || ctxt.extensions.gl_ext_buffer_storage
    {
        // glow falls back to glBufferStorageEXT when the core function isn't loaded
        let bind = bind_buffer(&mut ctxt, Some(id), ty);
        ctxt.glow
            .buffer_storage(bind, size as i32, data_bytes, immutable_storage_flags);
        obtained_size = ctxt.glow.get_buffer_parameter_i32(bind, glow::BUFFER_SIZE);
        immutable = could_be_immutable;
        created_with_buffer_storage = true;
    } else {
        let bind = bind_buffer(&mut ctxt, Some(id), ty);
        match data_bytes {
            Some(data) => ctxt
                .glow
                .buffer_data_u8_slice(bind, data, mutable_storage_flags),
            None => ctxt
                .glow
                .buffer_data_size(bind, size as i32, mutable_storage_flags),
        }
        obtained_size = ctxt.glow.get_buffer_parameter_i32(bind, glow::BUFFER_SIZE);
        immutable = false;
        created_with_buffer_storage = false;
    }

    if size != obtained_size as usize {
        ctxt.glow.delete_buffer(id);

        return Err(BufferCreationError::OutOfMemory);
    }

    let persistent_mapping = if let BufferMode::Persistent = mode {
        if immutable {
            let bind = bind_buffer(&mut ctxt, Some(id), ty);
            let ptr = ctxt.glow.map_buffer_range(
                bind,
                0,
                size as i32,
                glow::MAP_READ_BIT
                    | glow::MAP_WRITE_BIT
                    | glow::MAP_PERSISTENT_BIT
                    | glow::MAP_FLUSH_EXPLICIT_BIT,
            ) as *mut raw::c_void;

            if ptr.is_null() {
                let error = crate::get_gl_error(ctxt);
                panic!("glMapBufferRange returned null (error: {:?})", error);
            }

            Some(ptr)
        } else {
            None
        }
    } else {
        None
    };

    Ok((
        id,
        immutable,
        created_with_buffer_storage,
        persistent_mapping,
    ))
}

/// Returns true if a given buffer type is supported on a platform.
fn is_buffer_type_supported(ctxt: &mut CommandContext<'_>, ty: BufferType) -> bool {
    match ty {
        // SafeGL fails to initialize if they are not supported
        BufferType::ArrayBuffer | BufferType::ElementArrayBuffer => true,

        // core in OpenGL 3.1 and OpenGL ES 3.0
        BufferType::PixelPackBuffer
        | BufferType::PixelUnpackBuffer
        | BufferType::UniformBuffer
        | BufferType::CopyReadBuffer
        | BufferType::CopyWriteBuffer => true,

        BufferType::DrawIndirectBuffer => {
            // TODO: draw indirect buffers are actually supported in OpenGL 4.0 or
            //       with GL_ARB_draw_indirect, but restricting to multidraw is more convenient
            //       for index/multidraw.rs
            ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.extensions.gl_arb_multi_draw_indirect
                || ctxt.extensions.gl_ext_multi_draw_indirect
        }

        #[cfg(feature = "gl46")]
        BufferType::DispatchIndirectBuffer => {
            ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 1)
                || ctxt.extensions.gl_arb_compute_shader
        }

        BufferType::TextureBuffer => {
            ctxt.version.0 == Api::Gl
                || ctxt.extensions.gl_ext_texture_buffer
                || ctxt.extensions.gl_oes_texture_buffer
        }

        #[cfg(feature = "gl46")]
        BufferType::QueryBuffer => {
            ctxt.version >= &Version(Api::Gl, 4, 4)
                || ctxt.extensions.gl_arb_query_buffer_object
                || ctxt.extensions.gl_amd_query_buffer_object
        }

        #[cfg(feature = "gl46")]
        BufferType::ShaderStorageBuffer => {
            ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 1)
                || ctxt.extensions.gl_arb_shader_storage_buffer_object
                || ctxt.extensions.gl_nv_shader_storage_buffer_object
        }

        // core in OpenGL 3.0 and OpenGL ES 3.0
        BufferType::TransformFeedbackBuffer => true,

        #[cfg(feature = "gl46")]
        BufferType::AtomicCounterBuffer => {
            ctxt.version >= &Version(Api::Gl, 4, 2)
                || ctxt.version >= &Version(Api::GlEs, 3, 1)
                || ctxt.extensions.gl_arb_shader_atomic_counters
                || ctxt.extensions.gl_nv_shader_atomic_counters
        }
    }
}

/// Binds a buffer of the given type, and returns the GLenum of the bind point.
/// `id` can be `None`.
///
/// ## Unsafety
///
/// Assumes that the type of buffer is supported by the backend.
/// Binds the cached buffers to their bind points again, after code outside of SafeGL changed
/// them, and forgets the cached indexed bindings, which SafeGL sets before each use.
pub(crate) unsafe fn restore_bindings(ctxt: &mut CommandContext<'_>) {
    let state = &ctxt.state;
    #[cfg_attr(not(feature = "gl46"), allow(unused_mut))]
    let mut bindings = vec![
        (BufferType::ArrayBuffer, state.array_buffer_binding),
        (BufferType::PixelPackBuffer, state.pixel_pack_buffer_binding),
        (
            BufferType::PixelUnpackBuffer,
            state.pixel_unpack_buffer_binding,
        ),
        (BufferType::UniformBuffer, state.uniform_buffer_binding),
        (BufferType::CopyReadBuffer, state.copy_read_buffer_binding),
        (BufferType::CopyWriteBuffer, state.copy_write_buffer_binding),
        (
            BufferType::DrawIndirectBuffer,
            state.draw_indirect_buffer_binding,
        ),
        (BufferType::TextureBuffer, state.texture_buffer_binding),
        (
            BufferType::TransformFeedbackBuffer,
            state.transform_feedback_buffer_binding,
        ),
    ];
    #[cfg(feature = "gl46")]
    bindings.extend([
        (
            BufferType::DispatchIndirectBuffer,
            state.dispatch_indirect_buffer_binding,
        ),
        (BufferType::QueryBuffer, state.query_buffer_binding),
        (
            BufferType::AtomicCounterBuffer,
            state.atomic_counter_buffer_binding,
        ),
        (
            BufferType::ShaderStorageBuffer,
            state.shader_storage_buffer_binding,
        ),
    ]);

    for (ty, buffer) in bindings {
        if is_buffer_type_supported(ctxt, ty) {
            ctxt.glow.bind_buffer(ty.to_glenum(), buffer);
        }
    }

    ctxt.state.indexed_uniform_buffer_bindings.clear();
    ctxt.state.indexed_atomic_counter_buffer_bindings.clear();
    ctxt.state.indexed_shader_storage_buffer_bindings.clear();
    ctxt.state
        .indexed_transform_feedback_buffer_bindings
        .clear();
}

unsafe fn bind_buffer(
    ctxt: &mut CommandContext<'_>,
    id: Option<glow::Buffer>,
    ty: BufferType,
) -> gl::types::GLenum {
    macro_rules! check {
        ($ctxt:expr, $input_id:expr, $input_ty:expr, $check:ident, $state_var:ident) => {
            if $input_ty == BufferType::$check {
                let en = $input_ty.to_glenum();

                if ctxt.state.$state_var != $input_id {
                    ctxt.state.$state_var = $input_id;

                    ctxt.glow.bind_buffer(en, id);
                }

                return en;
            }
        };
    }

    check!(ctxt, id, ty, ArrayBuffer, array_buffer_binding);
    check!(ctxt, id, ty, PixelPackBuffer, pixel_pack_buffer_binding);
    check!(ctxt, id, ty, PixelUnpackBuffer, pixel_unpack_buffer_binding);
    check!(ctxt, id, ty, UniformBuffer, uniform_buffer_binding);
    check!(ctxt, id, ty, CopyReadBuffer, copy_read_buffer_binding);
    check!(ctxt, id, ty, CopyWriteBuffer, copy_write_buffer_binding);
    #[cfg(feature = "gl46")]
    check!(
        ctxt,
        id,
        ty,
        DispatchIndirectBuffer,
        dispatch_indirect_buffer_binding
    );
    check!(
        ctxt,
        id,
        ty,
        DrawIndirectBuffer,
        draw_indirect_buffer_binding
    );
    #[cfg(feature = "gl46")]
    check!(ctxt, id, ty, QueryBuffer, query_buffer_binding);
    check!(ctxt, id, ty, TextureBuffer, texture_buffer_binding);
    #[cfg(feature = "gl46")]
    check!(
        ctxt,
        id,
        ty,
        AtomicCounterBuffer,
        atomic_counter_buffer_binding
    );
    #[cfg(feature = "gl46")]
    check!(
        ctxt,
        id,
        ty,
        ShaderStorageBuffer,
        shader_storage_buffer_binding
    );

    if ty == BufferType::ElementArrayBuffer {
        // TODO: the state if the current buffer is not cached
        VertexAttributesSystem::hijack_current_element_array_buffer(ctxt);

        ctxt.glow.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, id);

        return gl::ELEMENT_ARRAY_BUFFER;
    }

    if ty == BufferType::TransformFeedbackBuffer {
        debug_assert!(ctxt.capabilities.max_indexed_transform_feedback_buffer >= 1);

        // FIXME: pause transform feedback if it is active

        if ctxt.state.transform_feedback_buffer_binding != id {
            ctxt.state.transform_feedback_buffer_binding = id;

            ctxt.glow.bind_buffer(glow::TRANSFORM_FEEDBACK_BUFFER, id);
        }

        return gl::TRANSFORM_FEEDBACK_BUFFER;
    }

    unreachable!();
}

/// Binds a buffer of the given type to an indexed bind point.
///
/// # Panic
///
/// Panics if the buffer type is not indexed.
///
/// # Unsafety
///
/// Assumes that the type of buffer is supported by the backend.
unsafe fn indexed_bind_buffer(
    ctxt: &mut CommandContext<'_>,
    id: glow::Buffer,
    ty: BufferType,
    index: gl::types::GLuint,
    range: Range<usize>,
) {
    let offset = range.start as gl::types::GLintptr;
    let size = (range.end - range.start) as gl::types::GLsizeiptr;

    macro_rules! check {
        ($ctxt:expr, $input_id:expr, $input_ty:expr, $input_index:expr, $check:ident,
         $state_var:ident, $generic_state_var:ident, $max:ident) => {
            if $input_ty == BufferType::$check {
                let en = $input_ty.to_glenum();

                if $input_index >= ctxt.capabilities.$max as gl::types::GLuint {
                    panic!("Indexed buffer out of range");
                }

                if ctxt.state.$state_var.len() <= $input_index as usize {
                    ctxt.state
                        .$state_var
                        .resize_with($input_index as usize + 1, Default::default);
                }

                let unit = &mut ctxt.state.$state_var[$input_index as usize];
                if unit.buffer != Some($input_id) || unit.offset != offset || unit.size != size {
                    unit.buffer = Some($input_id);
                    unit.offset = offset;
                    unit.size = size;

                    ctxt.glow.bind_buffer_range(
                        en,
                        $input_index,
                        Some(id),
                        offset as i32,
                        size as i32,
                    );
                    // glBindBufferRange also changes the generic binding. A cached
                    // indexed bind skips GL entirely, so must leave it unchanged.
                    ctxt.state.$generic_state_var = Some($input_id);
                }

                return;
            }
        };
    }

    check!(
        ctxt,
        id,
        ty,
        index,
        UniformBuffer,
        indexed_uniform_buffer_bindings,
        uniform_buffer_binding,
        max_indexed_uniform_buffer
    );
    check!(
        ctxt,
        id,
        ty,
        index,
        TransformFeedbackBuffer,
        indexed_transform_feedback_buffer_bindings,
        transform_feedback_buffer_binding,
        max_indexed_transform_feedback_buffer
    );
    #[cfg(feature = "gl46")]
    check!(
        ctxt,
        id,
        ty,
        index,
        AtomicCounterBuffer,
        indexed_atomic_counter_buffer_bindings,
        atomic_counter_buffer_binding,
        max_indexed_atomic_counter_buffer
    );
    #[cfg(feature = "gl46")]
    check!(
        ctxt,
        id,
        ty,
        index,
        ShaderStorageBuffer,
        indexed_shader_storage_buffer_bindings,
        shader_storage_buffer_binding,
        max_indexed_shader_storage_buffer
    );

    panic!();
}

/// Copies from a buffer to another.
///
/// # Safety
///
/// The buffer IDs must be valid. The offsets and size must be valid.
///
unsafe fn copy_buffer(
    ctxt: &mut CommandContext<'_>,
    source: glow::Buffer,
    source_offset: usize,
    dest: glow::Buffer,
    dest_offset: usize,
    size: usize,
) -> Result<(), CopyError> {
    fn find_bind_point(
        ctxt: &mut CommandContext<'_>,
        id: glow::Buffer,
    ) -> Option<gl::types::GLenum> {
        if ctxt.state.array_buffer_binding == Some(id) {
            Some(gl::ARRAY_BUFFER)
        } else if ctxt.state.pixel_pack_buffer_binding == Some(id) {
            Some(gl::PIXEL_PACK_BUFFER)
        } else if ctxt.state.pixel_unpack_buffer_binding == Some(id) {
            Some(gl::PIXEL_UNPACK_BUFFER)
        } else if ctxt.state.uniform_buffer_binding == Some(id) {
            Some(gl::UNIFORM_BUFFER)
        } else if ctxt.state.copy_read_buffer_binding == Some(id) {
            Some(gl::COPY_READ_BUFFER)
        } else if ctxt.state.copy_write_buffer_binding == Some(id) {
            Some(gl::COPY_WRITE_BUFFER)
        } else {
            None
        }
    }

    let source_bind_point = match find_bind_point(ctxt, source) {
        Some(p) => p,
        None => {
            // if the source is not bound and the destination is bound to COPY_READ,
            // we bind the source to COPY_WRITE instead, to avoid a state change
            if ctxt.state.copy_read_buffer_binding == Some(dest) {
                bind_buffer(ctxt, Some(source), BufferType::CopyWriteBuffer)
            } else {
                bind_buffer(ctxt, Some(source), BufferType::CopyReadBuffer)
            }
        }
    };

    let dest_bind_point = match find_bind_point(ctxt, dest) {
        Some(p) => p,
        None => {
            // if the source is bound to COPY_WRITE, binding the destination there would
            // replace the source, so we bind the destination to COPY_READ instead
            if source_bind_point == gl::COPY_WRITE_BUFFER {
                bind_buffer(ctxt, Some(dest), BufferType::CopyReadBuffer)
            } else {
                bind_buffer(ctxt, Some(dest), BufferType::CopyWriteBuffer)
            }
        }
    };

    ctxt.glow.copy_buffer_sub_data(
        source_bind_point,
        dest_bind_point,
        source_offset as i32,
        dest_offset as i32,
        size as i32,
    );

    Ok(())
}

/// Destroys a buffer.
unsafe fn destroy_buffer(ctxt: &mut CommandContext<'_>, id: glow::Buffer) {
    // FIXME: uncomment this and move it from Buffer's destructor
    //self.context.vertex_array_objects.purge_buffer(&mut ctxt, id);

    if ctxt.state.array_buffer_binding == Some(id) {
        ctxt.state.array_buffer_binding = None;
    }

    if ctxt.state.pixel_pack_buffer_binding == Some(id) {
        ctxt.state.pixel_pack_buffer_binding = None;
    }

    if ctxt.state.pixel_unpack_buffer_binding == Some(id) {
        ctxt.state.pixel_unpack_buffer_binding = None;
    }

    if ctxt.state.uniform_buffer_binding == Some(id) {
        ctxt.state.uniform_buffer_binding = None;
    }

    if ctxt.state.copy_read_buffer_binding == Some(id) {
        ctxt.state.copy_read_buffer_binding = None;
    }

    if ctxt.state.copy_write_buffer_binding == Some(id) {
        ctxt.state.copy_write_buffer_binding = None;
    }

    if ctxt.state.dispatch_indirect_buffer_binding == Some(id) {
        ctxt.state.dispatch_indirect_buffer_binding = None;
    }

    if ctxt.state.draw_indirect_buffer_binding == Some(id) {
        ctxt.state.draw_indirect_buffer_binding = None;
    }

    if ctxt.state.query_buffer_binding == Some(id) {
        ctxt.state.query_buffer_binding = None;
    }

    if ctxt.state.texture_buffer_binding == Some(id) {
        ctxt.state.texture_buffer_binding = None;
    }

    if ctxt.state.atomic_counter_buffer_binding == Some(id) {
        ctxt.state.atomic_counter_buffer_binding = None;
    }

    if ctxt.state.shader_storage_buffer_binding == Some(id) {
        ctxt.state.shader_storage_buffer_binding = None;
    }

    if ctxt.state.transform_feedback_buffer_binding == Some(id) {
        ctxt.state.transform_feedback_buffer_binding = None;
    }

    for point in ctxt.state.indexed_atomic_counter_buffer_bindings.iter_mut() {
        if point.buffer == Some(id) {
            point.buffer = None;
        }
    }

    for point in ctxt.state.indexed_shader_storage_buffer_bindings.iter_mut() {
        if point.buffer == Some(id) {
            point.buffer = None;
        }
    }

    for point in ctxt.state.indexed_uniform_buffer_bindings.iter_mut() {
        if point.buffer == Some(id) {
            point.buffer = None;
        }
    }

    for point in ctxt
        .state
        .indexed_transform_feedback_buffer_bindings
        .iter_mut()
    {
        // FIXME: end transform feedback if it is active
        if point.buffer == Some(id) {
            point.buffer = None;
        }
    }

    ctxt.glow.delete_buffer(id);
}

/// Flushes a range of a mapped buffer.
unsafe fn flush_range(
    mut ctxt: &mut CommandContext<'_>,
    id: glow::Buffer,
    ty: BufferType,
    range: Range<usize>,
) {
    let bind = bind_buffer(&mut ctxt, Some(id), ty);
    ctxt.glow
        .flush_mapped_buffer_range(bind, range.start as i32, (range.end - range.start) as i32)
}

/// Maps a range of a buffer.
///
/// *Warning*: always passes `GL_MAP_FLUSH_EXPLICIT_BIT`.
unsafe fn map_buffer(
    mut ctxt: &mut CommandContext<'_>,
    id: glow::Buffer,
    ty: BufferType,
    range: Range<usize>,
    read: bool,
    write: bool,
) -> Option<*mut ()> {
    let flags = match (read, write) {
        (true, true) => gl::MAP_FLUSH_EXPLICIT_BIT | gl::MAP_READ_BIT | gl::MAP_WRITE_BIT,
        (true, false) => gl::MAP_READ_BIT,
        (false, true) => gl::MAP_FLUSH_EXPLICIT_BIT | gl::MAP_WRITE_BIT,
        (false, false) => 0,
    };

    let bind = bind_buffer(&mut ctxt, Some(id), ty);
    Some(ctxt.glow.map_buffer_range(
        bind,
        range.start as i32,
        (range.end - range.start) as i32,
        flags,
    ) as *mut ())
}

/// Unmaps a previously-mapped buffer.
///
/// # Safety
///
/// Assumes that the buffer exists, that it is of the right type, and that it is already mapped.
unsafe fn unmap_buffer(mut ctxt: &mut CommandContext<'_>, id: glow::Buffer, ty: BufferType) {
    let bind = bind_buffer(&mut ctxt, Some(id), ty);
    ctxt.glow.unmap_buffer(bind);
}
