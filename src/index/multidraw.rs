//! Allows one to draw multiple geometry located in the same buffer.
//!
use std::ops::Deref;
use std::ops::DerefMut;
use std::os::raw;

use crate::backend::Facade;
use crate::buffer::{Buffer, BufferCreationError, BufferMode, BufferType};
#[cfg(safegl_gl46)]
use crate::buffer::BufferAnySlice;
use crate::buffer::{BufferMutSlice, BufferSlice};
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
    /// Numero of the first instance to draw.
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
    /// Numero of the first instance to draw.
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

/// A buffer containing a list of draw commands.
pub struct DrawCommandsNoIndicesBuffer {
    buffer: Buffer<[DrawCommandNoIndices]>,
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

    /// Builds an indices source from this buffer and a primitives type. This indices source can
    /// be passed to the `draw()` function.
    #[inline]
    pub fn with_primitive_type(&self, primitives: PrimitiveType) -> IndicesSource<'_> {
        IndicesSource::MultidrawArray {
            buffer: self.buffer.as_slice_any(),
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
            primitives,
            draw_count: Some(DrawCount {
                buffer: draw_count.as_slice_any(),
                max_draw_count: max_draw_count.min(self.buffer.len()),
            }),
        }
    }
}

impl Deref for DrawCommandsNoIndicesBuffer {
    type Target = Buffer<[DrawCommandNoIndices]>;

    #[inline]
    fn deref(&self) -> &Buffer<[DrawCommandNoIndices]> {
        &self.buffer
    }
}

impl DerefMut for DrawCommandsNoIndicesBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Buffer<[DrawCommandNoIndices]> {
        &mut self.buffer
    }
}

impl<'a> From<&'a DrawCommandsNoIndicesBuffer> for BufferSlice<'a, [DrawCommandNoIndices]> {
    #[inline]
    fn from(b: &'a DrawCommandsNoIndicesBuffer) -> BufferSlice<'a, [DrawCommandNoIndices]> {
        let b: &Buffer<[DrawCommandNoIndices]> = &*b;
        b.as_slice()
    }
}

impl<'a> From<&'a mut DrawCommandsNoIndicesBuffer> for BufferMutSlice<'a, [DrawCommandNoIndices]> {
    #[inline]
    fn from(b: &'a mut DrawCommandsNoIndicesBuffer) -> BufferMutSlice<'a, [DrawCommandNoIndices]> {
        let b: &mut Buffer<[DrawCommandNoIndices]> = &mut *b;
        b.as_mut_slice()
    }
}

/// A buffer containing a list of draw commands.
pub struct DrawCommandsIndicesBuffer {
    buffer: Buffer<[DrawCommandIndices]>,
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

    /// Builds an indices source from this buffer and a primitives type. This indices source can
    /// be passed to the `draw()` function.
    #[inline]
    pub fn with_index_buffer<'a, T>(&'a self, index_buffer: &'a IndexBuffer<T>) -> IndicesSource<'a>
    where
        T: Index,
    {
        IndicesSource::MultidrawElement {
            commands: self.buffer.as_slice_any(),
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

impl Deref for DrawCommandsIndicesBuffer {
    type Target = Buffer<[DrawCommandIndices]>;

    #[inline]
    fn deref(&self) -> &Buffer<[DrawCommandIndices]> {
        &self.buffer
    }
}

impl DerefMut for DrawCommandsIndicesBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Buffer<[DrawCommandIndices]> {
        &mut self.buffer
    }
}

impl<'a> From<&'a DrawCommandsIndicesBuffer> for BufferSlice<'a, [DrawCommandIndices]> {
    #[inline]
    fn from(b: &'a DrawCommandsIndicesBuffer) -> BufferSlice<'a, [DrawCommandIndices]> {
        let b: &Buffer<[DrawCommandIndices]> = &*b;
        b.as_slice()
    }
}

impl<'a> From<&'a mut DrawCommandsIndicesBuffer> for BufferMutSlice<'a, [DrawCommandIndices]> {
    #[inline]
    fn from(b: &'a mut DrawCommandsIndicesBuffer) -> BufferMutSlice<'a, [DrawCommandIndices]> {
        let b: &mut Buffer<[DrawCommandIndices]> = &mut *b;
        b.as_mut_slice()
    }
}
