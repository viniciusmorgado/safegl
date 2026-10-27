use crate::gl;
use crate::handles::{RawName, UniformLocations};

use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::ptr;

use fnv::FnvHasher;
use glow::HasContext;

use crate::context::CommandContext;
use crate::version::Api;
use crate::version::Version;

#[cfg(safegl_desktop)]
use crate::program;
use crate::uniforms::UniformType;
use crate::vertex::AttributeType;

/// Information about a uniform (except its name).
#[derive(Debug, Copy, Clone)]
pub struct Uniform {
    /// The location of the uniform.
    ///
    /// This is internal information, you probably don't need to use it.
    pub location: i32,

    /// Type of the uniform.
    pub ty: UniformType,

    /// If it is an array, the number of elements.
    pub size: Option<usize>,

    /// For an image uniform, the image unit that the shader gives it with `layout(binding = N)`,
    /// 0 without one. OpenGL ES can't change it, so SafeGL binds the image to this unit there;
    /// desktop OpenGL can, and SafeGL picks a unit itself. `None` for other uniforms.
    pub binding: Option<i32>,
}

/// Information about a uniform block (except its name).
#[derive(Debug, Clone)]
pub struct UniformBlock {
    /// Identifier of the block.
    ///
    /// This is internal information, you probably don't need to use it.
    pub id: i32,

    /// Initial bind point of the block.
    ///
    /// This is internal information, you probably don't need to use it.
    pub initial_binding: i32,

    /// Size in bytes of the data in the block.
    pub size: usize,

    /// Layout of the block.
    pub layout: BlockLayout,
}

/// Layout of a shader storage buffer or a uniform buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockLayout {
    /// Multiple elements, each having a name.
    Struct {
        /// The list of elements, with `name`/`layout` pairs.
        members: Vec<(String, BlockLayout)>,
    },

    /// A basic element.
    BasicType {
        /// Type of data.
        ty: UniformType,

        /// Offset of this element in bytes from the start of the buffer.
        offset_in_buffer: usize,
    },

    /// A fixed-size array.
    ///
    /// For example:
    ///
    /// ```notrust
    /// uint data[12];
    /// ```
    Array {
        /// Type of data of each element.
        content: Box<BlockLayout>,

        /// Number of elements in the array.
        length: usize,
    },

    /// An array whose size isn't known at compile-time. Can only be used as the last element of
    /// a buffer.
    ///
    /// Its actual size depends on the size of the buffer.
    ///
    /// For example:
    ///
    /// ```notrust
    /// buffer MyBuffer {
    ///     uint data[];
    /// }
    /// ```
    DynamicSizedArray {
        /// Type of data of each element.
        content: Box<BlockLayout>,
    },
}

/// Information about an attribute of a program (except its name).
///
/// Internal struct. Not public.
#[derive(Debug, Copy, Clone)]
pub struct Attribute {
    /// The index of the uniform.
    ///
    /// This is internal information, you probably don't need to use it.
    pub location: i32,

    /// Type of the attribute.
    pub ty: AttributeType,

    /// Number of elements of the attribute.
    pub size: usize,
}

/// Describes the layout of a buffer that can receive transform feedback output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformFeedbackBuffer {
    /// Slot of this buffer.
    ///
    /// This is internal information, you probably don't need to use it.
    pub id: i32,

    /// List of elements inside the buffer.
    pub elements: Vec<TransformFeedbackVarying>,

    /// Size in bytes between two consecutive elements.
    pub stride: usize,
}

/// Describes a varying that is being output with transform feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformFeedbackVarying {
    /// Name of the variable.
    pub name: String,

    /// Number of bytes between the start of the first element and the start of this one.
    pub offset: usize,

    /// Size in bytes of this value.
    pub size: usize,

    /// Type of the value.
    pub ty: AttributeType,
}

/// Type of transform feedback. Only used with the legacy interface.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TransformFeedbackMode {
    /// Each value is interleaved in the same buffer.
    Interleaved,

    /// Each value will go in a separate buffer.
    Separate,
}

/// Type of primitives that is being output by transform feedback.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OutputPrimitives {
    /// Points.
    Points,
    /// Lines.
    Lines,
    /// Triangles.
    Triangles,
    /// Quads.
    Quads,
}

