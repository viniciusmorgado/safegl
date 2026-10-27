use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::mem;

use smallvec::SmallVec;

use crate::buffer::BufferAnySlice;
use crate::program::{Attribute, Program};
use crate::vertex::AttributeType;
use crate::vertex::{AttributeValue, ConstantAttribute};
use crate::vertex::VertexFormat;
use crate::BufferExt;
use crate::GlObject;

use crate::context::CommandContext;
use crate::gl;
use crate::DrawError;
use glow::HasContext;

/// The number of VAOs kept for the same buffers and program bound at different offsets, after
/// which the least recently used one is destroyed, so that drawing from many slices of a buffer
/// doesn't create VAOs without bound.
const MAX_VAOS_PER_BUFFERS: usize = 16;

/// The key of a VAO in the cache: its vertex buffers with their offsets, then its index buffer,
/// its program, and whether it binds its buffers with `glBindVertexBuffer`.
///
/// The VAOs that bind their buffers with `glBindVertexBuffer` change the offsets of their
/// bindings when they are used, so their offsets are all 0 in the key, and their vertex buffers
/// are in the order of their binding points. The other VAOs have the offsets in their attribute
/// pointers, and their buffers are sorted.
///
/// The locations of the attributes given constant values come last: the VAO leaves them
/// disabled.
type VaoKey = (
    SmallVec<[(Option<glow::Buffer>, usize); 3]>,
    glow::Program,
    bool,
    SmallVec<[gl::types::GLint; 2]>,
);

/// A vertex buffer to bind: its buffer, format, offset in bytes, stride and divisor.
type VertexSource = (glow::Buffer, VertexFormat, usize, usize, Option<u32>);

/// Stores and handles vertex attributes.
pub struct VertexAttributesSystem {
    // we maintain a list of VAOs for each vertexbuffer-indexbuffer-program association
    vaos: RefCell<HashMap<VaoKey, VertexArrayObject>>,
}

/// Object allowing one to bind vertex attributes to the current context.
pub struct Binder<'a, 'b, 'c> {
    context: &'b mut CommandContext<'c>,
    program: &'a Program,
    element_array_buffer: Option<BufferAnySlice<'a>>,
    vertex_buffers: SmallVec<[VertexSource; 2]>,
    // the locations and values of the attributes with constant values
    constants: SmallVec<[(gl::types::GLint, AttributeValue); 2]>,
    // a constant value that doesn't have the type of its attribute
    constant_mismatch: bool,
    base_vertex: bool,
    base_instance: bool,
}

impl VertexAttributesSystem {
    /// Builds a new `VertexAttributesSystem`.
    #[inline]
    pub fn new() -> VertexAttributesSystem {
        VertexAttributesSystem {
            vaos: RefCell::new(HashMap::with_hasher(Default::default())),
        }
    }

    /// Starts the process of binding vertex attributes.
    ///
    /// `base_vertex` should be set to true if the backend supports the `glDraw*BaseVertex`
    /// functions. If `base_vertex` is true, then `bind` will return the base vertex to use.
    /// Likewise, `base_instance` tells whether the draw can start from a base instance, which
    /// `bind` returns then.
    #[inline]
    pub fn start<'a, 'b, 'c: 'b>(
        ctxt: &'b mut CommandContext<'c>,
        program: &'a Program,
        indices: Option<BufferAnySlice<'a>>,
        base_vertex: bool,
        base_instance: bool,
    ) -> Binder<'a, 'b, 'c> {
        if let Some(indices) = indices {
            indices.prepare_for_element_array(ctxt);
        }

