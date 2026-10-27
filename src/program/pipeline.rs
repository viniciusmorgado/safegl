//! Programs linked separately and combined into program pipelines, and the programs that draw
//! commands use.

#[cfg(any(safegl_gl41, safegl_gles31))]
use std::error::Error;
#[cfg(any(safegl_gl41, safegl_gles31))]
use std::fmt;
#[cfg(any(safegl_gl41, safegl_gles31))]
use std::ops::Deref;
#[cfg(any(safegl_gl41, safegl_gles31))]
use std::rc::Rc;

#[cfg(any(safegl_gl41, safegl_gles31))]
use crate::backend::Facade;
#[cfg(any(safegl_gl41, safegl_gles31))]
use crate::context::Context;
use crate::gl;
#[cfg(any(safegl_gl41, safegl_gles31))]
use crate::program::raw::RawProgram;
#[cfg(any(safegl_gl41, safegl_gles31))]
use crate::program::shader::build_shader;
#[cfg(safegl_gl46)]
use crate::program::shader::build_spirv_shader;
#[cfg(safegl_gl46)]
use crate::program::SpirvEntryPoint;
#[cfg(any(safegl_gl41, safegl_gles31))]
use crate::program::{
    LinkLocations, ProgramCreationError, ShaderType, TransformFeedbackMode, COMPILER_GLOBAL_LOCK,
};
use crate::program::{OutputPrimitives, Program};
#[cfg(any(safegl_gl41, safegl_gles31))]
use crate::{ContextExt, GlObject, ProgramExt, RawUniformValue};
#[cfg(any(safegl_gl41, safegl_gles31))]
use glow::HasContext;

/// The shaders of a separable program, written in GLSL, and how to link them. Every stage is
/// optional, see `SeparableProgram`.
#[cfg(any(safegl_gl41, safegl_gles31))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles31))))]
#[derive(Clone, Debug)]
pub struct SeparableSource<'a> {
    /// Source code of the vertex shader.
    pub vertex_shader: Option<&'a str>,
    /// Source code of the tessellation control shader.
    pub tessellation_control_shader: Option<&'a str>,
    /// Source code of the tessellation evaluation shader.
    pub tessellation_evaluation_shader: Option<&'a str>,
    /// Source code of the geometry shader.
    pub geometry_shader: Option<&'a str>,
    /// Source code of the fragment shader.
    pub fragment_shader: Option<&'a str>,
    /// The varyings to capture with transform feedback, and how.
    pub transform_feedback_varyings: Option<(Vec<String>, TransformFeedbackMode)>,
    /// Whether the fragment shader outputs sRGB colors itself, see
    /// [`ProgramCreationInput`](crate::program::ProgramCreationInput).
    /// Only the program of the fragment stage of a pipeline decides it.
    pub outputs_srgb: bool,
    /// Whether the shaders write `gl_PointSize`. Only the program of the last stage before
    /// rasterization of a pipeline decides it.
    pub uses_point_size: bool,
    /// The locations that the linker gives to attributes and fragment outputs.
    pub locations: LinkLocations<'a>,
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl Default for SeparableSource<'_> {
    fn default() -> Self {
        SeparableSource {
            vertex_shader: None,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
            locations: LinkLocations::default(),
        }
    }
}

