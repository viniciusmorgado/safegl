//! Allows one to draw multiple geometry located in the same buffer.
//!
use std::ops::Deref;
use std::ops::DerefMut;
use std::os::raw;

use crate::backend::Facade;
use crate::buffer::{Buffer, BufferCreationError, BufferMode, BufferType};
#[cfg(safegl_gl46)]
use crate::buffer::BufferAnySlice;
use crate::buffer::{AnyBitPattern, BufferMutSlice, BufferSlice};
use crate::index::{Index, IndexBuffer, IndicesSource, PrimitiveType};

/// The number of commands that a multidraw draws, which the GPU reads from a `u32` of a buffer
/// when it draws, up to a maximum (`glMultiDraw*IndirectCount`). A compute shader can write it,
/// for example after culling.
///
/// Built by `DrawCommandsNoIndicesBuffer::with_draw_count` and
/// `DrawCommandsIndicesBuffer::with_index_buffer_and_draw_count`. Drawing with it needs OpenGL
/// 4.6 or `GL_ARB_indirect_parameters`, otherwise the draw returns
/// `DrawError::DrawCountNotSupported`.
#[cfg(safegl_gl46)]
#[derive(Copy, Clone)]
pub struct DrawCount<'a> {
    buffer: BufferAnySlice<'a>,
    max_draw_count: usize,
}

#[cfg(safegl_gl46)]
impl<'a> DrawCount<'a> {
    /// The buffer of the number of commands.
    #[inline]
    pub fn buffer(&self) -> BufferAnySlice<'a> {
        self.buffer
    }

    /// The most commands that the draw draws, whatever the number in the buffer.
    #[inline]
    pub fn max_draw_count(&self) -> usize {
        self.max_draw_count
    }
}

/// Represents an element in a list of draw commands.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DrawCommandNoIndices {
    /// Number of vertices to draw.
    pub count: raw::c_uint,
    /// Number of instances to draw. If it's `0`, nothing will be drawn.
    pub instance_count: raw::c_uint,
    /// First vertex to draw in the vertices source.
    pub first_index: raw::c_uint,
    /// Number of the first instance to draw. It must be 0 below OpenGL 4.2 without
    /// `GL_ARB_base_instance` (and on OpenGL ES without `GL_EXT_base_instance`): like the other
    /// values of commands, which the GPU can write, SafeGL doesn't check it.
    pub base_instance: raw::c_uint,
}

implement_uniform_block!(
    DrawCommandNoIndices,
    count,
    instance_count,
    first_index,
    base_instance
);
implement_any_bit_pattern!(
    DrawCommandNoIndices,
    count,
    instance_count,
    first_index,
    base_instance
);

/// Represents an element in a list of draw commands.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DrawCommandIndices {
    /// Number of indices to use in the index buffer.
    pub count: raw::c_uint,
    /// Number of instances to draw. If it's `0`, nothing will be drawn.
    pub instance_count: raw::c_uint,
    /// First index to draw in the index buffer.
    pub first_index: raw::c_uint,
    /// Value to add to each index, which can be negative.
    pub base_vertex: raw::c_int,
    /// Number of the first instance to draw. It must be 0 below OpenGL 4.2 without
    /// `GL_ARB_base_instance` (and on OpenGL ES without `GL_EXT_base_instance`): like the other
    /// values of commands, which the GPU can write, SafeGL doesn't check it.
    pub base_instance: raw::c_uint,
}

implement_uniform_block!(
    DrawCommandIndices,
    count,
    instance_count,
    first_index,
    base_vertex,
    base_instance
);
implement_any_bit_pattern!(
    DrawCommandIndices,
    count,
    instance_count,
    first_index,
    base_vertex,
    base_instance
);

mod sealed {
    pub trait Command {}
    impl Command for super::DrawCommandNoIndices {}
    impl Command for super::DrawCommandIndices {}
}

