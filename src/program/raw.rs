use crate::gl;

use crate::context::CommandContext;
use crate::handles::RawName;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::version::Api;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::version::Version;

use crate::backend::Facade;
use crate::context::Context;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::vertex::TransformFeedback;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::TransformFeedbackExt;
use crate::ContextExt;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::uniforms::CollectedUniforms;

use std::cell::RefCell;
use std::collections::hash_map::{self, HashMap};
use std::fmt;
use std::hash::BuildHasherDefault;
use std::rc::Rc;

use fnv::FnvHasher;
use glow::HasContext;

#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::DrawError;
use crate::GlObject;
use crate::ProgramExt;
use crate::RawUniformValue;

#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::draw_parameters::AnySamplesPassedQuery;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::QueryExt;

#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::buffer::BufferSlice;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::BufferExt;
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::BufferSliceExt;

use crate::program::uniforms_storage::UniformsStorage;
#[cfg(any(safegl_gl41, safegl_gles30))]
use crate::program::{is_binary_supported, Binary, GetBinaryError};
use crate::program::spirv::Module;
use crate::program::{LinkLocations, ProgramCreationError};

#[cfg(any(safegl_gl41, safegl_gles30))]
use crate::program::binary_header::{attach_safegl_header, process_safegl_header};
#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::program::compute::ComputeCommand;
use crate::program::reflection::reflect_subroutine_data;
use crate::program::reflection::reflect_transform_feedback;
#[cfg(safegl_gl46)]
use crate::program::reflection::reflect_spirv;
use crate::program::reflection::{reflect_attributes, reflect_uniform_blocks, reflect_uniforms};
use crate::program::reflection::{reflect_geometry_input_type, reflect_geometry_output_type};
use crate::program::reflection::{reflect_shader_storage_blocks, reflect_tess_eval_output_type};
use crate::program::reflection::{Attribute, TransformFeedbackBuffer, TransformFeedbackMode};
use crate::program::reflection::{OutputPrimitives, Uniform, UniformBlock};
use crate::program::reflection::{ShaderStage, SubroutineData};
use crate::program::shader::Shader;

#[cfg(any(safegl_gl43, safegl_gles31))]
use crate::uniforms::Uniforms;

use crate::vertex::VertexFormat;
use crate::vertex_array_object::VertexAttributesSystem;

/// A combination of shaders linked together.
pub struct RawProgram {
    context: Rc<Context>,
    id: glow::Program,
    uniform_values: UniformsStorage,
    uniforms: HashMap<String, Uniform, BuildHasherDefault<FnvHasher>>,
    uniform_blocks: HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>>,
    subroutine_data: SubroutineData,
    attributes: HashMap<String, Attribute, BuildHasherDefault<FnvHasher>>,
    frag_data_locations: RefCell<HashMap<String, Option<u32>, BuildHasherDefault<FnvHasher>>>,
    tf_buffers: Vec<TransformFeedbackBuffer>,
    ssbos: HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>>,
    atomic_counters: HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>>,
    output_primitives: Option<OutputPrimitives>,
    geometry_input_type: Option<gl::types::GLenum>,
    has_geometry_shader: bool,
    has_tessellation_control_shader: bool,
    has_tessellation_evaluation_shader: bool,
    /// The locations and indices of the fragment outputs of a program linked from SPIR-V,
    /// which OpenGL can't look up by name.
    spirv_outputs: Option<HashMap<String, (u32, u32), BuildHasherDefault<FnvHasher>>>,
}