        Binder {
            context: ctxt,
            program,
            element_array_buffer: indices,
            vertex_buffers: SmallVec::new(),
            constants: SmallVec::new(),
            constant_mismatch: false,
            base_vertex,
            base_instance,
        }
    }

    /// This function *must* be called whenever you destroy a buffer so that the system can
    /// purge its VAOs cache.
    #[inline]
    pub fn purge_buffer(ctxt: &mut CommandContext<'_>, id: glow::Buffer) {
        VertexAttributesSystem::purge_if(ctxt, |&(ref buffers, _, _, _)| {
            buffers.iter().any(|&(b, _)| b == Some(id))
        })
    }

    /// This function *must* be called whenever you destroy a program so that the system can
    /// purge its VAOs cache.
    #[inline]
    pub fn purge_program(ctxt: &mut CommandContext<'_>, program: glow::Program) {
        VertexAttributesSystem::purge_if(ctxt, |&(_, p, _, _)| p == program)
    }

    /// Purges the VAOs cache.
    pub fn purge_all(ctxt: &mut CommandContext<'_>) {
        let vaos = mem::replace(
            &mut *ctxt.vertex_array_objects.vaos.borrow_mut(),
            HashMap::with_hasher(Default::default()),
        );

        for (_, vao) in vaos {
            vao.destroy(ctxt);
        }
    }

    /// Purges the VAOs cache. Contrary to `purge_all`, this function expects the system to be
    /// destroyed soon.
    pub fn cleanup(ctxt: &mut CommandContext<'_>) {
        let vaos = mem::replace(
            &mut *ctxt.vertex_array_objects.vaos.borrow_mut(),
            HashMap::with_hasher(Default::default()),
        );

        for (_, vao) in vaos {
            vao.destroy(ctxt);
        }
    }

    /// Tells the VAOs system that the currently bound element array buffer will change.
    pub fn hijack_current_element_array_buffer(ctxt: &mut CommandContext<'_>) {
        let vaos = ctxt.vertex_array_objects.vaos.borrow_mut();

        for (_, vao) in vaos.iter() {
            if Some(vao.id) == ctxt.state.vertex_array {
                vao.element_array_buffer_hijacked.set(true);
                return;
            }
        }
    }

    /// Purges VAOs that match a certain condition.
    fn purge_if<F>(ctxt: &mut CommandContext<'_>, mut condition: F)
    where
        F: FnMut(&VaoKey) -> bool,
    {
        let mut vaos = ctxt.vertex_array_objects.vaos.borrow_mut();

        let mut keys = Vec::with_capacity(4);
        for key in (*vaos).keys() {
            if condition(key) {
                keys.push(key.clone());
            }
        }

        for key in keys {
            vaos.remove(&key).unwrap().destroy(ctxt);
        }
    }
}

impl<'a, 'b, 'c> Binder<'a, 'b, 'c> {
    /// Adds a buffer to bind as a source of vertices.
    ///
    /// # Parameters
    ///
    /// - `buffer`: The buffer to bind.
    /// - `first`: Offset of the first element of the buffer in number of elements.
    /// - `divisor`: If `Some`, use this value for `glVertexAttribDivisor` (instancing-related).
    #[inline]
    pub fn add(
        mut self,
        buffer: &BufferAnySlice<'_>,
        bindings: VertexFormat,
        divisor: Option<u32>,
    ) -> Binder<'a, 'b, 'c> {
        let offset = buffer.get_offset_bytes();

        buffer.prepare_for_vertex_attrib_array(self.context);

        let (buffer, format, stride) = (buffer.get_id(), bindings, buffer.get_elements_size());