/// A program linked to be combined with other programs into a `ProgramPipeline`, which gives
/// each stage of the pipeline the program that has it (`GL_PROGRAM_SEPARABLE`).
///
/// It has the shaders of one or several stages, and dereferences to the `Program` that
/// describes its uniforms, attributes and outputs. On desktop OpenGL, the shaders of the
/// stages before rasterization that write `gl_Position` redeclare the `gl_PerVertex` block.
///
/// Needs OpenGL 4.1, `GL_ARB_separate_shader_objects` or OpenGL ES 3.1.
///
/// ```no_run
/// # fn example(display: &safegl::Display<safegl::glutin::surface::WindowSurface>,
/// #            vertex: &str, fragment: &str) {
/// use safegl::program::{ProgramPipeline, SeparableProgram, ShaderType};
///
/// let vertex = SeparableProgram::from_source(display, ShaderType::Vertex, vertex).unwrap();
/// let fragment = SeparableProgram::from_source(display, ShaderType::Fragment, fragment).unwrap();
/// let pipeline = ProgramPipeline::new(display, &[&vertex, &fragment]).unwrap();
/// // `pipeline` is drawn with like a program: `target.draw(..., &pipeline, ...)`
/// # }
/// ```
#[cfg(any(safegl_gl41, safegl_gles31))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles31))))]
pub struct SeparableProgram {
    program: Program,
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl SeparableProgram {
    /// Compiles and links the shaders of `source`, which has at least one.
    pub fn new<F>(facade: &F, source: SeparableSource<'_>) -> Result<Self, ProgramCreationError>
    where
        F: Facade + ?Sized,
    {
        if !super::are_pipelines_supported(&**facade.get_context()) {
            return Err(ProgramCreationError::ProgramPipelinesNotSupported);
        }
        let stages = [
            (
                source.vertex_shader,
                ShaderType::Vertex,
                gl::VERTEX_SHADER_BIT,
            ),
            (
                source.tessellation_control_shader,
                ShaderType::TesselationControl,
                gl::TESS_CONTROL_SHADER_BIT,
            ),
            (
                source.tessellation_evaluation_shader,
                ShaderType::TesselationEvaluation,
                gl::TESS_EVALUATION_SHADER_BIT,
            ),
            (
                source.geometry_shader,
                ShaderType::Geometry,
                gl::GEOMETRY_SHADER_BIT,
            ),
            (
                source.fragment_shader,
                ShaderType::Fragment,
                gl::FRAGMENT_SHADER_BIT,
            ),
        ];
        if stages.iter().all(|stage| stage.0.is_none()) {
            return Err(ProgramCreationError::NoShader);
        }

        let _lock = COMPILER_GLOBAL_LOCK.lock();
        let mut shaders = Vec::new();
        let mut bits = 0;
        for &(code, ty, bit) in &stages {
            if let Some(code) = code {
                shaders.push(build_shader(facade, ty.to_opengl_type(), code)?);
                bits |= bit;
            }
        }
        let raw = RawProgram::from_shaders(
            facade,
            &shaders,
            source.geometry_shader.is_some(),
            source.tessellation_control_shader.is_some(),
            source.tessellation_evaluation_shader.is_some(),
            source.transform_feedback_varyings,
            source.fragment_shader,
            source.locations,
            None,
            Some(bits),
        )?;
        Ok(SeparableProgram {
            program: Program::separable(raw, source.outputs_srgb, source.uses_point_size, bits),
        })
    }

    /// Compiles and links a program that has the shader of one stage.
    pub fn from_source<F>(
        facade: &F,
        stage: ShaderType,
        code: &str,
    ) -> Result<Self, ProgramCreationError>
    where
        F: Facade + ?Sized,
    {
        let mut source = SeparableSource::default();
        match stage {
            ShaderType::Vertex => source.vertex_shader = Some(code),
            ShaderType::TesselationControl => source.tessellation_control_shader = Some(code),
            ShaderType::TesselationEvaluation => source.tessellation_evaluation_shader = Some(code),
            ShaderType::Geometry => source.geometry_shader = Some(code),
            ShaderType::Fragment => source.fragment_shader = Some(code),
            // compute shaders are dispatched with `ComputeShader`
            ShaderType::Compute => return Err(ProgramCreationError::ShaderTypeNotSupported),
        }
        SeparableProgram::new(facade, source)
    }

    /// Links a program from entry points of SPIR-V modules, one per stage. The names of its
    /// resources are the ones of `SpirvEntryPoint`.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    pub fn from_spirv<F>(
        facade: &F,
        stages: &[(ShaderType, SpirvEntryPoint<'_>)],
    ) -> Result<Self, ProgramCreationError>
    where
        F: Facade + ?Sized,
    {
        if !super::are_pipelines_supported(&**facade.get_context()) {
            return Err(ProgramCreationError::ProgramPipelinesNotSupported);
        }
        if stages.is_empty() {
            return Err(ProgramCreationError::NoShader);
        }

        let _lock = COMPILER_GLOBAL_LOCK.lock();
        let mut shaders = Vec::new();
        let mut modules = Vec::new();
        let mut bits = 0;
        for &(ty, ref entry_point) in stages {
            let bit = stage_bit(ty).ok_or(ProgramCreationError::ShaderTypeNotSupported)?;
            if bits & bit != 0 {
                return Err(ProgramCreationError::ShaderTypeNotSupported);
            }
            let (shader, module) = build_spirv_shader(facade, ty.to_opengl_type(), entry_point)?;
            shaders.push(shader);
            modules.push(module);
            bits |= bit;
        }
        let raw = RawProgram::from_shaders(
            facade,
            &shaders,
            bits & gl::GEOMETRY_SHADER_BIT != 0,
            bits & gl::TESS_CONTROL_SHADER_BIT != 0,
            bits & gl::TESS_EVALUATION_SHADER_BIT != 0,
            None,
            None,
            Default::default(),
            Some(&modules),
            Some(bits),
        )?;
        Ok(SeparableProgram {
            program: Program::separable(raw, true, false, bits),
        })
    }

    /// Returns true if the program has the shader of `stage`.
    pub fn has_stage(&self, stage: ShaderType) -> bool {
        stage_bit(stage).is_some_and(|bit| self.program.separable_stages() & bit != 0)
    }
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl Deref for SeparableProgram {
    type Target = Program;

    #[inline]
    fn deref(&self) -> &Program {
        &self.program
    }
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl fmt::Debug for SeparableProgram {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SeparableProgram")
            .field("program", &self.program)
            .finish()
    }
}

/// The bit of a stage in `glUseProgramStages`.
#[cfg(any(safegl_gl41, safegl_gles31))]
fn stage_bit(stage: ShaderType) -> Option<gl::types::GLbitfield> {
    Some(match stage {
        ShaderType::Vertex => gl::VERTEX_SHADER_BIT,
        ShaderType::TesselationControl => gl::TESS_CONTROL_SHADER_BIT,
        ShaderType::TesselationEvaluation => gl::TESS_EVALUATION_SHADER_BIT,
        ShaderType::Geometry => gl::GEOMETRY_SHADER_BIT,
        ShaderType::Fragment => gl::FRAGMENT_SHADER_BIT,
        ShaderType::Compute => return None,
    })
}

/// Error that `ProgramPipeline::new` can return.
#[cfg(any(safegl_gl41, safegl_gles31))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles31))))]
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgramPipelineCreationError {
    /// Program pipelines need OpenGL 4.1, `GL_ARB_separate_shader_objects` or OpenGL ES 3.1.
    NotSupported,
    /// A program belongs to another context.
    ContextMismatch,
    /// Two programs have the shader of the same stage.
    StageGivenTwice,
    /// No program has a vertex shader.
    NoVertexShader,
    /// OpenGL doesn't validate the pipeline, for example because the outputs of a stage don't
    /// match the inputs of the next one. The string is its log.
    ValidationFailed(String),
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl fmt::Display for ProgramPipelineCreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ProgramPipelineCreationError::*;
        match self {
            NotSupported => fmt.write_str("Program pipelines are not supported by the backend"),
            ContextMismatch => fmt.write_str("A program belongs to another context"),
            StageGivenTwice => fmt.write_str("Two programs have the shader of the same stage"),
            NoVertexShader => fmt.write_str("No program has a vertex shader"),
            ValidationFailed(log) => write!(fmt, "The pipeline isn't valid: {}", log),
        }
    }
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl Error for ProgramPipelineCreationError {}

/// Separable programs combined: each stage uses the program that has its shader
/// (`glUseProgramStages`). It is drawn with like a program, and the uniforms of a draw command
/// are given to all its programs.
///
/// See `SeparableProgram`. Needs OpenGL 4.1, `GL_ARB_separate_shader_objects` or OpenGL ES 3.1.
#[cfg(any(safegl_gl41, safegl_gles31))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles31))))]
pub struct ProgramPipeline<'a> {
    context: Rc<Context>,
    id: gl::types::GLuint,
    programs: Vec<&'a Program>,
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl<'a> ProgramPipeline<'a> {
    /// Combines programs, each of which gives the pipeline the shaders of all its stages. One
    /// of them has a vertex shader, and no stage has two.
    ///
    /// The pipeline is validated (`glValidateProgramPipeline`): the outputs of each stage must
    /// match the inputs of the next one.
    pub fn new<F>(
        facade: &F,
        programs: &[&'a SeparableProgram],
    ) -> Result<ProgramPipeline<'a>, ProgramPipelineCreationError>
    where
        F: Facade + ?Sized,
    {
        let context = facade.get_context();
        if !super::are_pipelines_supported(&**context) {
            return Err(ProgramPipelineCreationError::NotSupported);
        }
        let mut stages = 0;
        for program in programs {
            if !std::ptr::eq(ProgramExt::get_context(&***program), &**context) {
                return Err(ProgramPipelineCreationError::ContextMismatch);
            }
            if stages & program.separable_stages() != 0 {
                return Err(ProgramPipelineCreationError::StageGivenTwice);
            }
            stages |= program.separable_stages();
        }
        if stages & gl::VERTEX_SHADER_BIT == 0 {
            return Err(ProgramPipelineCreationError::NoVertexShader);
        }

        let mut ctxt = context.make_current();
        let id = unsafe {
            let mut id = 0;
            ctxt.extra.GenProgramPipelines(1, &mut id);
            for program in programs {
                ctxt.extra.UseProgramStages(
                    id,
                    program.separable_stages(),
                    program.get_id().0.get(),
                );
            }
            id
        };
        let pipeline = ProgramPipeline {
            context: context.clone(),
            id,
            programs: programs.iter().map(|program| &***program).collect(),
        };

        // validation fails while samplers of different types use the same texture unit, which
        // draw commands never do: the samplers get their own units first
        let mut unit = 0;
        for program in &pipeline.programs {
            let samplers: Vec<_> = program
                .uniforms()
                .filter(|(_, uniform)| {
                    crate::uniforms::is_opaque(uniform.ty) && uniform.binding.is_none()
                })
                .map(|(_, uniform)| uniform.location)
                .collect();
            if samplers.is_empty() {
                continue;
            }
            program.raw().use_program(&mut ctxt);
            for location in samplers {
                program.set_uniform(&mut ctxt, location, &RawUniformValue::SignedInt(unit));
                unit += 1;
            }
        }

        let valid = unsafe {
            ctxt.extra.ValidateProgramPipeline(id);
            let mut status = 0;
            ctxt.extra
                .GetProgramPipelineiv(id, gl::VALIDATE_STATUS, &mut status);
            status != 0
        };
        if !valid {
            let log = unsafe {
                let mut length = 0;
                ctxt.extra
                    .GetProgramPipelineiv(id, gl::INFO_LOG_LENGTH, &mut length);
                let mut log = vec![0u8; length.max(1) as usize];
                ctxt.extra.GetProgramPipelineInfoLog(
                    id,
                    log.len() as gl::types::GLsizei,
                    &mut length,
                    log.as_mut_ptr() as *mut _,
                );
                log.truncate(length.max(0) as usize);
                String::from_utf8_lossy(&log).into_owned()
            };
            drop(ctxt);
            return Err(ProgramPipelineCreationError::ValidationFailed(log));
        }
        drop(ctxt);
        Ok(pipeline)
    }

    /// Returns the program of a stage, if the pipeline has one.
    fn stage(&self, bit: gl::types::GLbitfield) -> Option<&'a Program> {
        self.programs
            .iter()
            .copied()
            .find(|program| program.separable_stages() & bit != 0)
    }

    /// Binds the pipeline, which is used once no program is.
    pub(crate) fn bind(&self, ctxt: &mut crate::context::CommandContext<'_>) {
        unsafe {
            if ctxt.state.program.is_some() {
                ctxt.glow.use_program(None);
                ctxt.state.program = None;
            }
            if ctxt.state.program_pipeline != self.id {
                ctxt.extra.BindProgramPipeline(self.id);
                ctxt.state.program_pipeline = self.id;
            }
        }
    }

    /// Returns the context of the pipeline.
    pub(crate) fn get_context(&self) -> &Rc<Context> {
        &self.context
    }
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl fmt::Debug for ProgramPipeline<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProgramPipeline")
            .field("id", &self.id)
            .field("programs", &self.programs)
            .finish()
    }
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl Drop for ProgramPipeline<'_> {
    fn drop(&mut self) {
        let mut ctxt = self.context.make_current();
        unsafe {
            if ctxt.state.program_pipeline == self.id {
                ctxt.extra.BindProgramPipeline(0);
                ctxt.state.program_pipeline = 0;
            }
            ctxt.extra.DeleteProgramPipelines(1, &self.id);
        }
    }
}

/// The program of a draw command: a `Program`, or a `ProgramPipeline`.
///
/// Draw commands take anything that converts into it, so a reference to either can be given.
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum DrawProgram<'a> {
    /// A program that has all the stages.
    Program(&'a Program),
    /// Separable programs combined.
    #[cfg(any(safegl_gl41, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles31))))]
    Pipeline(&'a ProgramPipeline<'a>),
}

impl<'a> From<&'a Program> for DrawProgram<'a> {
    #[inline]
    fn from(program: &'a Program) -> DrawProgram<'a> {
        DrawProgram::Program(program)
    }
}

#[cfg(any(safegl_gl41, safegl_gles31))]
impl<'a> From<&'a ProgramPipeline<'a>> for DrawProgram<'a> {
    #[inline]
    fn from(pipeline: &'a ProgramPipeline<'a>) -> DrawProgram<'a> {
        DrawProgram::Pipeline(pipeline)
    }
}

impl<'a> DrawProgram<'a> {
    /// Returns the program of the vertex stage, which gives the attributes.
    pub(crate) fn vertex(&self) -> &'a Program {
        match *self {
            DrawProgram::Program(program) => program,
            #[cfg(any(safegl_gl41, safegl_gles31))]
            DrawProgram::Pipeline(pipeline) => pipeline
                .stage(gl::VERTEX_SHADER_BIT)
                .expect("pipelines have a vertex shader"),
        }
    }

    /// Returns the program of the fragment stage, which gives the outputs, if there is one.
    pub(crate) fn fragment(&self) -> Option<&'a Program> {
        match *self {
            DrawProgram::Program(program) => Some(program),
            #[cfg(any(safegl_gl41, safegl_gles31))]
            DrawProgram::Pipeline(pipeline) => pipeline.stage(gl::FRAGMENT_SHADER_BIT),
        }
    }

