//! A buffer is a memory location accessible to the video card.
//!
//! The purpose of buffers is to serve as a space where the GPU can read from or write data to.
//! It can contain a list of vertices, indices, uniform data, etc.
//!
//! # Buffers management in SafeGL
//!
//! There are three levels of abstraction in SafeGL:
//!
//!  - An `Alloc` corresponds to an OpenGL buffer object and is unsafe to use.
//!    This type is not public.
//!  - A `Buffer` wraps around an `Alloc` and provides safety by handling the data type and fences.
//!  - The `VertexBuffer`, `IndexBuffer`, `UniformBuffer`, `PixelBuffer`, etc. types are
//!    abstractions over a `Buffer` indicating their specific purpose. They implement `Deref`
//!    for the `Buffer`. These types are in the `vertex`, `index`, etc. modules.
//!
//! # Unsized types
//!
//! In order to put some data in a buffer, it must implement the `Content` trait. This trait is
//! automatically implemented on all `Sized` types and on slices (like `[u8]`). This means that
//! you can create a `Buffer<Foo>` (if `Foo` is sized) or a `Buffer<[u8]>` for example without
//! worrying about it.
//!
//! However unsized structs don't automatically implement this trait and you must call the
//! `implement_buffer_content!` macro on them. You must then use the `empty_unsized` constructor.
//!
//! ```no_run
//! # use safegl::implement_buffer_content;
//! # use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
//! # fn example<T>(display: safegl::Display<T>) where T: SurfaceTypeTrait + ResizeableSurface {
//! # use std::mem;
//! # use safegl::buffer::{BufferType, BufferMode};
//! struct Data {
//!     data: [f32],        // `[f32]` is unsized, therefore `Data` is unsized too
//! }
//!
//! implement_buffer_content!(Data);    // without this, you can't put `Data` in a SafeGL buffer
//! // every bit pattern is a valid `Data`, so a buffer can be created without data and read
//! safegl::implement_any_bit_pattern!(Data, data);
//!
//! // creates a buffer of 64 bytes, which thus holds 8 f32s
//! let mut buffer = safegl::buffer::Buffer::<Data>::empty_unsized(&display, BufferType::UniformBuffer,
//!                                                               64, BufferMode::Default).unwrap();
//!
//! // you can then write to it like you normally would
//! buffer.map().data[4] = 2.1;
//! # }
//! ```
//!
//! # Sharing a buffer between shader stages
//!
//! The same buffer can be read as vertices by a draw call and written by a compute shader, for
//! example to simulate particles on the GPU. `VertexBuffer<T>` dereferences to `Buffer<[T]>`, and a
//! `&Buffer<[T]>` is a uniform value that binds to a uniform block or a shader storage block when
//! `T` implements `UniformBlock`:
//!
//! ```no_run
//! # use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
//! # #[cfg(any(safegl_gl43, safegl_gles31))]
//! # fn example<T>(display: safegl::Display<T>, compute: safegl::program::ComputeShader)
//! # where T: SurfaceTypeTrait + ResizeableSurface {
//! #[derive(Copy, Clone)]
//! struct Particle {
//!     position: [f32; 4],
//! }
//! safegl::implement_vertex!(Particle, position);
//! safegl::implement_uniform_block!(Particle, position);
//!
//! let particles =
//!     safegl::VertexBuffer::new(&display, &[Particle { position: [0.0; 4] }; 1024]).unwrap();
//!
//! // a compute shader with `buffer Particles { vec4 position[]; };` writes the particles...
//! compute.execute(safegl::uniform! { Particles: &*particles }, 1024 / 64, 1, 1).unwrap();
//! // ...and the next draw calls read them as vertices, after the memory barrier SafeGL inserts
//! # }
//! ```
//!
pub use self::alloc::is_buffer_read_supported;
pub(crate) use self::alloc::restore_bindings;
#[cfg(unix)]
pub(crate) use self::alloc::Alloc;
pub use self::alloc::{CopyError, Mapping, ReadError, ReadMapping, WriteMapping};
#[cfg(safegl_gl43)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl43)))]
pub use self::clear::{ClearBufferError, ClearContent, ClearValue};
pub use self::fences::Inserter;

/// The fences to insert after a draw or compute command, kept inline so that commands that use
/// a few buffers don't allocate.
pub(crate) type FenceInserters<'a> = smallvec::SmallVec<[Inserter<'a>; 8]>;
pub use self::view::{Buffer, BufferAny, BufferMutSlice};
pub use self::view::{BufferAnySlice, BufferSlice};