impl RawProgram {
    /// Builds a new program from a list of shaders.
    // TODO: the "has_*" parameters are bad
    pub fn from_shaders<'a, F: ?Sized, I>(
        facade: &'a F,
        shaders: I,
        has_geometry_shader: bool,
        has_tessellation_control_shader: bool,
        has_tessellation_evaluation_shader: bool,
        transform_feedback: Option<(Vec<String>, TransformFeedbackMode)>,
        fragment_source: Option<&str>,
        locations: LinkLocations<'_>,
        spirv: Option<&[Module]>,
        separable: Option<gl::types::GLbitfield>,
    ) -> Result<RawProgram, ProgramCreationError>
    where
        F: Facade,
        I: IntoIterator<Item = &'a Shader>,
    {
        let mut ctxt = facade.get_context().make_current();
        check_link_locations(&ctxt, &locations)?;

        let shaders_ids = shaders.into_iter().map(|s| s.get_id()).collect::<Vec<_>>();

        let id = unsafe {
            let id = create_program(&mut ctxt)?;

            // attaching shaders
            for &sh in shaders_ids.iter() {
                ctxt.glow.attach_shader(id, sh);
            }

            // transform feedback varyings
            if let Some((names, mode)) = transform_feedback {
                let names = names.iter().map(|name| name.as_str()).collect::<Vec<_>>();

                let mode = match mode {
                    TransformFeedbackMode::Interleaved => glow::INTERLEAVED_ATTRIBS,
                    TransformFeedbackMode::Separate => glow::SEPARATE_ATTRIBS,
                };

                ctxt.glow.transform_feedback_varyings(id, &names, mode);
            }

            for &(name, location) in locations.attributes {
                ctxt.glow.bind_attrib_location(id, location, name);
            }

            // the fragment outputs without a location get locations in their declaration order,
            // so that the first one goes to the first draw buffer on every driver; OpenGL ES
            // requires a location when there are several outputs, unless the locations of some
            // of them are given here
            let given = locations.fragment_outputs;
            if ctxt.version.0 == crate::version::Api::Gl || !given.is_empty() {
                if let Some(source) = fragment_source {
                    bind_fragment_output_locations(&ctxt, id, source, given);
                }
            }
            for &(name, location, index) in given {
                bind_frag_data_location(&ctxt, id, location, index, name);
            }

            // the binaries of programs can be retrieved after they are linked with this hint,
            // which some drivers need (core in OpenGL 4.1 and OpenGL ES 3.0)
            if !crate::version::is_webgl()
                && (ctxt.version.0 == crate::version::Api::GlEs
                    || ctxt.version >= &crate::version::Version(crate::version::Api::Gl, 4, 1)
                    || ctxt.extensions.gl_arb_get_program_binary)
            {
                ctxt.extra.ProgramParameteri(
                    id.raw_name(),
                    gl::PROGRAM_BINARY_RETRIEVABLE_HINT,
                    gl::TRUE as gl::types::GLint,
                );
            }

            // programs used by program pipelines are linked separable, which callers check that
            // the context supports
            if separable.is_some() {
                ctxt.extra.ProgramParameteri(
                    id.raw_name(),
                    gl::PROGRAM_SEPARABLE,
                    gl::TRUE as gl::types::GLint,
                );
            }

            // linking
            {
                ctxt.report_debug_output_errors.set(false);
                ctxt.glow.link_program(id);
                ctxt.report_debug_output_errors.set(true);
            }

            // checking for errors
            check_program_link_errors(&mut ctxt, id)?;

            id
        };

        // drivers can leave out the names of the resources of SPIR-V modules
        let (uniforms, atomic_counters, uniform_locations, attributes, blocks, ssbos, outputs) =
            match spirv {
                #[cfg(safegl_gl46)]
                Some(modules) => {
                    let spirv = unsafe { reflect_spirv(&mut ctxt, id, modules) };
                    (
                        spirv.uniforms,
                        spirv.atomic_counters,
                        spirv.locations,
                        spirv.attributes,
                        spirv.uniform_blocks,
                        spirv.ssbos,
                        Some(spirv.outputs),
                    )
                }
                _ => unsafe {
                    let (uniforms, atomic_counters, uniform_locations) =
                        reflect_uniforms(&mut ctxt, id);
                    (
                        uniforms,
                        atomic_counters,
                        uniform_locations,
                        reflect_attributes(&mut ctxt, id),
                        reflect_uniform_blocks(&mut ctxt, id),
                        reflect_shader_storage_blocks(&mut ctxt, id),
                        None,
                    )
                },
            };
        let tf_buffers = unsafe { reflect_transform_feedback(&mut ctxt, id) };
        let subroutine_data = unsafe {
            reflect_subroutine_data(
                &mut ctxt,
                id,
                has_geometry_shader,
                has_tessellation_control_shader,
                has_tessellation_evaluation_shader,
                separable,
            )
        };

        let output_primitives = if has_geometry_shader {
            Some(unsafe { reflect_geometry_output_type(&mut ctxt, id) })
        } else if has_tessellation_evaluation_shader {
            Some(unsafe { reflect_tess_eval_output_type(&mut ctxt, id) })
        } else {
            None
        };
        let geometry_input_type = if has_geometry_shader {
            Some(unsafe { reflect_geometry_input_type(&mut ctxt, id) })
        } else {
            None
        };

        Ok(RawProgram {
            context: facade.get_context().clone(),
            id,
            uniforms,
            uniform_values: UniformsStorage::new(uniform_locations),
            uniform_blocks: blocks,
            subroutine_data,
            attributes,
            frag_data_locations: RefCell::new(HashMap::with_hasher(Default::default())),
            tf_buffers,
            ssbos,
            atomic_counters,
            output_primitives,
            geometry_input_type,
            has_geometry_shader,
            has_tessellation_control_shader,
            has_tessellation_evaluation_shader,
            spirv_outputs: outputs,
        })
    }

    /// Creates a program from binary.
    #[cfg(any(safegl_gl41, safegl_gles30))]
    pub fn from_binary<F: ?Sized>(
        facade: &F,
        binary: Binary,
    ) -> Result<RawProgram, ProgramCreationError>
    where
        F: Facade,
    {
        if !is_binary_supported(&**facade.get_context()) {
            return Err(ProgramCreationError::BinaryNotSupported);
        }

        let (
            has_geometry_shader,
            has_tessellation_control_shader,
            has_tessellation_evaluation_shader,
        ) = {
            match process_safegl_header(&binary.content) {
                Some(flags) => flags,
                None => return Err(ProgramCreationError::BinaryHeaderError),
            }
        };

        let mut ctxt = facade.get_context().make_current();

        let id = unsafe {
            let id = create_program(&mut ctxt)?;

            ctxt.glow.program_binary(
                id,
                &glow::ProgramBinary {
                    buffer: binary.content[1..].to_vec(),
                    format: binary.format,
                },
            );

            // checking for errors
            check_program_link_errors(&mut ctxt, id)?;

            id
        };

        let (
            (uniforms, atomic_counters, uniform_locations),
            attributes,
            blocks,
            tf_buffers,
            ssbos,
            subroutine_data,
        ) = unsafe {
            (
                reflect_uniforms(&mut ctxt, id),
                reflect_attributes(&mut ctxt, id),
                reflect_uniform_blocks(&mut ctxt, id),
                reflect_transform_feedback(&mut ctxt, id),
                reflect_shader_storage_blocks(&mut ctxt, id),
                reflect_subroutine_data(
                    &mut ctxt,
                    id,
                    has_geometry_shader,
                    has_tessellation_control_shader,
                    has_tessellation_evaluation_shader,
                    None,
                ),
            )
        };

        let output_primitives = if has_geometry_shader {
            Some(unsafe { reflect_geometry_output_type(&mut ctxt, id) })
        } else if has_tessellation_evaluation_shader {
            Some(unsafe { reflect_tess_eval_output_type(&mut ctxt, id) })
        } else {
            None
        };
        let geometry_input_type = if has_geometry_shader {
            Some(unsafe { reflect_geometry_input_type(&mut ctxt, id) })
        } else {
            None
        };

        Ok(RawProgram {
            context: facade.get_context().clone(),
            id,
            uniforms,
            uniform_values: UniformsStorage::new(uniform_locations),
            uniform_blocks: blocks,
            subroutine_data,
            attributes,
            frag_data_locations: RefCell::new(HashMap::with_hasher(Default::default())),
            tf_buffers,
            ssbos,
            atomic_counters,
            output_primitives,
            geometry_input_type,
            has_geometry_shader,
            has_tessellation_control_shader,
            has_tessellation_evaluation_shader,
            spirv_outputs: None,
        })
    }

    /// Returns the program's compiled binary.
    ///
    /// You can store the result in a file, then reload it later. This avoids having to compile
    /// the source code every time.
    #[cfg(any(safegl_gl41, safegl_gles30))]
    pub fn get_binary(&self) -> Result<Binary, GetBinaryError> {
        if !is_binary_supported(&*self.context) {
            return Err(GetBinaryError::NotSupported);
        }

        unsafe {
            let ctxt = self.context.make_current();

            let num_supported_formats = ctxt
                .glow
                .get_parameter_i32(glow::NUM_PROGRAM_BINARY_FORMATS);
            if num_supported_formats == 0 {
                return Err(GetBinaryError::NoFormats);
            }

            // glow queries `GL_PROGRAM_BINARY_LENGTH` itself and returns `None` if
            // `glGetProgramBinary` raised a GL error
            let glow::ProgramBinary {
                buffer: mut storage,
                format,
            } = match ctxt.glow.get_program_binary(self.id) {
                Some(binary) => binary,
                None => return Err(GetBinaryError::NotSupported),
            };
            attach_safegl_header(&self, &mut storage);
            Ok(Binary {
                format,
                content: storage,
            })
        }
    }

    /// Returns the *location* of an output fragment, if it exists.
    ///
    /// The *location* is low-level information that is used internally by SafeGL.
    /// You probably don't need to call this function.
    ///
    /// You can declare output fragments in your shaders by writing:
    ///
    /// ```notrust
    /// out vec4 foo;
    /// ```
    ///
    pub fn get_frag_data_location(&self, name: &str) -> Option<u32> {
        if let Some(outputs) = &self.spirv_outputs {
            return outputs.get(name).map(|output| output.0);
        }

        // looking for a cached value
        if let Some(result) = self.frag_data_locations.borrow_mut().get(name) {
            return *result;
        }

        // querying opengl
        let ctxt = self.context.make_current();

        let value = unsafe { ctxt.glow.get_frag_data_location(self.id, name) };

        let location = match value {
            -1 => None,
            a => Some(a as u32),
        };

        self.frag_data_locations
            .borrow_mut()
            .insert(name.to_owned(), location);
        location
    }

    /// Forgets the values of the subroutine uniforms, which binding a program or a pipeline
    /// resets.
    #[cfg(all(safegl_desktop, safegl_gl41))]
    pub(crate) fn flush_subroutine_uniforms(&self) {
        self.uniform_values.flush_subroutine_uniforms();
    }

    /// Returns the index of a fragment output: 1 for the second color output of dual-source
    /// blending, declared with `layout(location = 0, index = 1)`, and 0 for the others. Returns
    /// `None` if the program has no output with this name.
    pub fn get_frag_data_index(&self, name: &str) -> Option<u32> {
        if let Some(outputs) = &self.spirv_outputs {
            return outputs.get(name).map(|output| output.1);
        }
        self.get_frag_data_location(name)?;

        let ctxt = self.context.make_current();
        let name = std::ffi::CString::new(name).ok()?;
        let index = unsafe {
            if ctxt.version.0 == crate::version::Api::Gl {
                ctxt.extra.GetFragDataIndex(self.id.raw_name(), name.as_ptr())
            } else if ctxt.extensions.gl_ext_blend_func_extended {
                ctxt.extra.GetFragDataIndexEXT(self.id.raw_name(), name.as_ptr())
            } else {
                // without dual-source blending, outputs can't have another index
                0
            }
        };
        u32::try_from(index).ok()
    }

    /// Returns the size of the local work groups of a compute program
    /// (`GL_COMPUTE_WORK_GROUP_SIZE`).
    #[cfg(any(safegl_gl43, safegl_gles31))]
    pub fn compute_work_group_size(&self) -> [u32; 3] {
        let ctxt = self.context.make_current();
        let mut size = [0i32; 3];
        unsafe {
            ctxt.extra.GetProgramiv(
                self.id.raw_name(),
                gl::COMPUTE_WORK_GROUP_SIZE,
                size.as_mut_ptr(),
            )
        };
        size.map(|s| s.max(0) as u32)
    }

    /// Returns informations about a uniform variable, if it exists.
    #[inline]
    pub fn get_uniform(&self, name: &str) -> Option<&Uniform> {
        self.uniforms.get(name)
    }

    /// Returns an iterator to the list of uniforms.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # fn example(program: safegl::Program) {
    /// for (name, uniform) in program.uniforms() {
    ///     println!("Name: {} - Type: {:?}", name, uniform.ty);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn uniforms(&self) -> hash_map::Iter<'_, String, Uniform> {
        self.uniforms.iter()
    }

    /// Returns a list of uniform blocks.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # fn example(program: safegl::Program) {
    /// for (name, uniform) in program.get_uniform_blocks() {
    ///     println!("Name: {}", name);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn get_uniform_blocks(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        &self.uniform_blocks
    }

    /// Returns the list of transform feedback varyings.
    #[inline]
    pub fn get_transform_feedback_buffers(&self) -> &[TransformFeedbackBuffer] {
        &self.tf_buffers
    }

    /// True if the transform feedback output of this program matches the specified `VertexFormat`
    /// and `stride`.
    ///
    /// The `stride` is the number of bytes between two vertices.
    ///
    /// This function only check the identity for type and offset(exclude the naming) between vertex attributes.
    ///
    /// The correctness of semantic meaning(i.e. naming) between vertex attributes is the responsibility of user.
    pub fn transform_feedback_matches(&self, format: &VertexFormat, stride: usize) -> bool {
        // TODO: doesn't support multiple buffers

        if self.get_transform_feedback_buffers().len() != 1 {
            return false;
        }

        let buf = &self.get_transform_feedback_buffers()[0];

        if buf.stride != stride {
            return false;
        }

        for elem in buf.elements.iter() {
            if format
                .iter()
                .find(|e| e.1 == elem.offset && e.3 == elem.ty)
                .is_none()
            {
                return false;
            }

            if format
                .iter()
                .any(|e| e.1 != elem.offset && e.0 == elem.name)
            {
                return false;
            }
        }

        true
    }

    /// Returns the type of geometry that transform feedback would generate, or `None` if it
    /// depends on the vertex/index data passed when drawing.
    ///
    /// This corresponds to `GL_GEOMETRY_OUTPUT_TYPE` or `GL_TESS_GEN_MODE`. If the program doesn't
    /// contain either a geometry shader or a tessellation evaluation shader, returns `None`.
    #[inline]
    pub fn get_output_primitives(&self) -> Option<OutputPrimitives> {
        self.output_primitives
    }

    /// Returns the type of primitives that the geometry shader takes as input, such as
    /// `GL_TRIANGLES`, or `None` if the program has no geometry shader.
    #[inline]
    pub fn get_geometry_input_type(&self) -> Option<gl::types::GLenum> {
        self.geometry_input_type
    }

    /// Returns true if the program contains a tessellation stage.
    #[inline]
    pub fn has_tessellation_shaders(&self) -> bool {
        self.has_tessellation_control_shader() | self.has_tessellation_evaluation_shader()
    }

    /// Returns true if the program contains a tessellation control stage.
    #[inline]
    pub fn has_tessellation_control_shader(&self) -> bool {
        self.has_tessellation_control_shader
    }

    /// Returns true if the program contains a tessellation evaluation stage.
    #[inline]
    pub fn has_tessellation_evaluation_shader(&self) -> bool {
        self.has_tessellation_evaluation_shader
    }

    /// Returns true if the program contains a geometry shader.
    #[inline]
    pub fn has_geometry_shader(&self) -> bool {
        self.has_geometry_shader
    }

    /// Returns informations about an attribute, if it exists.
    #[inline]
    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.get(name)
    }

    /// Returns an iterator to the list of attributes.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # fn example(program: safegl::Program) {
    /// for (name, attribute) in program.attributes() {
    ///     println!("Name: {} - Type: {:?}", name, attribute.ty);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn attributes(&self) -> hash_map::Iter<'_, String, Attribute> {
        self.attributes.iter()
    }

    /// Returns the list of shader storage blocks.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # #[cfg(any(safegl_gl43, safegl_gles31))]
    /// # fn example(program: safegl::Program) {
    /// for (name, uniform) in program.get_shader_storage_blocks() {
    ///     println!("Name: {}", name);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn get_shader_storage_blocks(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        &self.ssbos
    }

    /// Returns the list of atomic_counters
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # #[cfg(any(safegl_gl42, safegl_gles31))]
    /// # fn example(program: safegl::Program) {
    /// for (name, uniform) in program.get_atomic_counters() {
    ///     println!("Name: {}", name);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn get_atomic_counters(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        &self.atomic_counters
    }

    /// Returns data associated with the programs subroutines.
    #[inline]
    pub fn get_subroutine_data(&self) -> &SubroutineData {
        &self.subroutine_data
    }

    /// Assumes that the program contains a compute shader and executes it.
    ///
    /// # Safety
    ///
    /// The program *must* contain a compute shader.
    /// TODO: check inside the program if it has a compute shader instead of being unsafe
    #[cfg(any(safegl_gl43, safegl_gles31))]
    pub unsafe fn dispatch_compute<U>(
        &self,
        uniforms: U,
        x: u32,
        y: u32,
        z: u32,
        statistics: &[&StatisticsQuery],
    ) -> Result<(), DrawError>
    where
        U: Uniforms,
    {
        // the values of the uniforms are collected before the context is borrowed, so that the
        // code that gives them can use SafeGL
        let uniforms = CollectedUniforms::new(&uniforms);
        let mut ctxt = self.context.make_current();

        if !is_compute_supported(&ctxt) {
            return Err(DrawError::ComputeShadersNotSupported);
        }

        // the maximum number of work groups is included
        let (max_x, max_y, max_z) = ctxt.capabilities.max_compute_work_group_count;
        if x > max_x as u32 || y > max_y as u32 || z > max_z as u32 {
            return Err(DrawError::WorkGroupCountTooLarge {
                requested: (x, y, z),
                maximum: (max_x as u32, max_y as u32, max_z as u32),
            });
        }

        AnySamplesPassedQuery::end_conditional_render(&mut ctxt);
        // `glUseProgram` is invalid while transform feedback records
        TransformFeedback::pause(&mut ctxt);

        let mut fences = crate::buffer::FenceInserters::new();

        self.use_program(&mut ctxt);
        uniforms.bind(&mut ctxt, self, &mut fences)?;
        sync_statistics(&mut ctxt, statistics)?;
        ctxt.glow.dispatch_compute(x, y, z);

        for fence in fences {
            fence.insert(&mut ctxt);
        }

        ctxt.state.next_draw_call_id += 1;

        Ok(())
    }

    /// Assumes that the program contains a compute shader and executes it.
    ///
    /// # Safety
    ///
    /// The program *must* contain a compute shader.
    /// TODO: check inside the program if it has a compute shader instead of being unsafe
    #[cfg(any(safegl_gl43, safegl_gles31))]
    pub unsafe fn dispatch_compute_indirect<U>(
        &self,
        uniforms: U,
        buffer: BufferSlice<'_, ComputeCommand>,
        statistics: &[&StatisticsQuery],
    ) -> Result<(), DrawError>
    where
        U: Uniforms,
    {
        // the values of the uniforms are collected before the context is borrowed, so that the
        // code that gives them can use SafeGL
        let uniforms = CollectedUniforms::new(&uniforms);
        let mut ctxt = self.context.make_current();

        if !is_compute_supported(&ctxt) {
            return Err(DrawError::ComputeShadersNotSupported);
        }

        if !ctxt.owns(buffer.get_context()) {
            return Err(DrawError::ContextMismatch {
                object: "dispatch commands buffer".to_owned(),
            });
        }

        // OpenGL requires the offset of the commands to be a multiple of 4
        let offset = buffer.get_offset_bytes();
        if offset % 4 != 0 {
            return Err(DrawError::BufferOffsetNotAligned {
                name: "dispatch commands buffer".to_owned(),
                offset,
                alignment: 4,
            });
        }

        AnySamplesPassedQuery::end_conditional_render(&mut ctxt);
        // `glUseProgram` is invalid while transform feedback records
        TransformFeedback::pause(&mut ctxt);

        buffer.prepare_and_bind_for_dispatch_indirect(&mut ctxt);

        if let Some(fence) = buffer.add_fence() {
            fence.insert(&mut ctxt);
        }

        self.use_program(&mut ctxt);

        let mut fences = crate::buffer::FenceInserters::new();
        uniforms.bind(&mut ctxt, self, &mut fences)?;
        sync_statistics(&mut ctxt, statistics)?;

        ctxt.glow.dispatch_compute_indirect(offset as i32);

        for fence in fences {
            fence.insert(&mut ctxt);
        }

        ctxt.state.next_draw_call_id += 1;

        Ok(())
    }
}