/// A type that a buffer of draw commands holds: a `DrawCommandNoIndices` or a
/// `DrawCommandIndices` at an offset in it, among other values. The GPU reads the commands with
/// the size of the type as stride.
///
/// Implement it with [`implement_draw_command!`](crate::implement_draw_command), which checks
/// the type and the offset of the command.
///
/// # Safety
///
/// `Self` must contain a value of type `Command` at the offset `OFFSET` in bytes, and its size
/// and `OFFSET` must be multiples of 4.
pub unsafe trait DrawCommandLayout: Copy + AnyBitPattern + 'static {
    /// The type of the command: `DrawCommandNoIndices` or `DrawCommandIndices`.
    type Command: sealed::Command;
    /// The offset in bytes of the command in the type.
    const OFFSET: usize;
}

unsafe impl DrawCommandLayout for DrawCommandNoIndices {
    type Command = DrawCommandNoIndices;
    const OFFSET: usize = 0;
}

unsafe impl DrawCommandLayout for DrawCommandIndices {
    type Command = DrawCommandIndices;
    const OFFSET: usize = 0;
}

/// A buffer of draw commands, or of values of type `C` that contain one (see
/// [`DrawCommandLayout`]).
///
/// Drawing one of its commands with `command` needs OpenGL 4.0 or OpenGL ES 3.1, and drawing
/// all of them with one call needs OpenGL 4.3, `GL_ARB_multi_draw_indirect` or
/// `GL_EXT_multi_draw_indirect` (`DrawError::MultidrawNotSupported` otherwise), except when the
/// buffer has a single command. The GPU reads the commands when it draws, so SafeGL can't check
/// them: their vertices, indices and instances must be in the buffers of the draw.
pub struct DrawCommandsNoIndicesBuffer<C = DrawCommandNoIndices>
where
    C: DrawCommandLayout<Command = DrawCommandNoIndices>,
{
    buffer: Buffer<[C]>,
}

impl DrawCommandsNoIndicesBuffer {
    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsNoIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Default,
        )?;
        Ok(DrawCommandsNoIndicesBuffer { buffer: buf })
    }

    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_dynamic<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsNoIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Dynamic,
        )?;
        Ok(DrawCommandsNoIndicesBuffer { buffer: buf })
    }

    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_persistent<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsNoIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Persistent,
        )?;
        Ok(DrawCommandsNoIndicesBuffer { buffer: buf })
    }

    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_immutable<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsNoIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Immutable,
        )?;
        Ok(DrawCommandsNoIndicesBuffer { buffer: buf })
    }
}

impl<C> DrawCommandsNoIndicesBuffer<C>
where
    C: DrawCommandLayout<Command = DrawCommandNoIndices>,
{
    /// Uses a buffer of values that contain draw commands, at the offset of their
    /// [`DrawCommandLayout`], as a buffer of draw commands.
    #[inline]
    pub fn from_buffer(buffer: Buffer<[C]>) -> DrawCommandsNoIndicesBuffer<C> {
        DrawCommandsNoIndicesBuffer { buffer }
    }

    /// Returns the buffer of the commands.
    #[inline]
    pub fn into_buffer(self) -> Buffer<[C]> {
        self.buffer
    }

    /// Builds an indices source that draws the command at `index`
    /// (`glDrawArraysIndirect`), or returns `None` if the buffer has no command at `index`.
    #[inline]
    pub fn command(&self, index: usize, primitives: PrimitiveType) -> Option<IndicesSource<'_>> {
        let command = self.buffer.slice(index..index + 1)?;
        Some(IndicesSource::MultidrawArray {
            buffer: command.as_slice_any(),
            command_offset: C::OFFSET,
            primitives,
            #[cfg(safegl_gl46)]
            draw_count: None,
        })
    }

    /// Builds an indices source that draws all the commands of the buffer with one call
    /// (`glMultiDrawArraysIndirect`). This indices source can be passed to the `draw()`
    /// function.
    #[cfg(safegl_gl43)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl43)))]
    #[inline]
    pub fn with_primitive_type(&self, primitives: PrimitiveType) -> IndicesSource<'_> {
        IndicesSource::MultidrawArray {
            buffer: self.buffer.as_slice_any(),
            command_offset: C::OFFSET,
            primitives,
            #[cfg(safegl_gl46)]
            draw_count: None,
        }
    }

    /// Like `with_primitive_type`, but the GPU reads the number of commands to draw from
    /// `draw_count` when it draws, and draws at most `max_draw_count` of them, which is at most
    /// the number of commands of this buffer. See [`DrawCount`].
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    #[inline]
    pub fn with_draw_count<'a>(
        &'a self,
        primitives: PrimitiveType,
        draw_count: BufferSlice<'a, u32>,
        max_draw_count: usize,
    ) -> IndicesSource<'a> {
        IndicesSource::MultidrawArray {
            buffer: self.buffer.as_slice_any(),
            command_offset: C::OFFSET,
            primitives,
            draw_count: Some(DrawCount {
                buffer: draw_count.as_slice_any(),
                max_draw_count: max_draw_count.min(self.buffer.len()),
            }),
        }
    }
}