/// DEPRECATED. Only here for backwards compatibility.
#[deprecated(note = "Only here for backwards compatibility")]
pub use self::view::Buffer as BufferView;
/// DEPRECATED. Only here for backwards compatibility.
#[deprecated(note = "Only here for backwards compatibility")]
pub use self::view::BufferAny as BufferViewAny;
/// DEPRECATED. Only here for backwards compatibility.
#[deprecated(note = "Only here for backwards compatibility")]
pub use self::view::BufferAnySlice as BufferViewAnySlice;
/// DEPRECATED. Only here for backwards compatibility.
#[deprecated(note = "Only here for backwards compatibility")]
pub use self::view::BufferMutSlice as BufferViewMutSlice;
/// DEPRECATED. Only here for backwards compatibility.
#[deprecated(note = "Only here for backwards compatibility")]
pub use self::view::BufferSlice as BufferViewSlice;

use crate::gl;
use std::error::Error;
use std::fmt;
use std::mem;
use std::slice;

mod alloc;
#[cfg(safegl_gl43)]
mod clear;
mod fences;
mod view;

/// Trait for types for which every bit pattern is a valid value, like integers and floats.
///
/// The content of a buffer is undefined after it is created without data (`empty` and similar
/// constructors) or invalidated, and anything the GPU writes into it is only bytes. SafeGL only
/// lets Rust read the content of a buffer, and only creates buffers whose content is undefined,
/// when the type of their content implements this trait. Types like `bool`, `char`, references
/// and `TextureHandle` don't implement it: a `bool` other than 0 or 1 is undefined behavior, and
/// a texture handle that wasn't obtained from a resident texture makes the GPU sample garbage or
/// crash.
///
/// The trait is implemented for the integer and floating-point types, and for arrays, slices and
/// tuples of types that implement it. Use `implement_any_bit_pattern!` to implement it for a
/// struct; the macro checks that every field of the struct implements it.
///
/// # Safety
///
/// Every bit pattern of the size of the type must be a valid value of the type. Padding bytes
/// don't matter.
///
/// # Example
///
/// A buffer of texture handles can't be created without data, so it never holds invalid handles:
///
/// ```compile_fail
/// # fn example(display: safegl::Display<glutin::surface::WindowSurface>) {
/// use safegl::buffer::{Buffer, BufferMode, BufferType};
/// let handles = Buffer::<[safegl::texture::TextureHandle<'static>]>::empty_array(
///     &display, BufferType::UniformBuffer, 16, BufferMode::Default);
/// # }
/// ```
///
/// Nor can a buffer of `bool` be read back:
///
/// ```compile_fail
/// # fn example(display: safegl::Display<glutin::surface::WindowSurface>) {
/// use safegl::buffer::{Buffer, BufferMode, BufferType};
/// let flags = Buffer::new(&display, &[true, false][..], BufferType::ArrayBuffer,
///                         BufferMode::Default).unwrap();
/// let flags = flags.read();
/// # }
/// ```
pub unsafe trait AnyBitPattern {}

macro_rules! impl_any_bit_pattern {
    ($($ty:ty),+) => {
        $(unsafe impl AnyBitPattern for $ty {})+
    };
}

impl_any_bit_pattern!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64, ());

unsafe impl<T: AnyBitPattern, const N: usize> AnyBitPattern for [T; N] {}
unsafe impl<T: AnyBitPattern> AnyBitPattern for [T] {}
unsafe impl<T: ?Sized> AnyBitPattern for std::marker::PhantomData<T> {}
unsafe impl<A: AnyBitPattern> AnyBitPattern for (A,) {}
unsafe impl<A: AnyBitPattern, B: AnyBitPattern> AnyBitPattern for (A, B) {}
unsafe impl<A: AnyBitPattern, B: AnyBitPattern, C: AnyBitPattern> AnyBitPattern for (A, B, C) {}
unsafe impl<A: AnyBitPattern, B: AnyBitPattern, C: AnyBitPattern, D: AnyBitPattern> AnyBitPattern
    for (A, B, C, D)
{
}

/// Trait for types of data that can be put inside buffers.
pub unsafe trait Content {
    /// A type that holds a sized version of the content.
    type Owned;

    /// Prepares an output buffer, then turns this buffer into an `Owned`.
    /// User-provided closure `F` must only write to and not read from `&mut Self`.
    ///
    /// # Safety
    ///
    /// The closure must fill the whole value, and `size` must be suitable for the type.
    unsafe fn read<F, E>(size: usize, _: F) -> Result<Self::Owned, E>
    where
        Self: AnyBitPattern,
        F: FnOnce(&mut Self) -> Result<(), E>;

    /// Returns the size of each element.
    fn get_elements_size() -> usize;

    /// Produces a pointer to the data.
    fn to_void_ptr(&self) -> *const ();