/// The query of a pipeline statistic given to a dispatch, which only exists at the `gl46` level.
#[cfg(safegl_gl46)]
pub type StatisticsQuery = crate::draw_parameters::PipelineStatisticsQuery;

/// The query of a pipeline statistic given to a dispatch, which only exists at the `gl46` level.
#[cfg(not(safegl_gl46))]
pub enum StatisticsQuery {}

/// Makes the statistics queries active for the next dispatch, and ends the other pipeline
/// statistics and transform feedback overflow queries.
#[cfg(any(safegl_gl43, safegl_gles31))]
fn sync_statistics(
    ctxt: &mut CommandContext<'_>,
    statistics: &[&StatisticsQuery],
) -> Result<(), DrawError> {
    #[cfg(safegl_gl46)]
    {
        let queries: smallvec::SmallVec<[_; 4]> = statistics.iter().map(|q| q.raw()).collect();
        crate::draw_parameters::sync_indexed_queries(ctxt, &queries, true)
    }
    #[cfg(not(safegl_gl46))]
    {
        let _ = (ctxt, statistics);
        Ok(())
    }
}

impl fmt::Debug for RawProgram {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        (format!("Program #{:?}", self.id)).fmt(formatter)
    }
}

impl GlObject for RawProgram {
    type Id = glow::Program;