impl<C> Deref for DrawCommandsNoIndicesBuffer<C>
where
    C: DrawCommandLayout<Command = DrawCommandNoIndices>,
{
    type Target = Buffer<[C]>;

    #[inline]
    fn deref(&self) -> &Buffer<[C]> {
        &self.buffer
    }
}

impl<C> DerefMut for DrawCommandsNoIndicesBuffer<C>
where
    C: DrawCommandLayout<Command = DrawCommandNoIndices>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Buffer<[C]> {
        &mut self.buffer
    }
}

impl<'a, C> From<&'a DrawCommandsNoIndicesBuffer<C>> for BufferSlice<'a, [C]>
where
    C: DrawCommandLayout<Command = DrawCommandNoIndices>,
{
    #[inline]
    fn from(b: &'a DrawCommandsNoIndicesBuffer<C>) -> BufferSlice<'a, [C]> {
        let b: &Buffer<[C]> = &*b;
        b.as_slice()
    }
}

impl<'a, C> From<&'a mut DrawCommandsNoIndicesBuffer<C>> for BufferMutSlice<'a, [C]>
where
    C: DrawCommandLayout<Command = DrawCommandNoIndices>,
{
    #[inline]
    fn from(b: &'a mut DrawCommandsNoIndicesBuffer<C>) -> BufferMutSlice<'a, [C]> {
        let b: &mut Buffer<[C]> = &mut *b;
        b.as_mut_slice()
    }
}

/// A buffer of draw commands, or of values of type `C` that contain one (see
/// [`DrawCommandLayout`]).
///
/// Drawing one of its commands with `command` needs OpenGL 4.0 or OpenGL ES 3.1, and drawing
/// all of them with one call needs OpenGL 4.3, `GL_ARB_multi_draw_indirect` or
/// `GL_EXT_multi_draw_indirect` (`DrawError::MultidrawNotSupported` otherwise), except when the
/// buffer has a single command. The GPU reads the commands when it draws, so SafeGL can't check
/// them: their vertices, indices and instances must be in the buffers of the draw.
pub struct DrawCommandsIndicesBuffer<C = DrawCommandIndices>
where
    C: DrawCommandLayout<Command = DrawCommandIndices>,
{
    buffer: Buffer<[C]>,
}

impl DrawCommandsIndicesBuffer {
    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Default,
        )?;
        Ok(DrawCommandsIndicesBuffer { buffer: buf })
    }

    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_dynamic<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Dynamic,
        )?;
        Ok(DrawCommandsIndicesBuffer { buffer: buf })
    }

    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_persistent<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Persistent,
        )?;
        Ok(DrawCommandsIndicesBuffer { buffer: buf })
    }

    /// Builds an empty buffer.
    ///
    /// The parameter indicates the number of elements.
    #[inline]
    pub fn empty_immutable<F: ?Sized>(
        facade: &F,
        elements: usize,
    ) -> Result<DrawCommandsIndicesBuffer, BufferCreationError>
    where
        F: Facade,
    {
        let buf = Buffer::empty_array(
            facade,
            BufferType::DrawIndirectBuffer,
            elements,
            BufferMode::Immutable,
        )?;
        Ok(DrawCommandsIndicesBuffer { buffer: buf })
    }
}