    /// Builds a pointer to this type from a raw pointer.
    fn ref_from_ptr<'a>(ptr: *mut (), size: usize) -> Option<*mut Self>;

    /// Returns true if the size is suitable to store a type like this.
    fn is_size_suitable(_: usize) -> bool;

    /// Returns the alignment that a pointer to this type needs, which the pointers of mapped
    /// buffers don't always have below OpenGL 4.2. The default, 16, is that of the largest
    /// scalar and vector types.
    #[inline]
    fn alignment() -> usize {
        16
    }
}

unsafe impl<T> Content for T
where
    T: Copy,
{
    type Owned = T;

    #[inline]
    unsafe fn read<F, E>(size: usize, f: F) -> Result<T, E>
    where
        T: AnyBitPattern,
        F: FnOnce(&mut T) -> Result<(), E>,
    {
        assert!(size == mem::size_of::<T>());
        // zeroes are a valid `T`, since every bit pattern is
        let mut value = mem::zeroed();
        f(&mut value)?;
        Ok(value)
    }

    #[inline]
    fn get_elements_size() -> usize {
        mem::size_of::<T>()
    }

    #[inline]
    fn to_void_ptr(&self) -> *const () {
        self as *const T as *const ()
    }

    #[inline]
    fn alignment() -> usize {
        mem::align_of::<T>()
    }

    #[inline]
    fn ref_from_ptr<'a>(ptr: *mut (), size: usize) -> Option<*mut T> {
        if size != mem::size_of::<T>() {
            return None;
        }

        Some(ptr as *mut T)
    }

    #[inline]
    fn is_size_suitable(size: usize) -> bool {
        size == mem::size_of::<T>()
    }
}

unsafe impl<T> Content for [T]
where
    T: Copy,
{
    type Owned = Vec<T>;

    #[inline]
    unsafe fn read<F, E>(size: usize, f: F) -> Result<Vec<T>, E>
    where
        [T]: AnyBitPattern,
        F: FnOnce(&mut [T]) -> Result<(), E>,
    {
        assert!(size % mem::size_of::<T>() == 0);
        let len = size / mem::size_of::<T>();
        let mut value = Vec::<T>::with_capacity(len);
        // zeroes are valid elements, since every bit pattern is; the closure must not see
        // uninitialized memory
        std::ptr::write_bytes(value.as_mut_ptr(), 0, len);
        value.set_len(len);
        f(&mut value)?;
        Ok(value)
    }

    #[inline]
    fn get_elements_size() -> usize {
        mem::size_of::<T>()
    }

    #[inline]
    fn to_void_ptr(&self) -> *const () {
        // an empty slice has no first element, but still has a pointer
        self.as_ptr() as *const ()
    }

    #[inline]
    fn alignment() -> usize {
        mem::align_of::<T>()
    }

    #[inline]
    fn ref_from_ptr<'a>(ptr: *mut (), size: usize) -> Option<*mut [T]> {
        if size % mem::size_of::<T>() != 0 {
            return None;
        }

        let ptr = ptr as *mut T;
        let size = size / mem::size_of::<T>();
        Some(unsafe { slice::from_raw_parts_mut(&mut *ptr, size) as *mut [T] })
    }

    #[inline]
    fn is_size_suitable(size: usize) -> bool {
        size % mem::size_of::<T>() == 0
    }
}

/// Error that can happen when creating a buffer.
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum BufferCreationError {
    /// Not enough memory to create the buffer.
    OutOfMemory,

    /// This type of buffer is not supported.
    BufferTypeNotSupported,

    /// The context is lost: nothing can be created in it.
    ContextLost,
}

impl fmt::Display for BufferCreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = match self {
            BufferCreationError::OutOfMemory => "Not enough memory to create the buffer",
            BufferCreationError::BufferTypeNotSupported => "This type of buffer is not supported",
            BufferCreationError::ContextLost => "The context is lost",
        };
        fmt.write_str(desc)
    }
}

impl Error for BufferCreationError {}