    #[inline]
    fn get_id(&self) -> glow::Program {
        self.id
    }
}

impl ProgramExt for RawProgram {
    #[inline]
    fn get_context(&self) -> &Context {
        &self.context
    }

    #[inline]
    fn use_program(&self, ctxt: &mut CommandContext<'_>) {
        unsafe {
            let program_id = self.get_id();
            if ctxt.state.program != Some(program_id) {
                ctxt.glow.use_program(Some(program_id));
                ctxt.state.program = Some(program_id);
                self.uniform_values.flush_subroutine_uniforms();
            }
        }
    }

    #[inline]
    fn set_uniform(
        &self,
        ctxt: &mut CommandContext<'_>,
        uniform_location: gl::types::GLint,
        value: &RawUniformValue,
    ) {
        self.uniform_values
            .set_uniform_value(ctxt, self.id, uniform_location, value);
    }

    #[inline]
    fn set_uniform_block_binding(
        &self,
        ctxt: &mut CommandContext<'_>,
        block_location: gl::types::GLuint,
        value: gl::types::GLuint,
    ) {
        self.uniform_values
            .set_uniform_block_binding(ctxt, self.id, block_location, value);
    }

    #[inline]
    fn set_shader_storage_block_binding(
        &self,
        ctxt: &mut CommandContext<'_>,
        block_location: gl::types::GLuint,
        value: gl::types::GLuint,
    ) {
        self.uniform_values
            .set_shader_storage_block_binding(ctxt, self.id, block_location, value);
    }

