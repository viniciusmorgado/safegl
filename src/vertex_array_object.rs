use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::mem;

use smallvec::SmallVec;

use crate::buffer::BufferAnySlice;
use crate::program::Program;
use crate::vertex::AttributeType;
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

/// Stores and handles vertex attributes.
pub struct VertexAttributesSystem {
    // we maintain a list of VAOs for each vertexbuffer-indexbuffer-program association
    // the key is a (buffers-list-with-offset, program) ; the buffers list must be sorted
    vaos: RefCell<
        HashMap<(SmallVec<[(Option<glow::Buffer>, usize); 3]>, glow::Program), VertexArrayObject>,
    >,
}

/// Object allowing one to bind vertex attributes to the current context.
pub struct Binder<'a, 'b, 'c> {
    context: &'b mut CommandContext<'c>,
    program: &'a Program,
    element_array_buffer: Option<BufferAnySlice<'a>>,
    vertex_buffers: SmallVec<[(glow::Buffer, VertexFormat, usize, usize, Option<u32>); 2]>,
    base_vertex: bool,
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
    #[inline]
    pub fn start<'a, 'b, 'c: 'b>(
        ctxt: &'b mut CommandContext<'c>,
        program: &'a Program,
        indices: Option<BufferAnySlice<'a>>,
        base_vertex: bool,
    ) -> Binder<'a, 'b, 'c> {
        if let Some(indices) = indices {
            indices.prepare_for_element_array(ctxt);
        }

        Binder {
            context: ctxt,
            program,
            element_array_buffer: indices,
            vertex_buffers: SmallVec::new(),
            base_vertex,
        }
    }

    /// This function *must* be called whenever you destroy a buffer so that the system can
    /// purge its VAOs cache.
    #[inline]
    pub fn purge_buffer(ctxt: &mut CommandContext<'_>, id: glow::Buffer) {
        VertexAttributesSystem::purge_if(ctxt, |&(ref buffers, _)| {
            buffers.iter().any(|&(b, _)| b == Some(id))
        })
    }

    /// This function *must* be called whenever you destroy a program so that the system can
    /// purge its VAOs cache.
    #[inline]
    pub fn purge_program(ctxt: &mut CommandContext<'_>, program: glow::Program) {
        VertexAttributesSystem::purge_if(ctxt, |&(_, p)| p == program)
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
        F: FnMut(&(SmallVec<[(Option<glow::Buffer>, usize); 3]>, glow::Program)) -> bool,
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

    /// Finish binding the vertex attributes.
    ///
    /// If `base_vertex` was set to true, returns the base vertex to use when drawing. Returns an
    /// error if the vertex format doesn't match the attributes of the program.
    pub fn bind(mut self) -> Result<Option<gl::types::GLint>, DrawError> {
        let ctxt = self.context;

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

        let mut buffers_list: SmallVec<[_; 3]> = self
            .vertex_buffers
            .iter()
            .map(|&(v, _, o, s, _)| (Some(v), o))
            .collect();
        buffers_list.push((self.element_array_buffer.map(|b| b.get_id()), 0));
        buffers_list.sort_unstable();

        let program_id = self.program.get_id();

        // trying to find an existing VAO in the cache
        let now = ctxt.state.next_draw_call_id;
        if let Some(value) = ctxt
            .vertex_array_objects
            .vaos
            .borrow_mut()
            .get(&(buffers_list.clone(), program_id))
        {
            value.last_used.set(now);
            value.bind(ctxt);
            return Ok(base_vertex.map(|v| v as gl::types::GLint));
        }

        // if not found, building a new one
        let new_vao = unsafe {
            VertexArrayObject::new(
                ctxt,
                &self.vertex_buffers,
                self.element_array_buffer,
                self.program,
            )?
        };
        new_vao.last_used.set(now);

        // destroying the least recently used VAO of the same buffers and program bound at other
        // offsets, if there are too many
        let least_recently_used = {
            let vaos = ctxt.vertex_array_objects.vaos.borrow();
            let same_buffers = vaos.iter().filter(|((buffers, program), _)| {
                *program == program_id
                    && buffers.len() == buffers_list.len()
                    && buffers.iter().zip(&buffers_list).all(|(a, b)| a.0 == b.0)
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
            .insert((buffers_list, program_id), new_vao);

        Ok(base_vertex.map(|v| v as gl::types::GLint))
    }
}

/// Stores informations about how to bind a vertex buffer, an index buffer and a program.
struct VertexArrayObject {
    id: glow::VertexArray,
    destroyed: bool,
    element_array_buffer: Option<glow::Buffer>,
    element_array_buffer_hijacked: Cell<bool>,
    // the draw call that used it last
    last_used: Cell<u64>,
}

impl VertexArrayObject {
    /// Builds a new `VertexArrayObject`.
    ///
    /// The vertex buffer, index buffer and program must not outlive the
    /// VAO, and the VB & program attributes must not change.
    unsafe fn new(
        mut ctxt: &mut CommandContext<'_>,
        vertex_buffers: &[(glow::Buffer, VertexFormat, usize, usize, Option<u32>)],
        index_buffer: Option<BufferAnySlice<'_>>,
        program: &Program,
    ) -> Result<VertexArrayObject, DrawError> {
        // checking the attributes types
        for &(_, ref bindings, _, _, _) in vertex_buffers {
            for &(ref name, _, location, ty, _) in bindings.iter() {
                let attribute = match location {
                    -1 => {
                        // No location specified in Vertex Format. Check name instead
                        match program.get_attribute(Borrow::<str>::borrow(name)) {
                            Some(a) => a,
                            None => continue,
                        }
                    }
                    _ => {
                        match program
                            .attributes()
                            .into_iter()
                            .find(|(_, a)| a.location == location)
                        {
                            Some((_, a)) => a,
                            None => continue,
                        }
                    }
                };

                // Unfortunately internal API used by GLES implementation on Vita
                // assumes all attributes as float4, so we should skip this check for it.
                #[cfg(not(target_os = "vita"))]
                if ty.get_num_components() != attribute.ty.get_num_components()
                    || attribute.size != 1
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

        // checking for missing attributes
        for (&ref name, attribute) in program.attributes() {
            let mut found = false;
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

        for &(vertex_buffer, ref bindings, offset, stride, divisor) in vertex_buffers {
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

        Ok(VertexArrayObject {
            id,
            destroyed: false,
            element_array_buffer: index_buffer.map(|b| b.get_id()),
            element_array_buffer_hijacked: Cell::new(false),
            last_used: Cell::new(0),
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
        AttributeType::I64 => (gl::INT64_NV, 1, 1),
        AttributeType::I64I64 => (gl::INT64_NV, 2, 1),
        AttributeType::I64I64I64 => (gl::INT64_NV, 3, 1),
        AttributeType::I64I64I64I64 => (gl::INT64_NV, 4, 1),
        AttributeType::U64 => (gl::UNSIGNED_INT64_NV, 1, 1),
        AttributeType::U64U64 => (gl::UNSIGNED_INT64_NV, 2, 1),
        AttributeType::U64U64U64 => (gl::UNSIGNED_INT64_NV, 3, 1),
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
        AttributeType::F64 => (gl::DOUBLE, 1, 1),
        AttributeType::F64F64 => (gl::DOUBLE, 2, 1),
        AttributeType::F64F64F64 => (gl::DOUBLE, 3, 1),
        AttributeType::F64F64F64F64 => (gl::DOUBLE, 4, 1),
        AttributeType::F64x2x2 => (gl::DOUBLE, 2, 2),
        AttributeType::F64x2x3 => (gl::DOUBLE, 2, 3),
        AttributeType::F64x2x4 => (gl::DOUBLE, 2, 4),
        AttributeType::F64x3x2 => (gl::DOUBLE, 3, 2),
        AttributeType::F64x3x3 => (gl::DOUBLE, 3, 3),
        AttributeType::F64x3x4 => (gl::DOUBLE, 3, 4),
        AttributeType::F64x4x2 => (gl::DOUBLE, 4, 2),
        AttributeType::F64x4x3 => (gl::DOUBLE, 4, 3),
        AttributeType::F64x4x4 => (gl::DOUBLE, 4, 4),
        AttributeType::I2I10I10I10Reversed => (gl::INT_2_10_10_10_REV, 4, 1),
        AttributeType::U2U10U10U10Reversed => (gl::UNSIGNED_INT_2_10_10_10_REV, 4, 1),
        AttributeType::I10I10I10I2 => (gl::INT_10_10_10_2_OES, 4, 1),
        AttributeType::U10U10U10U2 => (gl::UNSIGNED_INT_10_10_10_2_OES, 4, 1),
        AttributeType::F10F11F11UnsignedIntReversed => (gl::UNSIGNED_INT_10F_11F_11F_REV, 3, 1),
        AttributeType::FixedFloatI16U16 => (gl::FIXED, 1, 1),
    }
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

/// Binds an individual attribute to the current VAO.
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

    // binding attributes
    for &(ref name, offset, location, ty, normalize) in bindings.iter() {
        let (data_type, elements_count, instances_count) = vertex_binding_type_to_gl(ty);

        let attribute = match location {
            -1 => {
                // No location specified in Vertex Format. Check name instead
                match program.get_attribute(Borrow::<str>::borrow(name)) {
                    Some(a) => a,
                    None => continue,
                }
            }
            _ => {
                match program
                    .attributes()
                    .into_iter()
                    .find(|(_, a)| a.location == location)
                {
                    Some((_, a)) => a,
                    None => continue,
                }
            }
        };

        if attribute.location != -1 {
            let (attribute_ty, _, _) = vertex_binding_type_to_gl(attribute.ty);
            if normalize {
                // the columns of a matrix follow each other
                let column_size = elements_count as usize * component_size(data_type);
                for i in 0..instances_count {
                    ctxt.glow.vertex_attrib_pointer_f32(
                        (attribute.location + i) as u32,
                        elements_count as gl::types::GLint,
                        data_type,
                        true,
                        stride as i32,
                        (buffer_offset + offset + i as usize * column_size) as i32,
                    )
                }
            } else {
                match attribute_ty {
                    gl::BYTE
                    | gl::UNSIGNED_BYTE
                    | gl::SHORT
                    | gl::UNSIGNED_SHORT
                    | gl::INT
                    | gl::UNSIGNED_INT => ctxt.glow.vertex_attrib_pointer_i32(
                        attribute.location as u32,
                        elements_count as gl::types::GLint,
                        data_type,
                        stride as i32,
                        (buffer_offset + offset) as i32,
                    ),

                    gl::FLOAT => {
                        for i in 0..instances_count {
                            ctxt.glow.vertex_attrib_pointer_f32(
                                (attribute.location + i) as u32,
                                elements_count as gl::types::GLint,
                                data_type,
                                false,
                                stride as i32,
                                (buffer_offset + offset + (i * elements_count * 4) as usize) as i32,
                            )
                        }
                    }
                    gl::HALF_FLOAT => {
                        for i in 0..instances_count {
                            ctxt.glow.vertex_attrib_pointer_f32(
                                (attribute.location + i) as u32,
                                elements_count as gl::types::GLint,
                                data_type,
                                false,
                                stride as i32,
                                (buffer_offset + offset + (i * elements_count * 2) as usize) as i32,
                            )
                        }
                    }

                    gl::DOUBLE | gl::INT64_NV | gl::UNSIGNED_INT64_NV => {
                        for i in 0..instances_count {
                            ctxt.glow.vertex_attrib_pointer_f64(
                                (attribute.location + i) as u32,
                                elements_count as gl::types::GLint,
                                data_type,
                                stride as i32,
                                (buffer_offset + offset + (i * elements_count * 8) as usize) as i32,
                            )
                        }
                    }

                    _ => unreachable!(),
                }
            }

            for i in 0..instances_count {
                if let Some(divisor) = divisor {
                    ctxt.glow
                        .vertex_attrib_divisor((attribute.location + i) as u32, divisor);
                }
                ctxt.glow
                    .enable_vertex_attrib_array((attribute.location + i) as u32);
            }
        }
    }
}