        self.vertex_buffers
            .push((buffer, format, offset, stride, divisor));
        self
    }

    /// Gives an attribute of the program the same value for every vertex. A program without
    /// the attribute ignores it.
    pub fn add_constant(mut self, constant: &ConstantAttribute<'_>) -> Binder<'a, 'b, 'c> {
        let attribute = match self.program.get_attribute(constant.name()) {
            Some(attribute) if attribute.location != -1 => attribute,
            _ => return self,
        };
        if attribute.ty != constant.value().attribute_type() || attribute.size != 1 {
            self.constant_mismatch = true;
        }
        self.constants
            .push((attribute.location, constant.value()));
        self
    }

    /// Finish binding the vertex attributes.
    ///
    /// Returns the base vertex to use when drawing if `base_vertex` was set to true, and the base
    /// instance, which is 0 unless `base_instance` was set to true. Returns an error if the
    /// vertex format doesn't match the attributes of the program.
    pub fn bind(mut self) -> Result<(Option<gl::types::GLint>, u32), DrawError> {
        let ctxt = self.context;

        // the constant values are current values of the context, which the attributes that the
        // VAO leaves disabled read
        if self.constant_mismatch {
            return Err(DrawError::AttributeTypeMismatch);
        }
        for &(location, value) in &self.constants {
            unsafe { set_constant_attribute(ctxt, location as u32, value) };
        }
        let mut constant_locations: SmallVec<[gl::types::GLint; 2]> =
            self.constants.iter().map(|&(location, _)| location).collect();
        constant_locations.sort_unstable();
        constant_locations.dedup();
        if constant_locations.len() != self.constants.len() {
            return Err(DrawError::AttributeProvidedTwice);
        }

        // VAOs are core in OpenGL 3.0 and OpenGL ES 3.0

        // finding the base vertex
        let base_vertex = if self.base_vertex {
            Some(
                self.vertex_buffers
                    .iter()
                    .filter(|&&(_, _, _, _, div)| div.is_none())
                    .map(|&(_, _, off, stride, _)| off / stride)
                    .min()
                    .unwrap_or(0),
            )
        } else {
            None
        };

        // removing the offset corresponding to the base vertex, which only applies to the
        // per-vertex buffers
        if let Some(base_vertex) = base_vertex {
            for &mut (_, _, ref mut off, stride, div) in self.vertex_buffers.iter_mut() {
                if div.is_none() {
                    *off -= base_vertex * stride;
                }
            }
        }

        // the base instance does the same for the per-instance buffers: OpenGL adds it to the
        // element of every instance, whatever the divisor
        let base_instance = if self.base_instance {
            self.vertex_buffers
                .iter()
                .filter(|&&(_, _, _, _, div)| div.is_some())
                .map(|&(_, _, off, stride, _)| off / stride)
                .min()
                .unwrap_or(0)
        } else {
            0
        };
        if base_instance != 0 {
            for &mut (_, _, ref mut off, stride, div) in self.vertex_buffers.iter_mut() {
                if div.is_some() {
                    *off -= base_instance * stride;
                }
            }
        }
        let base_instance = base_instance as u32;

        let separate_bindings = uses_vertex_bindings(ctxt, self.program, &self.vertex_buffers);

        let mut buffers_list: SmallVec<[_; 3]> = self
            .vertex_buffers
            .iter()
            .map(|&(v, _, o, _, _)| (Some(v), if separate_bindings { 0 } else { o }))
            .collect();
        // the binding points follow the order of the buffers
        if !separate_bindings {
            buffers_list.sort_unstable();
        }
        buffers_list.push((self.element_array_buffer.map(|b| b.get_id()), 0));

        let program_id = self.program.get_id();
        let key = (
            buffers_list,
            program_id,
            separate_bindings,
            constant_locations,
        );

        // trying to find an existing VAO in the cache
        let now = ctxt.state.next_draw_call_id;
        if let Some(value) = ctxt.vertex_array_objects.vaos.borrow_mut().get(&key) {
            value.last_used.set(now);
            value.bind(ctxt);
            if separate_bindings {
                value.bind_vertex_buffers(ctxt, &self.vertex_buffers);
            }
            return Ok((base_vertex.map(|v| v as gl::types::GLint), base_instance));
        }

        // if not found, building a new one
        let new_vao = unsafe {
            VertexArrayObject::new(
                ctxt,
                &self.vertex_buffers,
                &key.3,
                self.element_array_buffer,
                self.program,
                separate_bindings,
            )?
        };
        new_vao.last_used.set(now);

        // destroying the least recently used VAO of the same buffers and program bound at other
        // offsets, if there are too many; the VAOs with separate bindings don't depend on the
        // offsets
        let least_recently_used = if separate_bindings {
            None
        } else {
            let vaos = ctxt.vertex_array_objects.vaos.borrow();
            let same_buffers = vaos.iter().filter(|((buffers, program, separate, _), _)| {
                *program == program_id
                    && !*separate
                    && buffers.len() == key.0.len()
                    && buffers.iter().zip(&key.0).all(|(a, b)| a.0 == b.0)
            });
            if same_buffers.clone().count() >= MAX_VAOS_PER_BUFFERS {
                same_buffers
                    .min_by_key(|(_, vao)| vao.last_used.get())
                    .map(|(key, _)| key.clone())
            } else {
                None
            }
        };
        if let Some(key) = least_recently_used {
            let vao = ctxt.vertex_array_objects.vaos.borrow_mut().remove(&key);
            if let Some(vao) = vao {
                vao.destroy(ctxt);
            }
        }

        new_vao.bind(ctxt);
        ctxt.vertex_array_objects
            .vaos
            .borrow_mut()
            .insert(key, new_vao);

        Ok((base_vertex.map(|v| v as gl::types::GLint), base_instance))
    }
}

/// Returns true if the vertex buffers can be bound with `glBindVertexBuffer` and their formats
/// set with `glVertexAttribFormat` (OpenGL 4.3, OpenGL ES 3.1 or `GL_ARB_vertex_attrib_binding`),
/// which is within the limits of these functions.
fn uses_vertex_bindings(
    ctxt: &CommandContext<'_>,
    program: &Program,
    vertex_buffers: &[VertexSource],
) -> bool {
    let caps = ctxt.capabilities;
    let (bindings, relative_offset) = match (
        caps.max_vertex_attrib_bindings,
        caps.max_vertex_attrib_relative_offset,
    ) {
        (Some(bindings), Some(relative_offset)) => (bindings, relative_offset),
        _ => return false,
    };

    vertex_buffers.len() <= bindings.max(0) as usize
        && vertex_buffers.iter().all(|(_, format, _, stride, _)| {
            caps.max_vertex_attrib_stride
                .is_none_or(|max| *stride <= max.max(0) as usize)
                && attribute_columns(program, format)
                    .iter()
                    .all(|column| column.offset <= relative_offset.max(0) as usize)
        })
}