    #[inline]
    fn set_subroutine_uniforms_for_stage(
        &self,
        ctxt: &mut CommandContext<'_>,
        stage: ShaderStage,
        indices: &[gl::types::GLuint],
    ) {
        self.uniform_values
            .set_subroutine_uniforms_for_stage(ctxt, self.id, stage, indices);
    }

    #[inline]
    fn get_uniform(&self, name: &str) -> Option<&Uniform> {
        self.uniforms.get(name)
    }

    #[inline]
    fn get_uniforms(&self) -> &HashMap<String, Uniform, BuildHasherDefault<FnvHasher>> {
        &self.uniforms
    }

    #[inline]
    fn is_uniform_set(&self, location: gl::types::GLint) -> bool {
        self.uniform_values.is_set(location)
    }

    #[inline]
    fn get_uniform_blocks(&self) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        &self.uniform_blocks
    }

    #[inline]
    fn get_shader_storage_blocks(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        &self.ssbos
    }

    #[inline]
    fn get_atomic_counters(&self) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        &self.atomic_counters
    }

    #[inline]
    fn get_subroutine_data(&self) -> &SubroutineData {
        &self.subroutine_data
    }
}

impl Drop for RawProgram {
    fn drop(&mut self) {
        let mut ctxt = self.context.make_current();

        // removing VAOs which contain this program
        VertexAttributesSystem::purge_program(&mut ctxt, self.id);

        // sending the destroy command
        unsafe {
            if ctxt.state.program == Some(self.id) {
                ctxt.glow.use_program(None);
                ctxt.state.program = None;
                self.uniform_values.flush_subroutine_uniforms();
            }

            if !ctxt.objects_lost() {
                ctxt.glow.delete_program(self.id);
            }
        }
    }
}

