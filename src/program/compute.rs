use crate::gl;

use crate::backend::Facade;
use crate::context::CommandContext;

use std::collections::hash_map::{self, HashMap};
use std::fmt;
use std::hash::BuildHasherDefault;
use std::os::raw;

use fnv::FnvHasher;

use crate::CapabilitiesSource;
use crate::DrawError;
use crate::GlObject;
use crate::ProgramExt;
use crate::RawUniformValue;

#[cfg(safegl_gl46)]
use crate::program::SpirvEntryPoint;
#[cfg(any(safegl_gl41, safegl_gles30))]
use crate::program::{Binary, GetBinaryError};
use crate::program::{ProgramCreationError, COMPILER_GLOBAL_LOCK};

use crate::program::reflection::{ShaderStage, SubroutineData};
use crate::program::reflection::{Uniform, UniformBlock};
#[cfg(safegl_gl46)]
use crate::program::shader::build_spirv_shader;
use crate::program::shader::{build_shader, check_shader_type_compatibility};

use crate::program::raw::RawProgram;

use crate::buffer::BufferSlice;
use crate::uniforms::Uniforms;

/// A combination of compute shaders linked together.
pub struct ComputeShader {
    raw: RawProgram,
}

impl ComputeShader {
    /// Returns true if the backend supports compute shaders.
    #[inline]
    pub fn is_supported<C: ?Sized>(ctxt: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        check_shader_type_compatibility(ctxt, gl::COMPUTE_SHADER)
    }

    /// Builds a new compute shader from some source code.
    #[inline]
    pub fn from_source<F: ?Sized>(
        facade: &F,
        src: &str,
    ) -> Result<ComputeShader, ProgramCreationError>
    where
        F: Facade,
    {
        let _lock = COMPILER_GLOBAL_LOCK.lock();

        let shader = build_shader(facade, gl::COMPUTE_SHADER, src)?;

        Ok(ComputeShader {
            raw: RawProgram::from_shaders(facade, &[shader], false, false, false, None, None)?,
        })
    }

    /// Builds a new compute shader from SPIR-V module.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    #[inline]
    pub fn from_spirv<F: ?Sized>(
        facade: &F,
        spirv: &SpirvEntryPoint,
    ) -> Result<ComputeShader, ProgramCreationError>
    where
        F: Facade,
    {
        let _lock = COMPILER_GLOBAL_LOCK.lock();

        let shader = build_spirv_shader(facade, gl::COMPUTE_SHADER, spirv)?;

        Ok(ComputeShader {
            raw: RawProgram::from_shaders(facade, &[shader], false, false, false, None, None)?,
        })
    }

    /// Builds a new compute shader from some binary.
    #[inline]
    #[cfg(any(safegl_gl41, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
    pub fn from_binary<F: ?Sized>(
        facade: &F,
        data: Binary,
    ) -> Result<ComputeShader, ProgramCreationError>
    where
        F: Facade,
    {
        let _lock = COMPILER_GLOBAL_LOCK.lock();

        Ok(ComputeShader {
            raw: RawProgram::from_binary(facade, data)?,
        })
    }

    /// Returns the size of the local work groups of the compute shader, from its
    /// `layout(local_size_x = ..., local_size_y = ..., local_size_z = ...) in;` declaration
    /// (`GL_COMPUTE_WORK_GROUP_SIZE`).
    #[inline]
    pub fn work_group_size(&self) -> [u32; 3] {
        self.raw.compute_work_group_size()
    }

    /// Executes the compute shader.
    ///
    /// `x * y * z` work groups will be started. The current work group can be retrieved with
    /// `gl_WorkGroupID`. Inside each work group, additional local work groups can be started
    /// depending on the attributes of the compute shader itself.
    ///
    /// Returns an error if the uniforms don't match the program, or if there are more work groups
    /// than the backend supports.
    #[inline]
    pub fn execute<U>(&self, uniforms: U, x: u32, y: u32, z: u32) -> Result<(), DrawError>
    where
        U: Uniforms,
    {
        unsafe { self.raw.dispatch_compute(uniforms, x, y, z, &[]) }
    }

    /// Executes the compute shader.
    ///
    /// This is similar to `execute`, except that the parameters are stored in a buffer.
    #[inline]
    pub fn execute_indirect<U>(
        &self,
        uniforms: U,
        buffer: BufferSlice<'_, ComputeCommand>,
    ) -> Result<(), DrawError>
    where
        U: Uniforms,
    {
        unsafe { self.raw.dispatch_compute_indirect(uniforms, buffer, &[]) }
    }