    /// Returns the program of the last stage before rasterization, which transform feedback
    /// captures and which writes the sizes of points.
    pub(crate) fn last_vertex_stage(&self) -> &'a Program {
        match *self {
            DrawProgram::Program(program) => program,
            #[cfg(any(safegl_gl41, safegl_gles31))]
            DrawProgram::Pipeline(pipeline) => pipeline
                .stage(gl::GEOMETRY_SHADER_BIT)
                .or_else(|| pipeline.stage(gl::TESS_EVALUATION_SHADER_BIT))
                .unwrap_or_else(|| self.vertex()),
        }
    }

    /// Returns the programs, without duplicates.
    pub(crate) fn programs(&self) -> &[&'a Program] {
        match self {
            DrawProgram::Program(program) => std::slice::from_ref(program),
            #[cfg(any(safegl_gl41, safegl_gles31))]
            DrawProgram::Pipeline(pipeline) => &pipeline.programs,
        }
    }

    /// Returns true if a stage has a tessellation evaluation shader.
    pub(crate) fn has_tessellation_evaluation_shader(&self) -> bool {
        self.programs()
            .iter()
            .any(|program| program.has_tessellation_evaluation_shader())
    }

    /// Returns the type of primitives that the geometry shader takes, if there is one.
    pub(crate) fn get_geometry_input_type(&self) -> Option<gl::types::GLenum> {
        self.programs()
            .iter()
            .find_map(|program| program.get_geometry_input_type())
    }

    /// Returns the type of primitives that the last stage before rasterization outputs, if it
    /// isn't the vertex shader.
    pub(crate) fn get_output_primitives(&self) -> Option<OutputPrimitives> {
        self.last_vertex_stage().get_output_primitives()
    }

    /// Makes the programs ready for a draw command: binds the program, or the pipeline, and
    /// sets the state that the programs decide (the size of points and sRGB conversion).
    pub(crate) fn use_programs(&self, ctxt: &mut crate::context::CommandContext<'_>) {
        match *self {
            DrawProgram::Program(program) => crate::ProgramExt::use_program(program, ctxt),
            #[cfg(any(safegl_gl41, safegl_gles31))]
            DrawProgram::Pipeline(pipeline) => {
                let outputs_srgb = self.fragment().is_none_or(Program::has_srgb_output);
                crate::program::program::apply_output_state(
                    ctxt,
                    self.last_vertex_stage().uses_point_size(),
                    outputs_srgb,
                );
                pipeline.bind(ctxt);
            }
        }
    }
}