/// Builds an empty program from within the GL context, which fails when the context is lost.
unsafe fn create_program(
    ctxt: &mut CommandContext<'_>,
) -> Result<glow::Program, ProgramCreationError> {
    // the objects of a lost context are gone, whether the browser creates them or not
    if ctxt.objects_lost() {
        return Err(ProgramCreationError::ContextLost);
    }
    ctxt.glow
        .create_program()
        .map_err(|_| ProgramCreationError::ContextLost)
}

unsafe fn check_program_link_errors(
    ctxt: &mut CommandContext<'_>,
    program: glow::Program,
) -> Result<(), ProgramCreationError> {
    if !ctxt.glow.get_program_link_status(program) {
        use crate::ProgramCreationError::LinkingError;

        match ctxt.glow.get_error() {
            glow::NO_ERROR => (),
            glow::INVALID_VALUE => {
                return Err(LinkingError(
                    "glLinkProgram triggered GL_INVALID_VALUE".to_string(),
                ));
            }
            glow::INVALID_OPERATION => {
                return Err(LinkingError(
                    "glLinkProgram triggered GL_INVALID_OPERATION".to_string(),
                ));
            }
            _ => {
                return Err(LinkingError(
                    "glLinkProgram triggered an unknown error".to_string(),
                ));
            }
        };

        let msg = ctxt.glow.get_program_info_log(program);
        return Err(LinkingError(msg));
    }

    Ok(())
}