/// Stores informations about how to bind a vertex buffer, an index buffer and a program.
struct VertexArrayObject {
    id: glow::VertexArray,
    destroyed: bool,
    element_array_buffer: Option<glow::Buffer>,
    element_array_buffer_hijacked: Cell<bool>,
    // the draw call that used it last
    last_used: Cell<u64>,
    // with separate bindings, the offset and stride bound to each binding point
    vertex_bindings: RefCell<SmallVec<[(usize, usize); 2]>>,
}

impl VertexArrayObject {
    /// Builds a new `VertexArrayObject`.
    ///
    /// The vertex buffer, index buffer and program must not outlive the
    /// VAO, and the VB & program attributes must not change.
    unsafe fn new(
        mut ctxt: &mut CommandContext<'_>,
        vertex_buffers: &[VertexSource],
        constant_locations: &[gl::types::GLint],
        index_buffer: Option<BufferAnySlice<'_>>,
        program: &Program,
        separate_bindings: bool,
    ) -> Result<VertexArrayObject, DrawError> {
        // checking the attributes types
        for &(_, ref bindings, _, _, _) in vertex_buffers {
            for &(ref name, _, location, ty, _) in bindings.iter() {
                let attribute = match find_attribute(program, name, location) {
                    Some(attribute) => attribute,
                    None => continue,
                };

                if ty.get_num_components() != attribute.ty.get_num_components()
                    || attribute.size != 1
                    || (is_bgra(ty) && attribute.ty != AttributeType::F32F32F32F32)
                {
                    return Err(DrawError::AttributeTypeMismatch);
                }
            }
        }

        // checking for duplicate attribute locations
        for &(_, ref bindings, _, _, _) in vertex_buffers {
            for (i, bi) in bindings.iter().enumerate() {
                for (o, bo) in bindings.iter().enumerate() {
                    if i != o && bi.2 == bo.2 && bi.2 != -1 {
                        panic!("The program attribute `{}` has the same binding location as program attribute `{}` (binding location {})",
                               bi.0, bo.0, bi.2)
                    }
                }
            }
        }

        // checking for attributes that a buffer provides and that have a constant value
        for &(_, ref bindings, _, _, _) in vertex_buffers {
            for &(ref name, _, location, _, _) in bindings.iter() {
                if let Some(attribute) = find_attribute(program, name, location) {
                    if constant_locations.contains(&attribute.location) {
                        return Err(DrawError::AttributeProvidedTwice);
                    }
                }
            }
        }

        // checking for missing attributes
        for (&ref name, attribute) in program.attributes() {
            let mut found = constant_locations.contains(&attribute.location);
            for &(_, ref bindings, _, _, _) in vertex_buffers {
                if bindings.iter().any(|&(ref n, _, location, _, _)| {
                    (location != -1 && location == attribute.location) || n == name
                }) {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(DrawError::AttributeMissing);
            }
        }

        // TODO: check for collisions between the vertices sources

        // building the VAO
        let id = ctxt
            .glow
            .create_vertex_array()
            .expect("glGenVertexArrays failed");

        // we don't use DSA as we're going to make multiple calls for this VAO
        // and we're likely going to use the VAO right after it's been created
        bind_vao(&mut ctxt, Some(id));

        // binding index buffer
        if let Some(index_buffer) = index_buffer {
            index_buffer.bind_to_element_array(&mut ctxt);
        }

        let mut vertex_bindings = SmallVec::new();
        for (index, &(vertex_buffer, ref bindings, offset, stride, divisor)) in
            vertex_buffers.iter().enumerate()
        {
            if separate_bindings {
                let index = index as u32;
                set_attribute_formats(ctxt, program, bindings, index);
                ctxt.glow.vertex_binding_divisor(index, divisor.unwrap_or(0));
                ctxt.glow
                    .bind_vertex_buffer(index, Some(vertex_buffer), offset as i32, stride as i32);
                vertex_bindings.push((offset, stride));
            } else {
                bind_attribute(
                    ctxt,
                    program,
                    vertex_buffer,
                    bindings,
                    offset,
                    stride,
                    divisor,
                );
            }
        }

        Ok(VertexArrayObject {
            id,
            destroyed: false,
            element_array_buffer: index_buffer.map(|b| b.get_id()),
            element_array_buffer_hijacked: Cell::new(false),
            last_used: Cell::new(0),
            vertex_bindings: RefCell::new(vertex_bindings),
        })
    }

    /// Sets this VAO as the current VAO.
    fn bind(&self, ctxt: &mut CommandContext<'_>) {
        unsafe {
            bind_vao(ctxt, Some(self.id));

            if self.element_array_buffer_hijacked.get() {
                // TODO: use a proper function
                ctxt.glow
                    .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, self.element_array_buffer);

                self.element_array_buffer_hijacked.set(false);
            }
        }
    }

