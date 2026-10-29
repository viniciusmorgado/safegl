use crate::gl;

use crate::context::CommandContext;
use crate::version::Api;

use crate::backend::Facade;

use std::collections::hash_map::{self, HashMap};
use std::fmt;
use std::mem;
use std::rc::Rc;

use crate::context::Context;
use std::hash::BuildHasherDefault;

use fnv::FnvHasher;
use glow::HasContext;

use crate::GlObject;
use crate::ContextExt;
use crate::ProgramExt;
use crate::RawUniformValue;

#[cfg(any(safegl_gl41, safegl_gles30))]
use crate::program::Binary;
#[cfg(any(safegl_gl41, safegl_gles30))]
use crate::program::GetBinaryError;
#[cfg(safegl_gl46)]
use crate::program::SpirvProgram;
use crate::program::{stage_source, StageSource};
use crate::program::{
    ProgramCreationError, ProgramCreationInput, ShaderType, COMPILER_GLOBAL_LOCK,
};

#[cfg(safegl_desktop)]
use crate::program::reflection::SubroutineUniform;
use crate::program::reflection::{Attribute, TransformFeedbackBuffer};
use crate::program::reflection::{OutputPrimitives, Uniform, UniformBlock};
use crate::program::reflection::{ShaderStage, SubroutineData};
use crate::program::raw::check_program_link_errors;
use crate::program::shader::{check_shader, declared_views, start_shader, Shader};
#[cfg(safegl_gl46)]
use crate::program::shader::build_spirv_shader;

use crate::program::raw::RawProgram;

use crate::vertex::VertexFormat;

/// A combination of shaders linked together.
pub struct Program {
    raw: RawProgram,
    outputs_srgb: bool,
    uses_point_size: bool,
    // the stages of a separable program, 0 for the others
    separable_stages: gl::types::GLbitfield,
    // the number of views that the vertex shader declares (`GL_OVR_multiview2`), `None` for
    // the programs whose source SafeGL doesn't have
    declared_views: Option<Option<u32>>,
}

impl Program {
    /// Builds a new program.
    // at the levels without binaries nor SPIR-V, every input is source code, which returns early
    #[allow(unreachable_code)]
    pub fn new<'a, F: ?Sized, I>(facade: &F, input: I) -> Result<Program, ProgramCreationError>
    where
        I: Into<ProgramCreationInput<'a>>,
        F: Facade,
    {
        let input = input.into();

        let (raw, outputs_srgb, uses_point_size) = match input {
            // the program is linked whether the driver compiles in the background or not
            input @ ProgramCreationInput::SourceCode { .. } => {
                return Program::new_pending(facade, input)?.finish();
            }

            #[cfg(any(safegl_gl41, safegl_gles30))]
            ProgramCreationInput::Binary {
                data,
                outputs_srgb,
                uses_point_size,
            } => (
                RawProgram::from_binary(facade, data)?,
                outputs_srgb,
                uses_point_size,
            ),

            #[cfg(safegl_gl46)]
            ProgramCreationInput::SpirV(SpirvProgram {
                vertex_shader,
                tessellation_control_shader,
                tessellation_evaluation_shader,
                geometry_shader,
                fragment_shader,
                outputs_srgb,
                uses_point_size,
            }) => {
                let mut has_geometry_shader = false;
                let mut has_tessellation_control_shader = false;
                let mut has_tessellation_evaluation_shader = false;

                let mut shaders = vec![
                    (vertex_shader, ShaderType::Vertex),
                    (fragment_shader, ShaderType::Fragment),
                ];

                if let Some(gs) = geometry_shader {
                    shaders.push((gs, ShaderType::Geometry));
                    has_geometry_shader = true;
                }

                if let Some(ts) = tessellation_control_shader {
                    shaders.push((ts, ShaderType::TesselationControl));
                    has_tessellation_control_shader = true;
                }

                if let Some(ts) = tessellation_evaluation_shader {
                    shaders.push((ts, ShaderType::TesselationEvaluation));
                    has_tessellation_evaluation_shader = true;
                }

                let _lock = COMPILER_GLOBAL_LOCK.lock();

                let mut shaders_store = Vec::new();
                let mut modules = Vec::new();
                for (src, ty) in shaders.into_iter() {
                    let (shader, module) = build_spirv_shader(facade, ty.to_opengl_type(), &src)?;
                    shaders_store.push(shader);
                    modules.push(module);
                }

                // the modules capture with `xfb_buffer` and `xfb_offset`
                (
                    RawProgram::from_shaders(
                        facade,
                        &shaders_store,
                        has_geometry_shader,
                        has_tessellation_control_shader,
                        has_tessellation_evaluation_shader,
                        None,
                        None,
                        Default::default(),
                        Some(&modules),
                        None,
                    )?,
                    outputs_srgb,
                    uses_point_size,
                )
            }
        };
        Ok(Program {
            raw,
            outputs_srgb,
            uses_point_size,
            separable_stages: 0,
            declared_views: None,
        })
    }