/// Returns true if compute shaders are supported.
#[cfg(any(safegl_gl43, safegl_gles31))]
fn is_compute_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version >= &Version(Api::Gl, 4, 3)
        || ctxt.version >= &Version(Api::GlEs, 3, 1)
        || ctxt.extensions.gl_arb_compute_shader
}

/// Gives a location and an index to a fragment output before linking. OpenGL ES needs
/// `GL_EXT_blend_func_extended`.
unsafe fn bind_frag_data_location(
    ctxt: &CommandContext<'_>,
    program: glow::Program,
    location: u32,
    index: u32,
    name: &str,
) {
    // `check_link_locations` checked the names given by the user, and GLSL names have no nul
    let Ok(name) = std::ffi::CString::new(name) else {
        return;
    };
    if ctxt.version.0 == crate::version::Api::Gl {
        ctxt.extra
            .BindFragDataLocationIndexed(program.raw_name(), location, index, name.as_ptr());
    } else {
        ctxt.extra.BindFragDataLocationIndexedEXT(
            program.raw_name(),
            location,
            index,
            name.as_ptr(),
        );
    }
}

/// Checks the locations given before linking, which OpenGL would refuse with an error.
fn check_link_locations(
    ctxt: &CommandContext<'_>,
    locations: &LinkLocations<'_>,
) -> Result<(), ProgramCreationError> {
    // WebGL reserves its own prefixes, and limits the length of the names it is given
    let webgl = |name: &str| {
        name.starts_with("webgl_") || name.starts_with("_webgl_") || name.len() > 1024
    };
    let invalid = |name: &str| {
        name.starts_with("gl_")
            || name.contains('\0')
            || name.is_empty()
            || (crate::version::is_webgl() && webgl(name))
    };
    let error = |name: &str| ProgramCreationError::InvalidLinkLocation {
        name: name.to_owned(),
    };

    let limits = &ctxt.capabilities.limits;
    for &(name, location) in locations.attributes {
        if invalid(name) || location >= limits.max_vertex_attribs.max(0) as u32 {
            return Err(error(name));
        }
    }

    if locations.fragment_outputs.is_empty() {
        return Ok(());
    }
    if !(ctxt.version.0 == crate::version::Api::Gl || ctxt.extensions.gl_ext_blend_func_extended) {
        return Err(ProgramCreationError::LinkLocationsNotSupported);
    }
    let draw_buffers = ctxt.capabilities.max_draw_buffers.max(0) as u32;
    let dual_source = limits.max_dual_source_draw_buffers.unwrap_or(0).max(0) as u32;
    for &(name, location, index) in locations.fragment_outputs {
        let max = match index {
            0 => draw_buffers,
            1 => dual_source,
            _ => 0,
        };
        if invalid(name) || location >= max {
            return Err(error(name));
        }
    }
    Ok(())
}