/// Returns a list of uniforms and a list of atomic counters of a program.
pub unsafe fn reflect_uniforms(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> (
    HashMap<String, Uniform, BuildHasherDefault<FnvHasher>>,
    HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>>,
    UniformLocations,
) {
    // number of active uniforms
    let active_uniforms = ctxt
        .glow
        .get_program_parameter_i32(program, glow::ACTIVE_UNIFORMS);

    let query_atomic_counters = ctxt.version >= &Version(Api::Gl, 4, 2)
        || ctxt.version >= &Version(Api::GlEs, 3, 1)
        || (ctxt.extensions.gl_arb_program_interface_query
            && ctxt.extensions.gl_arb_shader_atomic_counters);
    let mut active_atomic_counters: gl::types::GLint = 0;
    if query_atomic_counters {
        active_atomic_counters = ctxt
            .glow
            .get_program_parameter_i32(program, glow::ACTIVE_ATOMIC_COUNTER_BUFFERS);
    }

    let mut locations = UniformLocations::default();

    // the result of this function
    let mut uniforms = HashMap::with_hasher(BuildHasherDefault::<FnvHasher>::default());
    uniforms.reserve((active_uniforms - active_atomic_counters) as usize);

    let mut atomic_counters = HashMap::with_hasher(Default::default());
    atomic_counters.reserve(active_atomic_counters as usize);

    for uniform_id in 0..active_uniforms {
        let glow::ActiveUniform {
            size: data_size,
            utype: data_type,
            name: uniform_name,
        } = ctxt
            .glow
            .get_active_uniform(program, uniform_id as gl::types::GLuint)
            .expect("glGetActiveUniform failed");

        let gl_location = ctxt.glow.get_uniform_location(program, &uniform_name);
        let binding = image_binding(ctxt, program, data_type, gl_location.as_ref());
        let location = locations.register(gl_location);

        if data_type == gl::UNSIGNED_INT_ATOMIC_COUNTER {
            assert!(query_atomic_counters);
            let mut atomic_counter_buffer_bind_point: gl::types::GLint = 0;
            let atomic_counter_id = ctxt.glow.get_active_uniforms_parameter(
                program,
                &[uniform_id as gl::types::GLuint],
                glow::UNIFORM_ATOMIC_COUNTER_BUFFER_INDEX,
            )[0];
            // OpenGL ES has no `glGetActiveAtomicCounterBufferiv`
            if ctxt.version.0 == Api::GlEs
                || ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.extensions.gl_arb_program_interface_query
            {
                ctxt.extra.GetProgramResourceiv(
                    program.raw_name(),
                    gl::ATOMIC_COUNTER_BUFFER,
                    atomic_counter_id as gl::types::GLuint,
                    1,
                    &gl::BUFFER_BINDING,
                    1,
                    ptr::null_mut(),
                    &mut atomic_counter_buffer_bind_point,
                );
            } else {
                ctxt.extra.GetActiveAtomicCounterBufferiv(
                    program.raw_name(),
                    atomic_counter_id as gl::types::GLuint,
                    gl::ATOMIC_COUNTER_BUFFER_BINDING,
                    &mut atomic_counter_buffer_bind_point,
                );
            }
            atomic_counters.insert(
                uniform_name,
                UniformBlock {
                    id: atomic_counter_id,
                    initial_binding: atomic_counter_buffer_bind_point,
                    size: 4,
                    layout: BlockLayout::BasicType {
                        ty: UniformType::UnsignedInt,
                        offset_in_buffer: 0,
                    },
                },
            );
        } else {
            uniforms.insert(
                uniform_name,
                Uniform {
                    location: location as i32,
                    ty: glenum_to_uniform_type(data_type),
                    size: if data_size == 1 {
                        None
                    } else {
                        Some(data_size as usize)
                    },
                    binding,
                },
            );
        }
    }

    // Flatten arrays
    let mut uniforms_flattened = HashMap::with_hasher(Default::default());
    for (name, uniform) in uniforms {
        // If this is a normal non-array element, just move it over
        let Some(name_base) = name.strip_suffix("[0]") else {
            assert!(uniform.size.is_none());
            uniforms_flattened.insert(name, uniform);
            continue;
        };

        // As in OpenGL, the name of the array refers to its first element
        let first = Uniform {
            size: None,
            ..uniform
        };
        uniforms_flattened.insert(name_base.to_owned(), first);
        uniforms_flattened.insert(name.clone(), first);

        // The locations of the other elements aren't necessarily consecutive, and on WebGL they
        // are objects that must be queried one by one
        for i in 1..uniform.size.unwrap_or(1) {
            let element = format!("{}[{}]", name_base, i);
            let gl_location = ctxt.glow.get_uniform_location(program, &element);
            let binding = first.binding.and_then(|_| {
                image_binding(ctxt, program, gl::IMAGE_2D, gl_location.as_ref())
            });
            let location = locations.register(gl_location);
            if location != -1 {
                uniforms_flattened.insert(
                    element,
                    Uniform {
                        location,
                        binding,
                        ..first
                    },
                );
            }
        }
    }

    (uniforms_flattened, atomic_counters, locations)
}

/// Returns the image unit of a uniform of type `data_type`, if it is an image.
unsafe fn image_binding(
    ctxt: &CommandContext<'_>,
    program: glow::Program,
    data_type: gl::types::GLenum,
    location: Option<&glow::UniformLocation>,
) -> Option<i32> {
    if !(gl::IMAGE_1D..=gl::UNSIGNED_INT_IMAGE_2D_MULTISAMPLE_ARRAY).contains(&data_type) {
        return None;
    }
    let mut unit = [0];
    ctxt.glow.get_uniform_i32(program, location?, &mut unit);
    Some(unit[0])
}

pub unsafe fn reflect_attributes(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> HashMap<String, Attribute, BuildHasherDefault<FnvHasher>> {
    // number of active attributes
    // (glow sizes the name buffer itself from `GL_ACTIVE_ATTRIBUTE_MAX_LENGTH`)
    let active_attributes = ctxt
        .glow
        .get_program_parameter_i32(program, glow::ACTIVE_ATTRIBUTES);

    // the result of this function
    let mut attributes = HashMap::with_hasher(Default::default());
    attributes.reserve(active_attributes as usize);

    for attribute_id in 0..active_attributes {
        let glow::ActiveAttribute {
            size: data_size,
            atype: data_type,
            name: attr_name,
        } = ctxt
            .glow
            .get_active_attribute(program, attribute_id as gl::types::GLuint)
            .expect("glGetActiveAttrib failed");

        if attr_name.starts_with("gl_") {
            // ignoring everything built-in
            continue;
        }

        if attr_name.is_empty() {
            // Some spirv compilers add an empty attribute to shaders. Most drivers
            // don't expose this attribute, but some do.
            // Since we can't do anything with empty attribute names, we simply skip
            // them in this reflection code.
            continue;
        }

        let location = ctxt
            .glow
            .get_attrib_location(program, &attr_name)
            .map_or(-1, |location| location as gl::types::GLint);

        // inputs of vendor types that no vertex format has are left out
        let Some(ty) = glenum_to_attribute_type(data_type) else {
            continue;
        };

        attributes.insert(
            attr_name,
            Attribute {
                location,
                ty,
                size: data_size as usize,
            },
        );
    }

    attributes
}

pub unsafe fn reflect_uniform_blocks(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
    let active_blocks = ctxt
        .glow
        .get_program_parameter_i32(program, glow::ACTIVE_UNIFORM_BLOCKS);

    // no need to request other things if there are no blocks
    if active_blocks == 0 {
        return HashMap::with_hasher(Default::default());
    }

    let mut blocks = HashMap::with_hasher(Default::default());
    blocks.reserve(active_blocks as usize);

    for block_id in 0..active_blocks {
        // getting the name of the block
        // (glow sizes the buffer from `GL_UNIFORM_BLOCK_NAME_LENGTH`, falling back to 256 bytes)
        let name = ctxt
            .glow
            .get_active_uniform_block_name(program, block_id as gl::types::GLuint);

        // binding point for this block
        let binding = ctxt.glow.get_active_uniform_block_parameter_i32(
            program,
            block_id as gl::types::GLuint,
            glow::UNIFORM_BLOCK_BINDING,
        );

        // number of bytes
        let block_size = ctxt.glow.get_active_uniform_block_parameter_i32(
            program,
            block_id as gl::types::GLuint,
            glow::UNIFORM_BLOCK_DATA_SIZE,
        );

        // number of members
        let num_members = ctxt.glow.get_active_uniform_block_parameter_i32(
            program,
            block_id as gl::types::GLuint,
            glow::UNIFORM_BLOCK_ACTIVE_UNIFORMS,
        );

        // indices of the members
        let mut members_indices = ::std::iter::repeat(0)
            .take(num_members as usize)
            .collect::<Vec<gl::types::GLint>>();
        ctxt.glow.get_active_uniform_block_parameter_i32_slice(
            program,
            block_id as gl::types::GLuint,
            glow::UNIFORM_BLOCK_ACTIVE_UNIFORM_INDICES,
            &mut members_indices,
        );
        let members_indices = members_indices
            .into_iter()
            .map(|index| index as gl::types::GLuint)
            .collect::<Vec<_>>();

        // getting the offsets of the members
        let member_offsets = ctxt.glow.get_active_uniforms_parameter(
            program,
            &members_indices,
            glow::UNIFORM_OFFSET,
        );

        // getting the types of the members
        let member_types =
            ctxt.glow
                .get_active_uniforms_parameter(program, &members_indices, glow::UNIFORM_TYPE);

        // getting the array sizes of the members
        let member_size =
            ctxt.glow
                .get_active_uniforms_parameter(program, &members_indices, glow::UNIFORM_SIZE);

        // getting the length of the names of the members
        let member_name_len = ctxt.glow.get_active_uniforms_parameter(
            program,
            &members_indices,
            glow::UNIFORM_NAME_LENGTH,
        );

        // getting the names of the members
        let member_names =
            member_name_len
                .iter()
                .zip(members_indices.iter())
                .map(|(&name_len, &index)| {
                    // `glGetActiveUniformName` only exists on desktop OpenGL; `glGetActiveUniform`
                    // returns the same name on OpenGL ES and WebGL
                    if ctxt.version.0 != Api::Gl {
                        return ctxt
                            .glow
                            .get_active_uniform(program, index)
                            .map(|uniform| uniform.name)
                            .unwrap_or_default();
                    }

                    let mut name_tmp: Vec<u8> = Vec::with_capacity(1 + name_len as usize);
                    let mut name_len_tmp = name_len;
                    ctxt.extra.GetActiveUniformName(
                        program.raw_name(),
                        index,
                        name_len,
                        &mut name_len_tmp,
                        name_tmp.as_mut_ptr() as *mut gl::types::GLchar,
                    );
                    name_tmp.set_len(name_len_tmp as usize);

                    String::from_utf8(name_tmp).unwrap()
                });

        // now computing the list of members
        let block_name = name.clone();
        let members = member_names.enumerate().map(|(index, member_name)| {
            (
                strip_block_prefix(&block_name, member_name),
                member_offsets[index] as usize,
                glenum_to_uniform_type(member_types[index] as gl::types::GLenum),
                member_size[index] as usize,
                None,
            )
        });

        // finally inserting into the blocks list
        blocks.insert(
            name,
            UniformBlock {
                id: block_id as i32,
                initial_binding: binding as i32,
                size: block_size as usize,
                layout: introspection_output_to_layout(members),
            },
        );
    }

    blocks
}

pub unsafe fn reflect_transform_feedback(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> Vec<TransformFeedbackBuffer> {
    // with enhanced layouts, the program interface gives the buffers, offsets and strides of the
    // varyings, including those of the `xfb_buffer`, `xfb_offset` and `xfb_stride` qualifiers;
    // the NVIDIA driver only lists the buffers of these qualifiers there, and the varyings given
    // when linking are reflected below
    if ctxt.version >= &Version(Api::Gl, 4, 4)
        || (ctxt.extensions.gl_arb_enhanced_layouts
            && ctxt.extensions.gl_arb_program_interface_query)
    {
        if let Some(buffers) = reflect_transform_feedback_interface(ctxt, program) {
            return buffers;
        }
    }

    // querying the number of varying
    let num_varyings = ctxt
        .glow
        .get_program_parameter_i32(program, glow::TRANSFORM_FEEDBACK_VARYINGS);

    // no need to request other things if there are no varying
    if num_varyings == 0 {
        return Vec::with_capacity(0);
    }

    // querying "interleaved" or "separate"
    let buffer_mode = {
        let buffer_mode = ctxt
            .glow
            .get_program_parameter_i32(program, glow::TRANSFORM_FEEDBACK_BUFFER_MODE);
        glenum_to_transform_feedback_mode(buffer_mode as gl::types::GLenum)
    };

    let mut result = Vec::with_capacity(num_varyings as usize);

    for index in 0..num_varyings as gl::types::GLuint {
        // (glow uses a fixed 256-byte name buffer)
        let glow::ActiveTransformFeedback {
            size,
            tftype: ty,
            name,
        } = ctxt
            .glow
            .get_transform_feedback_varying(program, index)
            .expect("glGetTransformFeedbackVarying failed");

        if buffer_mode == TransformFeedbackMode::Interleaved {
            if result.is_empty() {
                result.push(TransformFeedbackBuffer {
                    id: 0,
                    elements: vec![],
                    stride: 0,
                });
            }

            // the special names of interleaved varyings (OpenGL 4.0): `gl_NextBuffer` starts the
            // next buffer, and `gl_SkipComponents1` to `4` leave floats unwritten
            if name == "gl_NextBuffer" {
                let id = result.len() as i32;
                result.push(TransformFeedbackBuffer {
                    id,
                    elements: vec![],
                    stride: 0,
                });
                continue;
            }
            if let Some(count) = name
                .strip_prefix("gl_SkipComponents")
                .and_then(|count| count.parse::<usize>().ok())
            {
                result.last_mut().unwrap().stride += count * 4;
                continue;
            }

            let ty = glenum_to_attribute_type(ty as gl::types::GLenum)
                .expect("transform feedback varyings have standard types");

            let buffer = result.last_mut().unwrap();
            let offset = buffer.stride;
            buffer.stride += size as usize * ty.get_size_bytes();
            buffer.elements.push(TransformFeedbackVarying {
                name,
                size: size as usize * ty.get_size_bytes(),
                offset,
                ty,
            });
        } else if buffer_mode == TransformFeedbackMode::Separate {
            let id = result.len();
            let ty = glenum_to_attribute_type(ty as gl::types::GLenum)
                .expect("transform feedback varyings have standard types");
            result.push(TransformFeedbackBuffer {
                id: id as i32,
                elements: vec![TransformFeedbackVarying {
                    name,
                    size: size as usize * ty.get_size_bytes(),
                    offset: 0,
                    ty,
                }],
                stride: size as usize * ty.get_size_bytes(),
            });
        } else {
            unreachable!();
        }
    }

    result
}

/// Obtains the type of data that the geometry shader stage outputs.
///
/// # Unsafety
///
/// - `program` must be a valid handle to a program.
/// - The program **must** contain a geometry shader.
pub unsafe fn reflect_geometry_output_type(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> OutputPrimitives {
    let value = ctxt
        .glow
        .get_program_parameter_i32(program, glow::GEOMETRY_OUTPUT_TYPE);

    match value as gl::types::GLenum {
        gl::POINTS => OutputPrimitives::Points,
        gl::LINE_STRIP => OutputPrimitives::Lines,
        gl::TRIANGLE_STRIP => OutputPrimitives::Triangles,
        _ => unreachable!(),
    }
}

/// Obtains the type of primitives that the geometry shader stage takes as input, such as
/// `GL_TRIANGLES` or `GL_LINES_ADJACENCY`.
///
/// # Unsafety
///
/// - `program` must be a valid handle to a program.
/// - The program **must** contain a geometry shader.
pub unsafe fn reflect_geometry_input_type(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> gl::types::GLenum {
    ctxt.glow
        .get_program_parameter_i32(program, glow::GEOMETRY_INPUT_TYPE) as gl::types::GLenum
}

/// Obtains the type of data that the tessellation evaluation shader stage outputs.
///
/// # Unsafety
///
/// - `program` must be a valid handle to a program.
/// - The program **must** contain a tessellation evaluation shader.
pub unsafe fn reflect_tess_eval_output_type(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> OutputPrimitives {
    let value = ctxt
        .glow
        .get_program_parameter_i32(program, glow::TESS_GEN_MODE);

    match value as gl::types::GLenum {
        gl::TRIANGLES => OutputPrimitives::Triangles,
        gl::ISOLINES => OutputPrimitives::Lines,
        gl::QUADS => OutputPrimitives::Quads,
        _ => unreachable!(),
    }
}

/// Returns the list of shader storage blocks of a program.
/// Reflects the buffers of transform feedback and their varyings through the program interface
/// (OpenGL 4.4, or `GL_ARB_enhanced_layouts` with `GL_ARB_program_interface_query`), or returns
/// `None` if the interface has varyings but no buffers.
unsafe fn reflect_transform_feedback_interface(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> Option<Vec<TransformFeedbackBuffer>> {
    let count = |interface| {
        let mut count: gl::types::GLint = 0;
        ctxt.extra.GetProgramInterfaceiv(
            program.raw_name(),
            interface,
            gl::ACTIVE_RESOURCES,
            &mut count,
        );
        count.max(0) as gl::types::GLuint
    };
    let properties = |interface, index, properties: &[gl::types::GLenum]| {
        let mut output: [gl::types::GLint; 5] = [0; 5];
        ctxt.extra.GetProgramResourceiv(
            program.raw_name(),
            interface,
            index,
            properties.len() as gl::types::GLsizei,
            properties.as_ptr(),
            properties.len() as gl::types::GLsizei,
            ptr::null_mut(),
            output.as_mut_ptr(),
        );
        output
    };

    let varyings = count(gl::TRANSFORM_FEEDBACK_VARYING);
    let buffers = count(gl::TRANSFORM_FEEDBACK_BUFFER);
    if buffers == 0 && varyings != 0 {
        return None;
    }

    let mut buffers: Vec<TransformFeedbackBuffer> = (0..buffers)
        .map(|index| {
            let [binding, stride, ..] = properties(
                gl::TRANSFORM_FEEDBACK_BUFFER,
                index,
                &[gl::BUFFER_BINDING, gl::TRANSFORM_FEEDBACK_BUFFER_STRIDE],
            );
            TransformFeedbackBuffer {
                id: binding,
                elements: Vec::new(),
                stride: stride as usize,
            }
        })
        .collect();

    for index in 0..varyings {
        let [name_length, ty, array_size, offset, buffer] = properties(
            gl::TRANSFORM_FEEDBACK_VARYING,
            index,
            &[
                gl::NAME_LENGTH,
                gl::TYPE,
                gl::ARRAY_SIZE,
                gl::OFFSET,
                gl::TRANSFORM_FEEDBACK_BUFFER_INDEX,
            ],
        );

        let name = {
            let mut name: Vec<u8> = vec![0; name_length.max(1) as usize];
            let mut length = 0;
            ctxt.extra.GetProgramResourceName(
                program.raw_name(),
                gl::TRANSFORM_FEEDBACK_VARYING,
                index,
                name.len() as gl::types::GLsizei,
                &mut length,
                name.as_mut_ptr() as *mut _,
            );
            name.truncate(length.max(0) as usize);
            String::from_utf8(name).unwrap()
        };

        // `gl_NextBuffer` and `gl_SkipComponents1` to `4` only move the varyings that follow
        if name == "gl_NextBuffer" || name.starts_with("gl_SkipComponents") || buffer < 0 {
            continue;
        }
        let ty = glenum_to_attribute_type(ty as gl::types::GLenum)
            .expect("transform feedback varyings have standard types");
        let size = array_size.max(1) as usize * ty.get_size_bytes();
        if let Some(buffer) = buffers.get_mut(buffer as usize) {
            buffer.elements.push(TransformFeedbackVarying {
                name,
                offset: offset as usize,
                size,
                ty,
            });
        }
    }

    for buffer in &mut buffers {
        buffer.elements.sort_by_key(|element| element.offset);
    }
    buffers.sort_by_key(|buffer| buffer.id);
    Some(buffers)
}

pub unsafe fn reflect_shader_storage_blocks(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
    if !(ctxt.version >= &Version(Api::Gl, 4, 3)
        || ctxt.version >= &Version(Api::GlEs, 3, 1)
        || (ctxt.extensions.gl_arb_program_interface_query
            && ctxt.extensions.gl_arb_shader_storage_buffer_object))
    {
        // not supported
        return HashMap::with_hasher(Default::default());
    }

    // number of active SSBOs
    let active_blocks = {
        let mut active_blocks: gl::types::GLint = 0;
        ctxt.extra.GetProgramInterfaceiv(
            program.raw_name(),
            gl::SHADER_STORAGE_BLOCK,
            gl::ACTIVE_RESOURCES,
            &mut active_blocks,
        );
        active_blocks as gl::types::GLuint
    };

    // the result of this function
    let mut blocks = HashMap::with_hasher(Default::default());
    blocks.reserve(active_blocks as usize);

    for block_id in 0..active_blocks {
        // getting basic infos
        let (name_len, num_variables, binding, total_size) = {
            let mut output: [gl::types::GLint; 4] = [0; 4];
            ctxt.extra.GetProgramResourceiv(
                program.raw_name(),
                gl::SHADER_STORAGE_BLOCK,
                block_id,
                4,
                [
                    gl::NAME_LENGTH,
                    gl::NUM_ACTIVE_VARIABLES,
                    gl::BUFFER_BINDING,
                    gl::BUFFER_DATA_SIZE,
                ]
                .as_ptr(),
                4,
                ptr::null_mut(),
                output.as_mut_ptr() as *mut _,
            );
            (
                output[0] as usize,
                output[1] as usize,
                output[2],
                output[3] as usize,
            )
        };

        // getting the name of the block
        let name = {
            let mut name_tmp: Vec<u8> = Vec::with_capacity(1 + name_len);
            let mut name_tmp_len = name_len as gl::types::GLsizei;

            ctxt.extra.GetProgramResourceName(
                program.raw_name(),
                gl::SHADER_STORAGE_BLOCK,
                block_id,
                name_tmp_len,
                &mut name_tmp_len,
                name_tmp.as_mut_ptr() as *mut _,
            );
            name_tmp.set_len(name_tmp_len as usize);
            String::from_utf8(name_tmp).unwrap()
        };

        // indices of the active variables
        let active_variables: Vec<gl::types::GLint> = {
            let mut variables = Vec::with_capacity(num_variables);
            ctxt.extra.GetProgramResourceiv(
                program.raw_name(),
                gl::SHADER_STORAGE_BLOCK,
                block_id,
                1,
                [gl::ACTIVE_VARIABLES].as_ptr(),
                num_variables as gl::types::GLsizei,
                ptr::null_mut(),
                variables.as_mut_ptr() as *mut _,
            );
            variables.set_len(num_variables);
            variables
        };

        // iterator over variables
        let block_name = name.clone();
        let members = active_variables.into_iter().map(|variable| {
            let (ty, array_size, offset, _array_stride, name_len, top_level_array_size) = {
                let mut output: [gl::types::GLint; 6] = [0; 6];
                ctxt.extra.GetProgramResourceiv(
                    program.raw_name(),
                    gl::BUFFER_VARIABLE,
                    variable as gl::types::GLuint,
                    6,
                    [
                        gl::TYPE,
                        gl::ARRAY_SIZE,
                        gl::OFFSET,
                        gl::ARRAY_STRIDE,
                        gl::NAME_LENGTH,
                        gl::TOP_LEVEL_ARRAY_SIZE,
                    ]
                    .as_ptr(),
                    6,
                    ptr::null_mut(),
                    output.as_mut_ptr() as *mut _,
                );
                (
                    glenum_to_uniform_type(output[0] as gl::types::GLenum),
                    output[1] as usize,
                    output[2] as usize,
                    output[3] as usize,
                    output[4] as usize,
                    output[5] as usize,
                )
            };

            let name = {
                let mut name_tmp: Vec<u8> = Vec::with_capacity(1 + name_len);
                let mut name_tmp_len = name_len as gl::types::GLsizei;

                ctxt.extra.GetProgramResourceName(
                    program.raw_name(),
                    gl::BUFFER_VARIABLE,
                    variable as gl::types::GLuint,
                    name_tmp_len,
                    &mut name_tmp_len,
                    name_tmp.as_mut_ptr() as *mut _,
                );
                name_tmp.set_len(name_tmp_len as usize);
                String::from_utf8(name_tmp).unwrap()
            };

            (
                strip_block_prefix(&block_name, name),
                offset,
                ty,
                array_size,
                Some(top_level_array_size),
            )
        });

        // finally inserting into the blocks list
        blocks.insert(
            name,
            UniformBlock {
                id: block_id as i32,
                initial_binding: binding as i32,
                size: total_size,
                layout: introspection_output_to_layout(members),
            },
        );
    }

    blocks
}

/// Removes the `Block.` prefix that OpenGL puts before the names of the members of a block with an
/// instance name, like `uniform Light { vec3 position; } light;`, so that they match the fields of
/// the Rust struct.
fn strip_block_prefix(block_name: &str, member: String) -> String {
    let block_name = block_name.split('[').next().unwrap_or(block_name);
    match member
        .strip_prefix(block_name)
        .and_then(|rest| rest.strip_prefix('.'))
    {
        Some(rest) => rest.to_owned(),
        None => member,
    }
}

/// Takes a list of elements produced by OpenGL's introspection API and turns them into
/// a `BlockLayout` object.
///
/// The iterator must produce a list of `(name, offset, ty, array_size, top_level_array_size)`.
/// The `top_level_array_size` can be `None` if unknown.
///
/// # Panic
///
/// Panic if the input doesn't conform to the OpenGL specs.
///
fn introspection_output_to_layout<I>(elements: I) -> BlockLayout
where
    I: Iterator<Item = (String, usize, UniformType, usize, Option<usize>)>,
{
    // `output` must be a BlockLayout::Struct, otherwise this function will panic
    fn process(
        output: &mut BlockLayout,
        name: &str,
        offset: usize,
        ty: UniformType,
        array_size: usize,
        top_level_array_size: Option<usize>,
    ) {
        let mut components = name.splitn(2, '.');
        let current_component = components.next().unwrap();
        let name_rest = components.next();

        // finding the appropriate place in `output` to write the element
        let member = if let BlockLayout::Struct { ref mut members } = output {
            // splitting the name and array size
            let (current_component, array) = if current_component.ends_with(']') {
                let open_bracket_pos = current_component.rfind('[').unwrap();
                let array = current_component[open_bracket_pos + 1..current_component.len() - 1]
                    .parse()
                    .unwrap();
                (&current_component[..open_bracket_pos], Some(array))
            } else {
                (current_component, None)
            };

            // because of a bug in Rust's borrow checker, we have to loop twice instead of just
            // call `if let Some() { } else { }`
            let existing = members.iter_mut().any(|m| m.0 == current_component);
            if existing {
                let member = &mut members
                    .iter_mut()
                    .find(|m| m.0 == current_component)
                    .unwrap()
                    .1;

                if let Some(array) = array {
                    match member {
                        BlockLayout::Array {
                            ref mut content,
                            ref mut length,
                        } => {
                            if *length <= array {
                                *length = array + 1;
                            }
                            &mut **content
                        }
                        BlockLayout::DynamicSizedArray { ref mut content } => &mut **content,
                        _ => unreachable!(),
                    }
                } else {
                    member
                }
            } else {
                // member doesn't exist yet in the output, adding it
                if let Some(array) = array {
                    if top_level_array_size == Some(0) {
                        members.push((
                            current_component.to_owned(),
                            BlockLayout::DynamicSizedArray {
                                content: Box::new(BlockLayout::Struct {
                                    members: Vec::new(),
                                }),
                            },
                        ));
                    } else {
                        members.push((
                            current_component.to_owned(),
                            BlockLayout::Array {
                                content: Box::new(BlockLayout::Struct {
                                    members: Vec::new(),
                                }),
                                length: if name_rest.is_some() {
                                    array
                                } else {
                                    array_size
                                },
                            },
                        ));
                    }

                    match &mut members.last_mut().unwrap().1 {
                        BlockLayout::Array {
                            ref mut content, ..
                        } => &mut **content,
                        BlockLayout::DynamicSizedArray { ref mut content } => &mut **content,
                        _ => unreachable!(),
                    }
                } else {
                    members.push((
                        current_component.to_owned(),
                        BlockLayout::Struct {
                            members: Vec::new(),
                        },
                    ));
                    &mut members.last_mut().unwrap().1
                }
            }
        } else {
            unreachable!();
        };

        // now adding either the other elements or the final element itself
        if let Some(name_rest) = name_rest {
            process(member, name_rest, offset, ty, array_size, None);
        } else {
            // don't write over the offset in buffer
            match *member {
                BlockLayout::BasicType { ty: ty_ex, .. } if ty_ex == ty => (),
                _ => {
                    *member = BlockLayout::BasicType {
                        offset_in_buffer: offset,
                        ty,
                    };
                }
            }
        }
    }

    // ↓ actual body of `introspection_output_to_layout` starts here ↓

    // drivers list the variables in any order, alphabetical for some of them: sorting them by
    // offset gives the members in the order of their declaration
    let mut elements: Vec<_> = elements.collect();
    elements.sort_by_key(|element| element.1);

    let mut layout = BlockLayout::Struct {
        members: Vec::new(),
    };
    for (name, offset, ty, array_size, top_level_array_size) in elements {
        process(
            &mut layout,
            &name,
            offset,
            ty,
            array_size,
            top_level_array_size,
        );
    }
    layout
}

#[inline]
fn glenum_to_uniform_type(ty: gl::types::GLenum) -> UniformType {
    match ty {
        gl::FLOAT => UniformType::Float,
        gl::FLOAT_VEC2 => UniformType::FloatVec2,
        gl::FLOAT_VEC3 => UniformType::FloatVec3,
        gl::FLOAT_VEC4 => UniformType::FloatVec4,
        gl::DOUBLE => UniformType::Double,
        gl::DOUBLE_VEC2 => UniformType::DoubleVec2,
        gl::DOUBLE_VEC3 => UniformType::DoubleVec3,
        gl::DOUBLE_VEC4 => UniformType::DoubleVec4,
        gl::INT => UniformType::Int,
        gl::INT_VEC2 => UniformType::IntVec2,
        gl::INT_VEC3 => UniformType::IntVec3,
        gl::INT_VEC4 => UniformType::IntVec4,
        gl::UNSIGNED_INT => UniformType::UnsignedInt,
        gl::UNSIGNED_INT_VEC2 => UniformType::UnsignedIntVec2,
        gl::UNSIGNED_INT_VEC3 => UniformType::UnsignedIntVec3,
        gl::UNSIGNED_INT_VEC4 => UniformType::UnsignedIntVec4,
        gl::BOOL => UniformType::Bool,
        gl::BOOL_VEC2 => UniformType::BoolVec2,
        gl::BOOL_VEC3 => UniformType::BoolVec3,
        gl::BOOL_VEC4 => UniformType::BoolVec4,
        gl::FLOAT_MAT2 => UniformType::FloatMat2,
        gl::FLOAT_MAT3 => UniformType::FloatMat3,
        gl::FLOAT_MAT4 => UniformType::FloatMat4,
        gl::FLOAT_MAT2x3 => UniformType::FloatMat2x3,
        gl::FLOAT_MAT2x4 => UniformType::FloatMat2x4,
        gl::FLOAT_MAT3x2 => UniformType::FloatMat3x2,
        gl::FLOAT_MAT3x4 => UniformType::FloatMat3x4,
        gl::FLOAT_MAT4x2 => UniformType::FloatMat4x2,
        gl::FLOAT_MAT4x3 => UniformType::FloatMat4x3,
        gl::DOUBLE_MAT2 => UniformType::DoubleMat2,
        gl::DOUBLE_MAT3 => UniformType::DoubleMat3,
        gl::DOUBLE_MAT4 => UniformType::DoubleMat4,
        gl::DOUBLE_MAT2x3 => UniformType::DoubleMat2x3,
        gl::DOUBLE_MAT2x4 => UniformType::DoubleMat2x4,
        gl::DOUBLE_MAT3x2 => UniformType::DoubleMat3x2,
        gl::DOUBLE_MAT3x4 => UniformType::DoubleMat3x4,
        gl::DOUBLE_MAT4x2 => UniformType::DoubleMat4x2,
        gl::DOUBLE_MAT4x3 => UniformType::DoubleMat4x3,
        gl::SAMPLER_1D => UniformType::Sampler1d,
        gl::SAMPLER_2D => UniformType::Sampler2d,
        gl::SAMPLER_3D => UniformType::Sampler3d,
        gl::SAMPLER_CUBE => UniformType::SamplerCube,
        gl::SAMPLER_1D_SHADOW => UniformType::Sampler1dShadow,
        gl::SAMPLER_2D_SHADOW => UniformType::Sampler2dShadow,
        gl::SAMPLER_1D_ARRAY => UniformType::Sampler1dArray,
        gl::SAMPLER_2D_ARRAY => UniformType::Sampler2dArray,
        gl::SAMPLER_CUBE_MAP_ARRAY => UniformType::SamplerCubeArray,
        gl::SAMPLER_1D_ARRAY_SHADOW => UniformType::Sampler1dArrayShadow,
        gl::SAMPLER_2D_ARRAY_SHADOW => UniformType::Sampler2dArrayShadow,
        gl::SAMPLER_2D_MULTISAMPLE => UniformType::Sampler2dMultisample,
        gl::SAMPLER_2D_MULTISAMPLE_ARRAY => UniformType::Sampler2dMultisampleArray,
        gl::SAMPLER_CUBE_SHADOW => UniformType::SamplerCubeShadow,
        gl::SAMPLER_BUFFER => UniformType::SamplerBuffer,
        gl::SAMPLER_2D_RECT => UniformType::Sampler2dRect,
        gl::SAMPLER_2D_RECT_SHADOW => UniformType::Sampler2dRectShadow,
        gl::INT_SAMPLER_1D => UniformType::ISampler1d,
        gl::INT_SAMPLER_2D => UniformType::ISampler2d,
        gl::INT_SAMPLER_3D => UniformType::ISampler3d,
        gl::INT_SAMPLER_CUBE => UniformType::ISamplerCube,
        gl::INT_SAMPLER_1D_ARRAY => UniformType::ISampler1dArray,
        gl::INT_SAMPLER_2D_ARRAY => UniformType::ISampler2dArray,
        gl::INT_SAMPLER_CUBE_MAP_ARRAY => UniformType::ISamplerCubeArray,
        gl::INT_SAMPLER_2D_MULTISAMPLE => UniformType::ISampler2dMultisample,
        gl::INT_SAMPLER_2D_MULTISAMPLE_ARRAY => UniformType::ISampler2dMultisampleArray,
        gl::INT_SAMPLER_BUFFER => UniformType::ISamplerBuffer,
        gl::INT_SAMPLER_2D_RECT => UniformType::ISampler2dRect,
        gl::UNSIGNED_INT_SAMPLER_1D => UniformType::USampler1d,
        gl::UNSIGNED_INT_SAMPLER_2D => UniformType::USampler2d,
        gl::UNSIGNED_INT_SAMPLER_3D => UniformType::USampler3d,
        gl::UNSIGNED_INT_SAMPLER_CUBE => UniformType::USamplerCube,
        gl::UNSIGNED_INT_SAMPLER_1D_ARRAY => UniformType::USampler1dArray,
        gl::UNSIGNED_INT_SAMPLER_2D_ARRAY => UniformType::USampler2dArray,
        gl::UNSIGNED_INT_SAMPLER_CUBE_MAP_ARRAY => UniformType::USamplerCubeArray,
        gl::UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE => UniformType::USampler2dMultisample,
        gl::UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE_ARRAY => UniformType::USampler2dMultisampleArray,
        gl::UNSIGNED_INT_SAMPLER_BUFFER => UniformType::USamplerBuffer,
        gl::UNSIGNED_INT_SAMPLER_2D_RECT => UniformType::USampler2dRect,
        gl::IMAGE_1D => UniformType::Image1d,
        gl::IMAGE_2D => UniformType::Image2d,
        gl::IMAGE_3D => UniformType::Image3d,
        gl::IMAGE_2D_RECT => UniformType::Image2dRect,
        gl::IMAGE_CUBE => UniformType::ImageCube,
        gl::IMAGE_BUFFER => UniformType::ImageBuffer,
        gl::IMAGE_1D_ARRAY => UniformType::Image1dArray,
        gl::IMAGE_2D_ARRAY => UniformType::Image2dArray,
        gl::IMAGE_2D_MULTISAMPLE => UniformType::Image2dMultisample,
        gl::IMAGE_2D_MULTISAMPLE_ARRAY => UniformType::Image2dMultisampleArray,
        gl::INT_IMAGE_1D => UniformType::IImage1d,
        gl::INT_IMAGE_2D => UniformType::IImage2d,
        gl::INT_IMAGE_3D => UniformType::IImage3d,
        gl::INT_IMAGE_2D_RECT => UniformType::IImage2dRect,
        gl::INT_IMAGE_CUBE => UniformType::IImageCube,
        gl::INT_IMAGE_BUFFER => UniformType::IImageBuffer,
        gl::INT_IMAGE_1D_ARRAY => UniformType::IImage1dArray,
        gl::INT_IMAGE_2D_ARRAY => UniformType::IImage2dArray,
        gl::INT_IMAGE_2D_MULTISAMPLE => UniformType::IImage2dMultisample,
        gl::INT_IMAGE_2D_MULTISAMPLE_ARRAY => UniformType::IImage2dMultisampleArray,
        gl::UNSIGNED_INT_IMAGE_1D => UniformType::UImage1d,
        gl::UNSIGNED_INT_IMAGE_2D => UniformType::UImage2d,
        gl::UNSIGNED_INT_IMAGE_3D => UniformType::UImage3d,
        gl::UNSIGNED_INT_IMAGE_2D_RECT => UniformType::UImage2dRect,
        gl::UNSIGNED_INT_IMAGE_CUBE => UniformType::UImageCube,
        gl::UNSIGNED_INT_IMAGE_BUFFER => UniformType::UImageBuffer,
        gl::UNSIGNED_INT_IMAGE_1D_ARRAY => UniformType::UImage1dArray,
        gl::UNSIGNED_INT_IMAGE_2D_ARRAY => UniformType::UImage2dArray,
        gl::UNSIGNED_INT_IMAGE_2D_MULTISAMPLE => UniformType::UImage2dMultisample,
        gl::UNSIGNED_INT_IMAGE_2D_MULTISAMPLE_ARRAY => UniformType::UImage2dMultisampleArray,
        gl::UNSIGNED_INT_ATOMIC_COUNTER => UniformType::AtomicCounterUint,
        gl::SAMPLER_CUBE_MAP_ARRAY_SHADOW => UniformType::SamplerCubeArrayShadow,
        gl::IMAGE_CUBE_MAP_ARRAY => UniformType::ImageCubeArray,
        gl::INT_IMAGE_CUBE_MAP_ARRAY => UniformType::IImageCubeArray,
        gl::UNSIGNED_INT_IMAGE_CUBE_MAP_ARRAY => UniformType::UImageCubeArray,
        gl::INT64_ARB => UniformType::Int64,
        gl::INT64_VEC2_ARB => UniformType::Int64Vec2,
        gl::INT64_VEC3_ARB => UniformType::Int64Vec3,
        gl::INT64_VEC4_ARB => UniformType::Int64Vec4,
        gl::UNSIGNED_INT64_ARB => UniformType::UnsignedInt64,
        gl::UNSIGNED_INT64_VEC2_ARB => UniformType::UnsignedInt64Vec2,
        gl::UNSIGNED_INT64_VEC3_ARB => UniformType::UnsignedInt64Vec3,
        gl::UNSIGNED_INT64_VEC4_ARB => UniformType::UnsignedInt64Vec4,
        // types of vendor extensions, which no value can be bound to
        v => UniformType::Unknown(v),
    }
}

#[inline]
fn glenum_to_attribute_type(value: gl::types::GLenum) -> Option<AttributeType> {
    Some(match value {
        gl::FLOAT => AttributeType::F32,
        gl::FLOAT_VEC2 => AttributeType::F32F32,
        gl::FLOAT_VEC3 => AttributeType::F32F32F32,
        gl::FLOAT_VEC4 => AttributeType::F32F32F32F32,
        gl::INT => AttributeType::I32,
        gl::INT_VEC2 => AttributeType::I32I32,
        gl::INT_VEC3 => AttributeType::I32I32I32,
        gl::INT_VEC4 => AttributeType::I32I32I32I32,
        gl::UNSIGNED_INT => AttributeType::U32,
        gl::UNSIGNED_INT_VEC2 => AttributeType::U32U32,
        gl::UNSIGNED_INT_VEC3 => AttributeType::U32U32U32,
        gl::UNSIGNED_INT_VEC4 => AttributeType::U32U32U32U32,
        gl::FLOAT_MAT2 => AttributeType::F32x2x2,
        gl::FLOAT_MAT3 => AttributeType::F32x3x3,
        gl::FLOAT_MAT4 => AttributeType::F32x4x4,
        gl::FLOAT_MAT2x3 => AttributeType::F32x2x3,
        gl::FLOAT_MAT2x4 => AttributeType::F32x2x4,
        gl::FLOAT_MAT3x2 => AttributeType::F32x3x2,
        gl::FLOAT_MAT3x4 => AttributeType::F32x3x4,
        gl::FLOAT_MAT4x2 => AttributeType::F32x4x2,
        gl::FLOAT_MAT4x3 => AttributeType::F32x4x3,
        #[cfg(safegl_desktop)]
        gl::DOUBLE => AttributeType::F64,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_VEC2 => AttributeType::F64F64,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_VEC3 => AttributeType::F64F64F64,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_VEC4 => AttributeType::F64F64F64F64,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT2 => AttributeType::F64x2x2,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT3 => AttributeType::F64x3x3,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT4 => AttributeType::F64x4x4,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT2x3 => AttributeType::F64x2x3,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT2x4 => AttributeType::F64x2x4,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT3x2 => AttributeType::F64x3x2,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT3x4 => AttributeType::F64x3x4,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT4x2 => AttributeType::F64x4x2,
        #[cfg(safegl_desktop)]
        gl::DOUBLE_MAT4x3 => AttributeType::F64x4x3,
        #[cfg(safegl_desktop)]
        gl::INT64_ARB => AttributeType::I64,
        #[cfg(safegl_desktop)]
        gl::INT64_VEC2_ARB => AttributeType::I64I64,
        #[cfg(safegl_desktop)]
        gl::INT64_VEC3_ARB => AttributeType::I64I64I64,
        #[cfg(safegl_desktop)]
        gl::INT64_VEC4_ARB => AttributeType::I64I64I64I64,
        #[cfg(safegl_desktop)]
        gl::UNSIGNED_INT64_ARB => AttributeType::U64,
        #[cfg(safegl_desktop)]
        gl::UNSIGNED_INT64_VEC2_ARB => AttributeType::U64U64,
        #[cfg(safegl_desktop)]
        gl::UNSIGNED_INT64_VEC3_ARB => AttributeType::U64U64U64,
        #[cfg(safegl_desktop)]
        gl::UNSIGNED_INT64_VEC4_ARB => AttributeType::U64U64U64U64,
        gl::FLOAT16_NV => AttributeType::F16,
        gl::FLOAT16_VEC2_NV => AttributeType::F16F16,
        gl::FLOAT16_VEC3_NV => AttributeType::F16F16F16,
        gl::FLOAT16_VEC4_NV => AttributeType::F16F16F16F16,
        _ => return None,
    })
}

#[inline]
fn glenum_to_transform_feedback_mode(value: gl::types::GLenum) -> TransformFeedbackMode {
    match value {
        gl::INTERLEAVED_ATTRIBS => {
            TransformFeedbackMode::Interleaved
        },
        gl::SEPARATE_ATTRIBS => {
            TransformFeedbackMode::Separate
        },
        v => panic!("Unknown value returned by OpenGL varying mode: {}", v)
    }
}

/// Contains all subroutine data of a program.
#[derive(Debug, Default, Clone)]
pub struct SubroutineData {
    /// Number of subroutine uniform locations per shader stage.
    /// This is *not* equal to the number of subroutine uniforms per stage,
    /// because users can use `#layout(location=...)`.
    pub location_counts: HashMap<ShaderStage, usize, BuildHasherDefault<FnvHasher>>,

    /// The list of all subroutine uniforms of the program stored in a structured way to enable fast lookups.
    /// A subroutine uniform is uniquely defined by a name and a shader stage.
    pub subroutine_uniforms:
        HashMap<(String, ShaderStage), SubroutineUniform, BuildHasherDefault<FnvHasher>>,
}

/// Information about a Subroutine Uniform (except name)
#[derive(Debug, Clone)]
pub struct SubroutineUniform {
    /// The index of the subroutine uniform.
    /// Needed to query information from the OpenGL backend.
    pub index: u32,

    /// The location of the uniform.
    /// This is used to bind subroutines to this subroutine uniform.
    pub location: i32,

    /// If the uniform is an array, the size of the array.
    pub size: Option<usize>,

    /// A list of subroutines that can potentially be used with this uniform.
    pub compatible_subroutines: Vec<Subroutine>,
}

/// Information about a subroutine.
#[derive(Debug, Clone)]
pub struct Subroutine {
    /// The index of the subroutine, needed to bind this to a subroutine uniform.
    pub index: u32,

    /// The name of the subroutine.
    pub name: String,
}

/// The different stages of the program pipeline.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    // FIXME? According to https://www.opengl.org/sdk/docs/man/html/glUniformSubroutines.xhtml ,
    // compute shaders are not supported.
    // Compute,
}

impl ShaderStage {
    /// Converts the `ShaderStage` to its GLenum equivalent
    pub fn to_gl_enum(&self) -> gl::types::GLenum {
        match *self {
            ShaderStage::Vertex => gl::VERTEX_SHADER,
            ShaderStage::Fragment => gl::FRAGMENT_SHADER,
            ShaderStage::TessellationControl => gl::TESS_CONTROL_SHADER,
            ShaderStage::TessellationEvaluation => gl::TESS_EVALUATION_SHADER,
            ShaderStage::Geometry => gl::GEOMETRY_SHADER,
            // Compute => gl::COMPUTE_SHADER,
        }
    }
}

fn get_shader_stages(
    has_geometry_shader: bool,
    has_tessellation_control_shader: bool,
    has_tessellation_evaluation_shader: bool,
) -> Vec<ShaderStage> {
    let mut stages = vec![ShaderStage::Vertex, ShaderStage::Fragment];
    if has_tessellation_evaluation_shader {
        stages.push(ShaderStage::TessellationEvaluation);
    }
    if has_tessellation_control_shader {
        stages.push(ShaderStage::TessellationControl);
    }
    if has_geometry_shader {
        stages.push(ShaderStage::Geometry);
    }
    stages
}

/// Returns the data associated with a programs subroutines, which are desktop OpenGL only.
#[cfg(not(safegl_desktop))]
pub unsafe fn reflect_subroutine_data(
    _: &mut CommandContext<'_>,
    _: glow::Program,
    _: bool,
    _: bool,
    _: bool,
) -> SubroutineData {
    Default::default()
}

/// Returns the data associated with a programs subroutines.
#[cfg(safegl_desktop)]
pub unsafe fn reflect_subroutine_data(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
    has_geometry_shader: bool,
    has_tessellation_control_shader: bool,
    has_tessellation_evaluation_shader: bool,
) -> SubroutineData {
    if !program::is_subroutine_supported(ctxt) {
        return Default::default();
    }

    // Compute shader programs are not handled: #1718
    let shader_count = ctxt
        .glow
        .get_program_parameter_i32(program, glow::ATTACHED_SHADERS);
    if shader_count < 2 {
        return Default::default();
    }

    let shader_stages = get_shader_stages(
        has_geometry_shader,
        has_tessellation_control_shader,
        has_tessellation_evaluation_shader,
    );
    let mut subroutine_uniforms = HashMap::with_hasher(Default::default());
    let mut location_counts = HashMap::with_hasher(Default::default());
    for stage in shader_stages.iter() {
        let mut location_count: gl::types::GLint = 0;
        ctxt.extra.GetProgramStageiv(
            program.raw_name(),
            stage.to_gl_enum(),
            gl::ACTIVE_SUBROUTINE_UNIFORM_LOCATIONS,
            &mut location_count,
        );
        location_counts.insert(*stage, location_count as usize);
        let mut subroutine_count: gl::types::GLint = 0;
        ctxt.extra.GetProgramStageiv(
            program.raw_name(),
            stage.to_gl_enum(),
            gl::ACTIVE_SUBROUTINE_UNIFORMS,
            &mut subroutine_count,
        );
        for i in 0..subroutine_count {
            // Get the name of the uniform
            let mut uniform_name_tmp: Vec<u8> = vec![0; 64];
            let mut name_len: gl::types::GLsizei = 0;
            ctxt.extra.GetActiveSubroutineUniformName(
                program.raw_name(),
                stage.to_gl_enum(),
                i as gl::types::GLuint,
                (uniform_name_tmp.len() - 1) as gl::types::GLint,
                &mut name_len,
                uniform_name_tmp.as_mut_ptr() as *mut gl::types::GLchar,
            );

            let location = ctxt.extra.GetSubroutineUniformLocation(
                program.raw_name(),
                stage.to_gl_enum(),
                uniform_name_tmp.as_ptr() as *const gl::types::GLchar,
            );

            uniform_name_tmp.set_len(name_len as usize);
            let uniform_name = String::from_utf8(uniform_name_tmp).unwrap();

            let mut size: gl::types::GLint = 0;
            ctxt.extra.GetActiveSubroutineUniformiv(
                program.raw_name(),
                stage.to_gl_enum(),
                i as u32,
                gl::UNIFORM_SIZE,
                &mut size,
            );
            let size = if size == 1 { None } else { Some(size as usize) };

            // Get the number of compatible subroutines.
            let mut compatible_count: gl::types::GLint = 0;
            ctxt.extra.GetActiveSubroutineUniformiv(
                program.raw_name(),
                stage.to_gl_enum(),
                i as u32,
                gl::NUM_COMPATIBLE_SUBROUTINES,
                &mut compatible_count,
            );

            // Get the indices of compatible subroutines.
            let mut compatible_sr_indices: Vec<gl::types::GLuint> =
                Vec::with_capacity(compatible_count as usize);
            ctxt.extra.GetActiveSubroutineUniformiv(
                program.raw_name(),
                stage.to_gl_enum(),
                i as gl::types::GLuint,
                gl::COMPATIBLE_SUBROUTINES,
                compatible_sr_indices.as_mut_ptr() as *mut gl::types::GLint,
            );
            compatible_sr_indices.set_len(compatible_count as usize);
            let mut compatible_subroutines: Vec<Subroutine> = Vec::new();
            for j in 0..compatible_count {
                // Get the names of compatible subroutines.
                let mut subroutine_name_tmp: Vec<u8> = vec![0; 64];
                let mut name_len: gl::types::GLsizei = 0;
                ctxt.extra.GetActiveSubroutineName(
                    program.raw_name(),
                    stage.to_gl_enum(),
                    compatible_sr_indices[j as usize],
                    subroutine_name_tmp.len() as gl::types::GLint,
                    &mut name_len,
                    subroutine_name_tmp.as_mut_ptr() as *mut gl::types::GLchar,
                );

                subroutine_name_tmp.set_len(name_len as usize);
                let subroutine_name = String::from_utf8(subroutine_name_tmp).unwrap();
                compatible_subroutines.push(Subroutine {
                    index: compatible_sr_indices[j as usize],
                    name: subroutine_name,
                });
            }

            let subroutine_uniform = SubroutineUniform {
                index: i as u32,
                location,
                size,
                compatible_subroutines,
            };
            subroutine_uniforms.insert((uniform_name, *stage), subroutine_uniform);
        }
    }
    SubroutineData {
        location_counts,
        subroutine_uniforms,
    }
}

#[cfg(test)]
mod tests {
    use super::{introspection_output_to_layout, BlockLayout, UniformType};

    #[test]
    fn members_in_declaration_order() {
        // the order of NVIDIA's driver
        let elements = vec![
            ("dir".to_owned(), 8, UniformType::FloatVec2, 1, None),
            ("pos".to_owned(), 0, UniformType::FloatVec2, 1, None),
            ("speed".to_owned(), 16, UniformType::Float, 1, None),
        ];
        let member = |name: &str, ty, offset_in_buffer| {
            (
                name.to_owned(),
                BlockLayout::BasicType {
                    ty,
                    offset_in_buffer,
                },
            )
        };

        assert_eq!(
            introspection_output_to_layout(elements.into_iter()),
            BlockLayout::Struct {
                members: vec![
                    member("pos", UniformType::FloatVec2, 0),
                    member("dir", UniformType::FloatVec2, 8),
                    member("speed", UniformType::Float, 16),
                ],
            }
        );
    }
}