    /// Starts building a program, and returns it before its shaders are compiled and linked:
    /// drivers with `GL_KHR_parallel_shader_compile` or `GL_ARB_parallel_shader_compile`, and
    /// browsers with `KHR_parallel_shader_compile`, do it in the background, and
    /// `PendingProgram::is_ready` tells when it is done, so that a program can wait for it
    /// without blocking. `PendingProgram::finish` returns the program or its errors.
    ///
    /// Elsewhere, and for the inputs other than source code, the program is built at once.
    pub fn new_pending<'a, F: ?Sized, I>(
        facade: &F,
        input: I,
    ) -> Result<PendingProgram, ProgramCreationError>
    where
        I: Into<ProgramCreationInput<'a>>,
        F: Facade,
    {
        let (
            vertex_shader,
            tessellation_control_shader,
            tessellation_evaluation_shader,
            geometry_shader,
            fragment_shader,
            transform_feedback_varyings,
            outputs_srgb,
            uses_point_size,
            locations,
        ) = match input.into() {
            ProgramCreationInput::SourceCode {
                vertex_shader,
                tessellation_control_shader,
                tessellation_evaluation_shader,
                geometry_shader,
                fragment_shader,
                transform_feedback_varyings,
                outputs_srgb,
                uses_point_size,
                locations,
            } => (
                vertex_shader,
                tessellation_control_shader,
                tessellation_evaluation_shader,
                geometry_shader,
                fragment_shader,
                transform_feedback_varyings,
                outputs_srgb,
                uses_point_size,
                locations,
            ),
            // binaries and SPIR-V modules
            #[allow(unreachable_patterns)]
            input => {
                return Program::new(facade, input).map(|program| PendingProgram {
                    context: facade.get_context().clone(),
                    state: PendingState::Built(program),
                })
            }
        };

        let mut shaders = vec![
            (vertex_shader, ShaderType::Vertex),
            (fragment_shader, ShaderType::Fragment),
        ];
        let declared_views = declared_views(vertex_shader);
        let has_geometry_shader = geometry_shader.is_some();
        let has_tessellation_control_shader = tessellation_control_shader.is_some();
        let has_tessellation_evaluation_shader = tessellation_evaluation_shader.is_some();
        if let Some(gs) = geometry_shader {
            shaders.push((stage_source(gs), ShaderType::Geometry));
        }
        if let Some(ts) = tessellation_control_shader {
            shaders.push((stage_source(ts), ShaderType::TesselationControl));
        }
        if let Some(ts) = tessellation_evaluation_shader {
            shaders.push((stage_source(ts), ShaderType::TesselationEvaluation));
        }

        let _lock = COMPILER_GLOBAL_LOCK.lock();
        let mut shaders_store = Vec::with_capacity(shaders.len());
        for (source, ty) in shaders {
            let ty = ty.to_opengl_type();
            shaders_store.push((start_shader(facade, ty, source)?, ty));
        }
        let id = RawProgram::link(
            facade,
            shaders_store.iter().map(|(shader, _)| shader),
            transform_feedback_varyings,
            Some(fragment_shader),
            locations,
            None,
        )?;

        Ok(PendingProgram {
            context: facade.get_context().clone(),
            state: PendingState::Linking {
                id,
                shaders: shaders_store,
                has_geometry_shader,
                has_tessellation_control_shader,
                has_tessellation_evaluation_shader,
                outputs_srgb,
                uses_point_size,
                declared_views,
            },
        })
    }

    /// A program linked to be used in program pipelines, with the stages of its shaders, as
    /// `glUseProgramStages` names them.
    #[cfg(any(safegl_gl41, safegl_gles31))]
    pub(crate) fn separable(
        raw: RawProgram,
        outputs_srgb: bool,
        uses_point_size: bool,
        stages: gl::types::GLbitfield,
    ) -> Program {
        Program {
            raw,
            outputs_srgb,
            uses_point_size,
            separable_stages: stages,
            declared_views: None,
        }
    }

    /// For a program linked to be used in program pipelines, the stages of its shaders, as
    /// `glUseProgramStages` names them; 0 for the other programs.
    #[cfg(any(safegl_gl41, safegl_gles31))]
    pub(crate) fn separable_stages(&self) -> gl::types::GLbitfield {
        self.separable_stages
    }

    /// The number of views that the vertex shader declares (`GL_OVR_multiview2`), `None` if it
    /// declares none, and `Err(())` if SafeGL doesn't know: for binaries, SPIR-V modules and
    /// the programs of program pipelines.
    pub(crate) fn declared_views(&self) -> Result<Option<u32>, ()> {
        self.declared_views.ok_or(())
    }

    /// Returns the program, which knows its uniform values.
    #[cfg(any(safegl_gl41, safegl_gles31))]
    pub(crate) fn raw(&self) -> &RawProgram {
        &self.raw
    }

    /// Builds a new program from GLSL source code.
    ///
    /// A program is a group of shaders linked together.
    ///
    /// # Parameters
    ///
    /// - `vertex_shader`: Source code of the vertex shader.
    /// - `fragment_shader`: Source code of the fragment shader.
    /// - `geometry_shader`: Source code of the geometry shader, which needs a desktop OpenGL
    ///   level or `gles32` (see [`StageSource`](crate::program::StageSource)).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
    /// # fn example<T>(display: safegl::Display<T>) where T: SurfaceTypeTrait + ResizeableSurface {
    /// # let vertex_source = ""; let fragment_source = "";
    /// let program = safegl::Program::from_source(&display, vertex_source, fragment_source, None);
    /// # }
    /// ```
    ///
    #[inline]
    pub fn from_source<'a, F: ?Sized>(
        facade: &F,
        vertex_shader: &'a str,
        fragment_shader: &'a str,
        geometry_shader: Option<StageSource<'a>>,
    ) -> Result<Program, ProgramCreationError>
    where
        F: Facade,
    {
        Program::new(
            facade,
            ProgramCreationInput::SourceCode {
                vertex_shader,
                fragment_shader,
                geometry_shader,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                transform_feedback_varyings: None,
                outputs_srgb: true,
                uses_point_size: false,
                locations: Default::default(),
            },
        )
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
    #[inline]
    pub fn get_frag_data_location(&self, name: &str) -> Option<u32> {
        self.raw.get_frag_data_location(name)
    }

    /// Returns the index of a fragment output: 1 for the second color output of dual-source
    /// blending, declared with `layout(location = 0, index = 1)`, and 0 for the others. Returns
    /// `None` if the program has no output with this name.
    ///
    /// The outputs at index 1 are blended with the `*SourceOne*` factors of
    /// `LinearBlendingFactor`.
    #[inline]
    pub fn get_frag_data_index(&self, name: &str) -> Option<u32> {
        self.raw.get_frag_data_index(name)
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

    /// Returns the list of transform feedback varyings.
    #[inline]
    pub fn get_transform_feedback_buffers(&self) -> &[TransformFeedbackBuffer] {
        self.raw.get_transform_feedback_buffers()
    }

    /// True if the transform feedback output of this program matches the specified `VertexFormat`
    /// and `stride`.
    ///
    /// The `stride` is the number of bytes between two vertices.
    #[inline]
    pub fn transform_feedback_matches(&self, format: &VertexFormat, stride: usize) -> bool {
        self.raw.transform_feedback_matches(format, stride)
    }

    /// Returns the type of geometry that transform feedback would generate, or `None` if it
    /// depends on the vertex/index data passed when drawing.
    ///
    /// This corresponds to `GL_GEOMETRY_OUTPUT_TYPE` or `GL_TESS_GEN_MODE`. If the program doesn't
    /// contain either a geometry shader or a tessellation evaluation shader, returns `None`.
    #[inline]
    pub fn get_output_primitives(&self) -> Option<OutputPrimitives> {
        self.raw.get_output_primitives()
    }

    /// Returns the type of primitives that the geometry shader takes as input, such as
    /// `GL_TRIANGLES`, or `None` if the program has no geometry shader.
    #[inline]
    pub(crate) fn get_geometry_input_type(&self) -> Option<gl::types::GLenum> {
        self.raw.get_geometry_input_type()
    }

    /// Returns true if the program contains a tessellation stage.
    #[inline]
    pub fn has_tessellation_shaders(&self) -> bool {
        self.raw.has_tessellation_shaders()
    }

    /// Returns true if the program contains a tessellation control stage.
    #[inline]
    pub fn has_tessellation_control_shader(&self) -> bool {
        self.raw.has_tessellation_control_shader()
    }

    /// Returns true if the program contains a tessellation evaluation stage.
    #[inline]
    pub fn has_tessellation_evaluation_shader(&self) -> bool {
        self.raw.has_tessellation_evaluation_shader()
    }

    /// Returns true if the program contains a geometry shader.
    #[inline]
    pub fn has_geometry_shader(&self) -> bool {
        self.raw.has_geometry_shader()
    }

    /// Returns informations about an attribute, if it exists.
    #[inline]
    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.raw.get_attribute(name)
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
        self.raw.attributes()
    }

    /// Returns true if the program has been configured to output sRGB instead of RGB.
    #[inline]
    pub fn has_srgb_output(&self) -> bool {
        self.outputs_srgb
    }

    /// Returns the list of shader storage blocks.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # fn example(program: safegl::Program) {
    /// for (name, uniform) in program.get_shader_storage_blocks() {
    ///     println!("Name: {}", name);
    /// }
    /// # }
    /// ```
    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
    #[inline]
    pub fn get_shader_storage_blocks(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        self.raw.get_shader_storage_blocks()
    }

    /// Returns the list of atomic counters.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # fn example(program: safegl::Program) {
    /// for (name, uniform) in program.get_atomic_counters() {
    ///     println!("Name: {}", name);
    /// }
    /// # }
    /// ```
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    #[inline]
    pub fn get_atomic_counters(
        &self,
    ) -> &HashMap<String, UniformBlock, BuildHasherDefault<FnvHasher>> {
        self.raw.get_atomic_counters()
    }

    /// Returns the subroutine uniforms of this program.
    ///
    /// Since subroutine uniforms are unique per shader and *not* per program,
    /// the keys of the `HashMap` are in the format `("subroutine_name", ShaderStage)`.
    /// ## Example
    ///
    /// ```no_run
    /// # fn example(program: safegl::Program) {
    /// for (&(ref name, shader), uniform) in program.get_subroutine_uniforms() {
    ///     println!("Name: {}", name);
    /// }
    /// # }
    /// ```
    #[inline]
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub fn get_subroutine_uniforms(
        &self,
    ) -> &HashMap<(String, ShaderStage), SubroutineUniform, BuildHasherDefault<FnvHasher>> {
        &self.raw.get_subroutine_data().subroutine_uniforms
    }

    /// Returns true if the program has been configured to use the `gl_PointSize` variable.
    ///
    /// If the program uses `gl_PointSize` without having been configured appropriately, then
    /// setting the value of `gl_PointSize` will have no effect.
    #[inline]
    pub fn uses_point_size(&self) -> bool {
        self.uses_point_size
    }
}