/// How the buffer is created.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BufferMode {
    /// This is the default mode suitable for any usage. Will never be slow, will never be fast
    /// either.
    ///
    /// Other modes should always be preferred, but you can use this one if you don't know what
    /// will happen to the buffer.
    ///
    /// # Implementation
    ///
    /// Tries to use `glBufferStorage` with the `GL_DYNAMIC_STORAGE_BIT` flag.
    ///
    /// If this function is not available, falls back to `glBufferData` with `GL_STATIC_DRAW`.
    ///
    Default,

    /// The mode to use when you modify a buffer multiple times per frame. Similar to `Default` in
    /// that it is suitable for most usages.
    ///
    /// Use this if you do a quick succession of modify the buffer, draw, modify, draw, etc. This
    /// is something that you shouldn't do by the way.
    ///
    /// With this mode, the OpenGL driver automatically manages the buffer for us. It will try to
    /// find the most appropriate storage depending on how we use it. It is guaranteed to never be
    /// too slow, but it won't be too fast either.
    ///
    /// # Implementation
    ///
    /// Tries to use `glBufferStorage` with the `GL_DYNAMIC_STORAGE_BIT` and
    /// `GL_CLIENT_STORAGE_BIT` flags.
    ///
    /// If this function is not available, falls back to `glBufferData` with `GL_DYNAMIC_DRAW`.
    ///
    Dynamic,

    /// Optimized for when you modify a buffer exactly once per frame. You can modify it more than
    /// once per frame, but if you modify it too often things will slow down.
    ///
    /// With this mode, SafeGL automatically handles synchronization to prevent the buffer from
    /// being access by both the GPU and the CPU simultaneously. If you try to modify the buffer,
    /// the execution will block until the GPU has finished using it. For this reason, a quick
    /// succession of modifying and drawing using the same buffer will be very slow.
    ///
    /// When using persistent mapping, it is recommended to use triple buffering. This is done by
    /// creating a buffer that has three times the capacity that it would normally have. You modify
    /// and draw the first third, then modify and draw the second third, then the last part, then
    /// go back to the first third, etc.
    ///
    /// # Implementation
    ///
    /// Tries to use `glBufferStorage` with `GL_MAP_PERSISTENT_BIT`. Sync fences are automatically
    /// managed by SafeGL.
    ///
    /// If this function is not available, falls back to `glBufferData` with `GL_DYNAMIC_DRAW`.
    ///
    Persistent,

    /// Optimized when you will never touch the content of the buffer.
    ///
    /// Immutable buffers should be created once and never touched again. Modifying their content
    /// is permitted, but is very slow.
    ///
    /// # Implementation
    ///
    /// Tries to use `glBufferStorage` without any flag. Modifications are done by creating
    /// temporary buffers and making the GPU copy the data from the temporary buffer to the real
    /// one.
    ///
    /// If this function is not available, falls back to `glBufferData` with `GL_STATIC_DRAW`.
    ///
    Immutable,
}

impl Default for BufferMode {
    fn default() -> BufferMode {
        BufferMode::Default
    }
}

/// Type of a buffer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum BufferType {
    ArrayBuffer,
    PixelPackBuffer,
    PixelUnpackBuffer,
    UniformBuffer,
    CopyReadBuffer,
    CopyWriteBuffer,
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    AtomicCounterBuffer,
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    DispatchIndirectBuffer,
    DrawIndirectBuffer,
    #[cfg(safegl_gl44)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
    QueryBuffer,
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    ParameterBuffer,
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    ShaderStorageBuffer,
    TextureBuffer,
    TransformFeedbackBuffer,
    ElementArrayBuffer,
}

impl BufferType {
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            BufferType::ArrayBuffer => gl::ARRAY_BUFFER,
            BufferType::PixelPackBuffer => gl::PIXEL_PACK_BUFFER,
            BufferType::PixelUnpackBuffer => gl::PIXEL_UNPACK_BUFFER,
            BufferType::UniformBuffer => gl::UNIFORM_BUFFER,
            BufferType::CopyReadBuffer => gl::COPY_READ_BUFFER,
            BufferType::CopyWriteBuffer => gl::COPY_WRITE_BUFFER,
            #[cfg(any(safegl_gl42, safegl_gles31))]
            BufferType::AtomicCounterBuffer => gl::ATOMIC_COUNTER_BUFFER,
            #[cfg(any(safegl_gl43, safegl_gles31))]
            BufferType::DispatchIndirectBuffer => gl::DISPATCH_INDIRECT_BUFFER,
            BufferType::DrawIndirectBuffer => gl::DRAW_INDIRECT_BUFFER,
            #[cfg(safegl_gl44)]
            BufferType::QueryBuffer => gl::QUERY_BUFFER,
            #[cfg(safegl_gl46)]
            BufferType::ParameterBuffer => gl::PARAMETER_BUFFER,
            #[cfg(any(safegl_gl43, safegl_gles31))]
            BufferType::ShaderStorageBuffer => gl::SHADER_STORAGE_BUFFER,
            BufferType::TextureBuffer => gl::TEXTURE_BUFFER,
            BufferType::TransformFeedbackBuffer => gl::TRANSFORM_FEEDBACK_BUFFER,
            BufferType::ElementArrayBuffer => gl::ELEMENT_ARRAY_BUFFER,
        }
    }
}