    /// Binds the vertex buffers of this VAO, which must be the current one and use separate
    /// bindings, at their offsets, where they changed.
    fn bind_vertex_buffers(&self, ctxt: &mut CommandContext<'_>, vertex_buffers: &[VertexSource]) {
        let mut bound = self.vertex_bindings.borrow_mut();
        for (index, &(vertex_buffer, _, offset, stride, _)) in vertex_buffers.iter().enumerate() {
            if bound[index] != (offset, stride) {
                unsafe {
                    ctxt.glow.bind_vertex_buffer(
                        index as u32,
                        Some(vertex_buffer),
                        offset as i32,
                        stride as i32,
                    );
                }
                bound[index] = (offset, stride);
            }
        }
    }

    /// Must be called to destroy the VAO (otherwise its destructor will panic as a safety
    /// measure).
    fn destroy(mut self, ctxt: &mut CommandContext<'_>) {
        self.destroyed = true;

        // unbinding
        if ctxt.state.vertex_array == Some(self.id) {
            ctxt.state.vertex_array = None;
        }

        // deleting
        unsafe { ctxt.glow.delete_vertex_array(self.id) };
    }
}

impl Drop for VertexArrayObject {
    #[inline]
    fn drop(&mut self) {
        assert!(self.destroyed);
    }
}

impl GlObject for VertexArrayObject {
    type Id = glow::VertexArray;

    #[inline]
    fn get_id(&self) -> glow::VertexArray {
        self.id
    }
}