    /// Executes the compute shader like `execute`, while the queries count their pipeline
    /// statistics, such as `PipelineStatistic::ComputeShaderInvocations`. Like the queries of
    /// draws, a query keeps counting through the dispatches and draws that follow each other
    /// and are given it; each query must count another statistic, or this returns
    /// `DrawError::WrongQueryOperation`.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    #[inline]
    pub fn execute_with_statistics<U>(
        &self,
        uniforms: U,
        x: u32,
        y: u32,
        z: u32,
        statistics: &[&crate::draw_parameters::PipelineStatisticsQuery],
    ) -> Result<(), DrawError>
    where
        U: Uniforms,
    {
        unsafe { self.raw.dispatch_compute(uniforms, x, y, z, statistics) }
    }

    /// Returns the program's compiled binary.
    ///
    /// You can store the result in a file, then reload it later. This avoids having to compile
    /// the source code every time.
    #[inline]
    #[cfg(any(safegl_gl41, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
    pub fn get_binary(&self) -> Result<Binary, GetBinaryError> {
        self.raw.get_binary()
    }

    /// Returns informations about a uniform variable, if it exists.
    #[inline]
    pub fn get_uniform(&self, name: &str) -> Option<&Uniform> {
        self.raw.get_uniform(name)
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
        self.raw.uniforms()
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
        self.raw.get_uniform_blocks()
    }

    /// Returns the list of shader storage blocks.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// fn example(program: safegl::Program) {
    /// for (name, uniform) in program.get_shader_storage_blocks() {
    ///     println!("Name: {}", name);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn get_shader_storage_blocks(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        self.raw.get_shader_storage_blocks()
    }
}

impl fmt::Debug for ComputeShader {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "{:?}", self.raw)
    }
}

#[cfg(any(safegl_gl43, safegl_gles32))]
impl crate::debug::sealed::Labeled for ComputeShader {
    fn labeled_object(
        &self,
    ) -> (&crate::context::Context, crate::debug::sealed::LabeledObject) {
        (
            ProgramExt::get_context(self),
            crate::debug::sealed::LabeledObject::Object(crate::gl::PROGRAM, self.get_id().0.get()),
        )
    }
}

impl GlObject for ComputeShader {
    type Id = glow::Program;

    #[inline]
    fn get_id(&self) -> glow::Program {
        self.raw.get_id()
    }
}

impl ProgramExt for ComputeShader {
    #[inline]
    fn get_context(&self) -> &crate::context::Context {
        self.raw.get_context()
    }

    #[inline]
    fn use_program(&self, ctxt: &mut CommandContext<'_>) {
        self.raw.use_program(ctxt)
    }

    #[inline]
    fn set_uniform(
        &self,
        ctxt: &mut CommandContext<'_>,
        uniform_location: gl::types::GLint,
        value: &RawUniformValue,
    ) {
        self.raw.set_uniform(ctxt, uniform_location, value)
    }

    #[inline]
    fn set_uniform_block_binding(
        &self,
        ctxt: &mut CommandContext<'_>,
        block_location: gl::types::GLuint,
        value: gl::types::GLuint,
    ) {
        self.raw
            .set_uniform_block_binding(ctxt, block_location, value)
    }

    #[inline]
    fn set_shader_storage_block_binding(
        &self,
        ctxt: &mut CommandContext<'_>,
        block_location: gl::types::GLuint,
        value: gl::types::GLuint,
    ) {
        self.raw
            .set_shader_storage_block_binding(ctxt, block_location, value)
    }

    #[inline]
    fn set_subroutine_uniforms_for_stage(
        &self,
        ctxt: &mut CommandContext<'_>,
        stage: ShaderStage,
        indices: &[gl::types::GLuint],
    ) {
        self.raw
            .set_subroutine_uniforms_for_stage(ctxt, stage, indices);
    }

    #[inline]
    fn get_uniform(&self, name: &str) -> Option<&Uniform> {
        self.raw.get_uniform(name)
    }

    #[inline]
    fn get_uniforms(&self) -> &HashMap<String, Uniform, BuildHasherDefault<FnvHasher>> {
        self.raw.get_uniforms()
    }

    #[inline]
    fn is_uniform_set(&self, location: gl::types::GLint) -> bool {
        self.raw.is_uniform_set(location)
    }

    #[inline]
    fn get_uniform_blocks(&self) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        self.raw.get_uniform_blocks()
    }

    #[inline]
    fn get_shader_storage_blocks(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        self.raw.get_shader_storage_blocks()
    }

    #[inline]
    fn get_atomic_counters(&self) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        self.raw.get_atomic_counters()
    }

    #[inline]
    fn get_subroutine_data(&self) -> &SubroutineData {
        self.raw.get_subroutine_data()
    }
}

/// Represents a compute shader command waiting to be dispatched.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ComputeCommand {
    /// Number of X groups.
    pub num_groups_x: raw::c_uint,
    /// Number of Y groups.
    pub num_groups_y: raw::c_uint,
    /// Number of Z groups.
    pub num_groups_z: raw::c_uint,
}

implement_uniform_block!(ComputeCommand, num_groups_x, num_groups_y, num_groups_z);