/// Enables `GL_PROGRAM_POINT_SIZE` for the programs that write `gl_PointSize` and disables it for
/// the others, and converts the colors that the programs write to sRGB targets unless they
/// output sRGB themselves.
pub(crate) fn apply_output_state(
    ctxt: &mut CommandContext<'_>,
    uses_point_size: bool,
    outputs_srgb: bool,
) {
    // compatibility was checked at program creation
    if ctxt.version.0 == Api::Gl && ctxt.state.enabled_program_point_size != uses_point_size {
        ctxt.state.enabled_program_point_size = uses_point_size;
        unsafe {
            if uses_point_size {
                ctxt.glow.enable(glow::PROGRAM_POINT_SIZE);
            } else {
                ctxt.glow.disable(glow::PROGRAM_POINT_SIZE);
            }
        }
    }

    if (ctxt.version.0 == Api::Gl || ctxt.extensions.gl_ext_srgb_write_control)
        && ctxt.state.enabled_framebuffer_srgb == outputs_srgb
    {
        ctxt.state.enabled_framebuffer_srgb = !outputs_srgb;

        if outputs_srgb {
            unsafe { ctxt.glow.disable(glow::FRAMEBUFFER_SRGB) };
        } else {
            unsafe { ctxt.glow.enable(glow::FRAMEBUFFER_SRGB) };
        }
    }
}