fn vertex_binding_type_to_gl(
    ty: AttributeType,
) -> (gl::types::GLenum, gl::types::GLint, gl::types::GLint) {
    match ty {
        AttributeType::I8 => (gl::BYTE, 1, 1),
        AttributeType::I8I8 => (gl::BYTE, 2, 1),
        AttributeType::I8I8I8 => (gl::BYTE, 3, 1),
        AttributeType::I8I8I8I8 => (gl::BYTE, 4, 1),
        AttributeType::U8 => (gl::UNSIGNED_BYTE, 1, 1),
        AttributeType::U8U8 => (gl::UNSIGNED_BYTE, 2, 1),
        AttributeType::U8U8U8 => (gl::UNSIGNED_BYTE, 3, 1),
        AttributeType::U8U8U8U8 => (gl::UNSIGNED_BYTE, 4, 1),
        AttributeType::I16 => (gl::SHORT, 1, 1),
        AttributeType::I16I16 => (gl::SHORT, 2, 1),
        AttributeType::I16I16I16 => (gl::SHORT, 3, 1),
        AttributeType::I16I16I16I16 => (gl::SHORT, 4, 1),
        AttributeType::U16 => (gl::UNSIGNED_SHORT, 1, 1),
        AttributeType::U16U16 => (gl::UNSIGNED_SHORT, 2, 1),
        AttributeType::U16U16U16 => (gl::UNSIGNED_SHORT, 3, 1),
        AttributeType::U16U16U16U16 => (gl::UNSIGNED_SHORT, 4, 1),
        AttributeType::I32 => (gl::INT, 1, 1),
        AttributeType::I32I32 => (gl::INT, 2, 1),
        AttributeType::I32I32I32 => (gl::INT, 3, 1),
        AttributeType::I32I32I32I32 => (gl::INT, 4, 1),
        AttributeType::U32 => (gl::UNSIGNED_INT, 1, 1),
        AttributeType::U32U32 => (gl::UNSIGNED_INT, 2, 1),
        AttributeType::U32U32U32 => (gl::UNSIGNED_INT, 3, 1),
        AttributeType::U32U32U32U32 => (gl::UNSIGNED_INT, 4, 1),
        #[cfg(safegl_desktop)]
        AttributeType::I64 => (gl::INT64_NV, 1, 1),
        #[cfg(safegl_desktop)]
        AttributeType::I64I64 => (gl::INT64_NV, 2, 1),
        #[cfg(safegl_desktop)]
        AttributeType::I64I64I64 => (gl::INT64_NV, 3, 1),
        #[cfg(safegl_desktop)]
        AttributeType::I64I64I64I64 => (gl::INT64_NV, 4, 1),
        #[cfg(safegl_desktop)]
        AttributeType::U64 => (gl::UNSIGNED_INT64_NV, 1, 1),
        #[cfg(safegl_desktop)]
        AttributeType::U64U64 => (gl::UNSIGNED_INT64_NV, 2, 1),
        #[cfg(safegl_desktop)]
        AttributeType::U64U64U64 => (gl::UNSIGNED_INT64_NV, 3, 1),
        #[cfg(safegl_desktop)]
        AttributeType::U64U64U64U64 => (gl::UNSIGNED_INT64_NV, 4, 1),
        AttributeType::F16 => (gl::HALF_FLOAT, 1, 1),
        AttributeType::F16F16 => (gl::HALF_FLOAT, 2, 1),
        AttributeType::F16F16F16 => (gl::HALF_FLOAT, 3, 1),
        AttributeType::F16F16F16F16 => (gl::HALF_FLOAT, 4, 1),
        AttributeType::F16x2x2 => (gl::HALF_FLOAT, 2, 2),
        AttributeType::F16x2x3 => (gl::HALF_FLOAT, 2, 3),
        AttributeType::F16x2x4 => (gl::HALF_FLOAT, 2, 4),
        AttributeType::F16x3x2 => (gl::HALF_FLOAT, 3, 2),
        AttributeType::F16x3x3 => (gl::HALF_FLOAT, 3, 3),
        AttributeType::F16x3x4 => (gl::HALF_FLOAT, 3, 4),
        AttributeType::F16x4x2 => (gl::HALF_FLOAT, 4, 2),
        AttributeType::F16x4x3 => (gl::HALF_FLOAT, 4, 3),
        AttributeType::F16x4x4 => (gl::HALF_FLOAT, 4, 4),
        AttributeType::F32 => (gl::FLOAT, 1, 1),
        AttributeType::F32F32 => (gl::FLOAT, 2, 1),
        AttributeType::F32F32F32 => (gl::FLOAT, 3, 1),
        AttributeType::F32F32F32F32 => (gl::FLOAT, 4, 1),
        AttributeType::F32x2x2 => (gl::FLOAT, 2, 2),
        AttributeType::F32x2x3 => (gl::FLOAT, 2, 3),
        AttributeType::F32x2x4 => (gl::FLOAT, 2, 4),
        AttributeType::F32x3x2 => (gl::FLOAT, 3, 2),
        AttributeType::F32x3x3 => (gl::FLOAT, 3, 3),
        AttributeType::F32x3x4 => (gl::FLOAT, 3, 4),
        AttributeType::F32x4x2 => (gl::FLOAT, 4, 2),
        AttributeType::F32x4x3 => (gl::FLOAT, 4, 3),
        AttributeType::F32x4x4 => (gl::FLOAT, 4, 4),
        #[cfg(safegl_desktop)]
        AttributeType::F64 => (gl::DOUBLE, 1, 1),
        #[cfg(safegl_desktop)]
        AttributeType::F64F64 => (gl::DOUBLE, 2, 1),
        #[cfg(safegl_desktop)]
        AttributeType::F64F64F64 => (gl::DOUBLE, 3, 1),
        #[cfg(safegl_desktop)]
        AttributeType::F64F64F64F64 => (gl::DOUBLE, 4, 1),
        #[cfg(safegl_desktop)]
        AttributeType::F64x2x2 => (gl::DOUBLE, 2, 2),
        #[cfg(safegl_desktop)]
        AttributeType::F64x2x3 => (gl::DOUBLE, 2, 3),
        #[cfg(safegl_desktop)]
        AttributeType::F64x2x4 => (gl::DOUBLE, 2, 4),
        #[cfg(safegl_desktop)]
        AttributeType::F64x3x2 => (gl::DOUBLE, 3, 2),
        #[cfg(safegl_desktop)]
        AttributeType::F64x3x3 => (gl::DOUBLE, 3, 3),
        #[cfg(safegl_desktop)]
        AttributeType::F64x3x4 => (gl::DOUBLE, 3, 4),
        #[cfg(safegl_desktop)]
        AttributeType::F64x4x2 => (gl::DOUBLE, 4, 2),
        #[cfg(safegl_desktop)]
        AttributeType::F64x4x3 => (gl::DOUBLE, 4, 3),
        #[cfg(safegl_desktop)]
        AttributeType::F64x4x4 => (gl::DOUBLE, 4, 4),
        AttributeType::I2I10I10I10Reversed => (gl::INT_2_10_10_10_REV, 4, 1),
        AttributeType::U2U10U10U10Reversed => (gl::UNSIGNED_INT_2_10_10_10_REV, 4, 1),
        #[cfg(safegl_desktop)]
        AttributeType::F10F11F11UnsignedIntReversed => (gl::UNSIGNED_INT_10F_11F_11F_REV, 3, 1),
        #[cfg(any(safegl_desktop, safegl_gles30))]
        AttributeType::FixedFloatI16U16 => (gl::FIXED, 1, 1),
        // `GL_BGRA` takes the place of the number of components
        #[cfg(safegl_desktop)]
        AttributeType::U8U8U8U8Bgra => (gl::UNSIGNED_BYTE, gl::BGRA as gl::types::GLint, 1),
        #[cfg(safegl_desktop)]
        AttributeType::I2I10I10I10ReversedBgra => {
            (gl::INT_2_10_10_10_REV, gl::BGRA as gl::types::GLint, 1)
        }
        #[cfg(safegl_desktop)]
        AttributeType::U2U10U10U10ReversedBgra => {
            (gl::UNSIGNED_INT_2_10_10_10_REV, gl::BGRA as gl::types::GLint, 1)
        }
    }
}