/// Binds the outputs of a fragment shader that have no location to locations in their
/// declaration order, skipping the locations given in the shader and in `given`, the locations
/// and indices given before linking.
unsafe fn bind_fragment_output_locations(
    ctxt: &CommandContext<'_>,
    program: glow::Program,
    source: &str,
    given: &[(&str, u32, u32)],
) {
    let outputs = crate::program::shader::fragment_outputs(source);

    // the locations of index 0 given before linking, with the sizes of their outputs
    let given_locations: Vec<(u32, u32)> = given
        .iter()
        .filter(|&&(_, _, index)| index == 0)
        .map(|&(name, location, _)| {
            let size = outputs
                .iter()
                .find(|output| output.name == name)
                .map_or(1, |output| output.size);
            (location, size)
        })
        .collect();

    let is_free = |location: u32, size: u32| {
        outputs.iter().all(|output| match output.location {
            Some(taken) => location + size <= taken || taken + output.size <= location,
            None => true,
        }) && given_locations
            .iter()
            .all(|&(taken, taken_size)| location + size <= taken || taken + taken_size <= location)
    };

    let mut next = 0;
    let mut assigned: Vec<(u32, u32)> = Vec::new();
    let unplaced = outputs.iter().filter(|output| {
        output.location.is_none() && !given.iter().any(|&(name, _, _)| name == output.name)
    });
    for output in unplaced {
        while !is_free(next, output.size)
            || assigned
                .iter()
                .any(|&(start, size)| next < start + size && start < next + output.size)
        {
            next += 1;
        }
        bind_frag_data_location(ctxt, program, next, 0, &output.name);
        assigned.push((next, output.size));
        next += output.size;
    }
}