impl fmt::Debug for Program {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "{:?}", self.raw)
    }
}

#[cfg(any(safegl_gl43, safegl_gles30))]
impl crate::debug::sealed::Labeled for Program {
    fn labeled_object(
        &self,
    ) -> (&crate::context::Context, crate::debug::sealed::LabeledObject) {
        (
            ProgramExt::get_context(self),
            crate::debug::sealed::LabeledObject::Object(crate::gl::PROGRAM, self.get_id().0.get()),
        )
    }
}

impl GlObject for Program {
    type Id = glow::Program;

    #[inline]
    fn get_id(&self) -> glow::Program {
        self.raw.get_id()
    }
}

impl ProgramExt for Program {
    #[inline]
    fn get_context(&self) -> &crate::context::Context {
        self.raw.get_context()
    }

    fn use_program(&self, ctxt: &mut CommandContext<'_>) {
        apply_output_state(ctxt, self.uses_point_size, self.outputs_srgb);
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

/// A program whose shaders may still be compiling and linking in the background. See
/// `Program::new_pending`.
pub struct PendingProgram {
    context: Rc<Context>,
    state: PendingState,
}

enum PendingState {
    /// The program is linking.
    Linking {
        id: glow::Program,
        // the shaders, with their `GLenum` types, whose errors `finish` returns
        shaders: Vec<(Shader, gl::types::GLenum)>,
        has_geometry_shader: bool,
        has_tessellation_control_shader: bool,
        has_tessellation_evaluation_shader: bool,
        outputs_srgb: bool,
        uses_point_size: bool,
        declared_views: Option<u32>,
    },
    /// The program was built at once.
    Built(Program),
    /// `finish` took the program.
    Finished,
}

impl PendingProgram {
    /// Returns true if the compilation and the link of the program are done, so that `finish`
    /// doesn't wait. Without parallel compilation, it is always true.
    pub fn is_ready(&self) -> bool {
        let PendingState::Linking { id, .. } = self.state else {
            return true;
        };
        let ctxt = self.context.make_current();
        if ctxt.objects_lost() || !is_parallel_compile_supported(&ctxt) {
            return true;
        }
        unsafe { ctxt.glow.get_program_completion_status(id) }
    }

    /// Returns the program, after waiting for its compilation and its link if they aren't done,
    /// or the error of the first shader that didn't compile, or the error of the link.
    pub fn finish(mut self) -> Result<Program, ProgramCreationError> {
        match mem::replace(&mut self.state, PendingState::Finished) {
            PendingState::Built(program) => Ok(program),
            PendingState::Finished => unreachable!("a pending program is finished once"),
            PendingState::Linking {
                id,
                shaders,
                has_geometry_shader,
                has_tessellation_control_shader,
                has_tessellation_evaluation_shader,
                outputs_srgb,
                uses_point_size,
                declared_views,
            } => {
                let linked = unsafe {
                    let mut ctxt = self.context.make_current();
                    if ctxt.objects_lost() {
                        Err(ProgramCreationError::ContextLost)
                    } else if ctxt.glow.get_program_link_status(id) {
                        Ok(())
                    } else {
                        // the shaders that didn't compile, then the link
                        shaders
                            .iter()
                            .try_for_each(|(shader, ty)| check_shader(&ctxt, shader, *ty))
                            .and_then(|()| check_program_link_errors(&mut ctxt, id))
                    }
                };
                if let Err(error) = linked {
                    self.delete(id);
                    return Err(error);
                }
                drop(shaders);

                let raw = RawProgram::from_linked(
                    &self.context,
                    id,
                    has_geometry_shader,
                    has_tessellation_control_shader,
                    has_tessellation_evaluation_shader,
                    None,
                    None,
                )?;
                Ok(Program {
                    raw,
                    outputs_srgb,
                    uses_point_size,
                    separable_stages: 0,
                    declared_views: Some(declared_views),
                })
            }
        }
    }

    /// Deletes the program, which is linking or failed.
    fn delete(&self, id: glow::Program) {
        let ctxt = self.context.make_current();
        if !ctxt.objects_lost() {
            unsafe { ctxt.glow.delete_program(id) };
        }
    }
}

impl Drop for PendingProgram {
    fn drop(&mut self) {
        if let PendingState::Linking { id, .. } = self.state {
            self.delete(id);
        }
    }
}

impl fmt::Debug for PendingProgram {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let state = match self.state {
            PendingState::Linking { .. } => "linking",
            PendingState::Built(_) | PendingState::Finished => "built",
        };
        write!(formatter, "PendingProgram({state})")
    }
}

/// Returns true if the backend compiles and links in the background, and reports when it is done
/// (`GL_KHR_parallel_shader_compile`, `GL_ARB_parallel_shader_compile`, or
/// `KHR_parallel_shader_compile` in browsers).
pub(crate) fn is_parallel_compile_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.extensions.gl_khr_parallel_shader_compile
        || ctxt.extensions.gl_arb_parallel_shader_compile
}