/// Returns true if `ty` is one of the `GL_BGRA` types, which OpenGL always normalizes and
/// only feeds to `vec4`s.
fn is_bgra(ty: AttributeType) -> bool {
    #[cfg(safegl_desktop)]
    if let AttributeType::U8U8U8U8Bgra
    | AttributeType::I2I10I10I10ReversedBgra
    | AttributeType::U2U10U10U10ReversedBgra = ty
    {
        return true;
    }
    let _ = ty;
    false
}

/// Binds the vertex array object as the current one. Unbinds if `None` is passed.
fn bind_vao(ctxt: &mut CommandContext<'_>, vao_id: Option<glow::VertexArray>) {
    if ctxt.state.vertex_array != vao_id {
        unsafe { ctxt.glow.bind_vertex_array(vao_id) };

        ctxt.state.vertex_array = vao_id;
    }
}

/// Returns the number of bytes of a component of type `data_type`.
fn component_size(data_type: gl::types::GLenum) -> usize {
    match data_type {
        gl::BYTE | gl::UNSIGNED_BYTE => 1,
        gl::SHORT | gl::UNSIGNED_SHORT | gl::HALF_FLOAT => 2,
        gl::DOUBLE | gl::INT64_NV | gl::UNSIGNED_INT64_NV => 8,
        _ => 4,
    }
}

/// Returns the attribute of the program that a member of a vertex format feeds: the one at its
/// location, or without location, the one with its name.
fn find_attribute<'p>(program: &'p Program, name: &str, location: i32) -> Option<&'p Attribute> {
    match location {
        -1 => program.get_attribute(name),
        _ => program
            .attributes()
            .find(|(_, a)| a.location == location)
            .map(|(_, a)| a),
    }
}

/// Sets the current value of the attribute at `location`, which it has when its array is
/// disabled, unless it already has it.
unsafe fn set_constant_attribute(
    ctxt: &mut CommandContext<'_>,
    location: u32,
    value: AttributeValue,
) {
    let index = location as usize;
    if ctxt.state.vertex_attrib_values.get(index).copied().flatten() == Some(value) {
        return;
    }
    apply_constant_attribute(ctxt, location, value);
    if ctxt.state.vertex_attrib_values.len() <= index {
        ctxt.state.vertex_attrib_values.resize(index + 1, None);
    }
    ctxt.state.vertex_attrib_values[index] = Some(value);
}

/// Calls the `glVertexAttrib*` function of `value`.
pub(crate) unsafe fn apply_constant_attribute(
    ctxt: &CommandContext<'_>,
    location: u32,
    value: AttributeValue,
) {
    match value {
        AttributeValue::Float(v, _) => {
            ctxt.glow
                .vertex_attrib_4_f32(location, v[0], v[1], v[2], v[3])
        }
        AttributeValue::Int(v, _) => {
            ctxt.glow
                .vertex_attrib_4_i32(location, v[0], v[1], v[2], v[3])
        }
        AttributeValue::UnsignedInt(v, _) => {
            ctxt.glow
                .vertex_attrib_4_u32(location, v[0], v[1], v[2], v[3])
        }
        #[cfg(safegl_gl41)]
        AttributeValue::Double(v, _) => {
            ctxt.extra
                .VertexAttribL4d(location, v[0], v[1], v[2], v[3])
        }
    }
}

/// How the shader reads the values of an attribute, which chooses the function that sets its
/// format.
#[derive(Copy, Clone)]
enum ColumnKind {
    /// Converted to floats, normalized if true.
    Float(bool),
    /// Integers.
    Integer,
    /// 64-bit values.
    Long,
}