impl<C> DrawCommandsIndicesBuffer<C>
where
    C: DrawCommandLayout<Command = DrawCommandIndices>,
{
    /// Uses a buffer of values that contain draw commands, at the offset of their
    /// [`DrawCommandLayout`], as a buffer of draw commands.
    #[inline]
    pub fn from_buffer(buffer: Buffer<[C]>) -> DrawCommandsIndicesBuffer<C> {
        DrawCommandsIndicesBuffer { buffer }
    }

    /// Returns the buffer of the commands.
    #[inline]
    pub fn into_buffer(self) -> Buffer<[C]> {
        self.buffer
    }

    /// Builds an indices source that draws the command at `index` with the indices of
    /// `index_buffer` (`glDrawElementsIndirect`), or returns `None` if the buffer has no command
    /// at `index`.
    #[inline]
    pub fn command_with_index_buffer<'a, T>(
        &'a self,
        index: usize,
        index_buffer: &'a IndexBuffer<T>,
    ) -> Option<IndicesSource<'a>>
    where
        T: Index,
    {
        let command = self.buffer.slice(index..index + 1)?;
        Some(IndicesSource::MultidrawElement {
            commands: command.as_slice_any(),
            command_offset: C::OFFSET,
            indices: index_buffer.as_slice_any(),
            data_type: index_buffer.get_indices_type(),
            primitives: index_buffer.get_primitives_type(),
            #[cfg(safegl_gl46)]
            draw_count: None,
        })
    }

    /// Builds an indices source that draws all the commands of the buffer with one call
    /// (`glMultiDrawElementsIndirect`), with the indices of `index_buffer`. This indices source
    /// can be passed to the `draw()` function.
    #[cfg(safegl_gl43)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl43)))]
    #[inline]
    pub fn with_index_buffer<'a, T>(&'a self, index_buffer: &'a IndexBuffer<T>) -> IndicesSource<'a>
    where
        T: Index,
    {
        IndicesSource::MultidrawElement {
            commands: self.buffer.as_slice_any(),
            command_offset: C::OFFSET,
            indices: index_buffer.as_slice_any(),
            data_type: index_buffer.get_indices_type(),
            primitives: index_buffer.get_primitives_type(),
            #[cfg(safegl_gl46)]
            draw_count: None,
        }
    }

    /// Like `with_index_buffer`, but the GPU reads the number of commands to draw from
    /// `draw_count` when it draws, and draws at most `max_draw_count` of them, which is at most
    /// the number of commands of this buffer. See [`DrawCount`].
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    #[inline]
    pub fn with_index_buffer_and_draw_count<'a, T>(
        &'a self,
        index_buffer: &'a IndexBuffer<T>,
        draw_count: BufferSlice<'a, u32>,
        max_draw_count: usize,
    ) -> IndicesSource<'a>
    where
        T: Index,
    {
        IndicesSource::MultidrawElement {
            commands: self.buffer.as_slice_any(),
            command_offset: C::OFFSET,
            indices: index_buffer.as_slice_any(),
            data_type: index_buffer.get_indices_type(),
            primitives: index_buffer.get_primitives_type(),
            draw_count: Some(DrawCount {
                buffer: draw_count.as_slice_any(),
                max_draw_count: max_draw_count.min(self.buffer.len()),
            }),
        }
    }
}

impl<C> Deref for DrawCommandsIndicesBuffer<C>
where
    C: DrawCommandLayout<Command = DrawCommandIndices>,
{
    type Target = Buffer<[C]>;

    #[inline]
    fn deref(&self) -> &Buffer<[C]> {
        &self.buffer
    }
}

impl<C> DerefMut for DrawCommandsIndicesBuffer<C>
where
    C: DrawCommandLayout<Command = DrawCommandIndices>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Buffer<[C]> {
        &mut self.buffer
    }
}

impl<'a, C> From<&'a DrawCommandsIndicesBuffer<C>> for BufferSlice<'a, [C]>
where
    C: DrawCommandLayout<Command = DrawCommandIndices>,
{
    #[inline]
    fn from(b: &'a DrawCommandsIndicesBuffer<C>) -> BufferSlice<'a, [C]> {
        let b: &Buffer<[C]> = &*b;
        b.as_slice()
    }
}

impl<'a, C> From<&'a mut DrawCommandsIndicesBuffer<C>> for BufferMutSlice<'a, [C]>
where
    C: DrawCommandLayout<Command = DrawCommandIndices>,
{
    #[inline]
    fn from(b: &'a mut DrawCommandsIndicesBuffer<C>) -> BufferMutSlice<'a, [C]> {
        let b: &mut Buffer<[C]> = &mut *b;
        b.as_mut_slice()
    }
}