/// A column of an attribute in a vertex: a matrix has one per column, at consecutive locations.
struct Column {
    location: u32,
    components: gl::types::GLint,
    data_type: gl::types::GLenum,
    kind: ColumnKind,
    // the offset in the vertex
    offset: usize,
}

/// Returns the columns of the attributes of `bindings` that the program uses.
fn attribute_columns(program: &Program, bindings: &VertexFormat) -> SmallVec<[Column; 8]> {
    let mut columns = SmallVec::new();
    for &(ref name, offset, location, ty, normalize) in bindings.iter() {
        let attribute = match find_attribute(program, Borrow::<str>::borrow(name), location) {
            Some(attribute) if attribute.location != -1 => attribute,
            _ => continue,
        };

        let (data_type, components, columns_count) = vertex_binding_type_to_gl(ty);
        let kind = if normalize || is_bgra(ty) {
            ColumnKind::Float(true)
        } else {
            match vertex_binding_type_to_gl(attribute.ty).0 {
                gl::BYTE
                | gl::UNSIGNED_BYTE
                | gl::SHORT
                | gl::UNSIGNED_SHORT
                | gl::INT
                | gl::UNSIGNED_INT => ColumnKind::Integer,
                gl::FLOAT | gl::HALF_FLOAT => ColumnKind::Float(false),
                gl::DOUBLE | gl::INT64_NV | gl::UNSIGNED_INT64_NV => ColumnKind::Long,
                _ => unreachable!(),
            }
        };

        // the columns of a matrix follow each other
        let column_size = components as usize * component_size(data_type);
        for i in 0..columns_count {
            columns.push(Column {
                location: (attribute.location + i) as u32,
                components,
                data_type,
                kind,
                offset: offset + i as usize * column_size,
            });
        }
    }
    columns
}

/// Binds the attributes of a vertex buffer to the current VAO, with `glVertexAttribPointer`.
unsafe fn bind_attribute(
    ctxt: &mut CommandContext<'_>,
    program: &Program,
    vertex_buffer: glow::Buffer,
    bindings: &VertexFormat,
    buffer_offset: usize,
    stride: usize,
    divisor: Option<u32>,
) {
    // glVertexAttribPointer uses the current array buffer
    // TODO: use a proper function
    if ctxt.state.array_buffer_binding != Some(vertex_buffer) {
        ctxt.glow
            .bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
        ctxt.state.array_buffer_binding = Some(vertex_buffer);
    }

    for column in attribute_columns(program, bindings) {
        let pointer = (buffer_offset + column.offset) as i32;
        let stride = stride as i32;
        match column.kind {
            ColumnKind::Float(normalized) => ctxt.glow.vertex_attrib_pointer_f32(
                column.location,
                column.components,
                column.data_type,
                normalized,
                stride,
                pointer,
            ),
            ColumnKind::Integer => ctxt.glow.vertex_attrib_pointer_i32(
                column.location,
                column.components,
                column.data_type,
                stride,
                pointer,
            ),
            ColumnKind::Long => ctxt.glow.vertex_attrib_pointer_f64(
                column.location,
                column.components,
                column.data_type,
                stride,
                pointer,
            ),
        }

        if let Some(divisor) = divisor {
            ctxt.glow.vertex_attrib_divisor(column.location, divisor);
        }
        ctxt.glow.enable_vertex_attrib_array(column.location);
    }
}

/// Sets the formats of the attributes of a vertex buffer in the current VAO, and binds them to
/// the binding point `binding_index`, with `glVertexAttribFormat` and `glVertexAttribBinding`.
unsafe fn set_attribute_formats(
    ctxt: &mut CommandContext<'_>,
    program: &Program,
    bindings: &VertexFormat,
    binding_index: u32,
) {
    for column in attribute_columns(program, bindings) {
        let offset = column.offset as u32;
        match column.kind {
            ColumnKind::Float(normalized) => ctxt.glow.vertex_attrib_format_f32(
                column.location,
                column.components,
                column.data_type,
                normalized,
                offset,
            ),
            ColumnKind::Integer => ctxt.glow.vertex_attrib_format_i32(
                column.location,
                column.components,
                column.data_type,
                offset,
            ),
            ColumnKind::Long => ctxt.glow.vertex_attrib_format_f64(
                column.location,
                column.components,
                column.data_type,
                offset,
            ),
        }

        ctxt.glow.vertex_attrib_binding(column.location, binding_index);
        ctxt.glow.enable_vertex_attrib_array(column.location);
    }
}
