//! Items related to creating an OpenGL program.

use crate::CapabilitiesSource;
use std::error::Error;
use std::fmt;
use std::sync::Mutex;

use crate::gl;
#[cfg(any(safegl_desktop, safegl_gles30))]
use crate::version::Api;
#[cfg(any(safegl_gl41, safegl_gles30))]
use crate::version::Version;

#[cfg(any(safegl_gl43, safegl_gles31))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles31))))]
pub use self::compute::{ComputeCommand, ComputeShader};
pub use self::program::Program;
pub use self::reflection::ShaderStage;
#[cfg(not(safegl_desktop))]
pub(crate) use self::reflection::SubroutineData;
pub use self::reflection::{
    Attribute, TransformFeedbackBuffer, TransformFeedbackMode, TransformFeedbackVarying,
};
pub use self::reflection::{BlockLayout, OutputPrimitives, Uniform, UniformBlock};
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
pub use self::reflection::{SubroutineData, SubroutineUniform};

#[cfg(any(safegl_gl41, safegl_gles30))]
mod binary_header;
#[cfg(any(safegl_gl43, safegl_gles31))]
mod compute;
mod program;
mod raw;
mod reflection;
mod shader;
mod uniforms_storage;

/// Returns true if the backend supports geometry shaders.
#[inline]
pub fn is_geometry_shader_supported<C: ?Sized>(ctxt: &C) -> bool
where
    C: CapabilitiesSource,
{
    shader::check_shader_type_compatibility(ctxt, gl::GEOMETRY_SHADER)
}

/// Returns true if the backend supports tessellation shaders.
#[inline]
pub fn is_tessellation_shader_supported<C: ?Sized>(ctxt: &C) -> bool
where
    C: CapabilitiesSource,
{
    shader::check_shader_type_compatibility(ctxt, gl::TESS_CONTROL_SHADER)
}

/// Returns true if the backend supports creating and retrieving binary format.
#[inline]
#[cfg(any(safegl_gl41, safegl_gles30))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
pub fn is_binary_supported<C: ?Sized>(ctxt: &C) -> bool
where
    C: CapabilitiesSource,
{
    // program binaries are core in OpenGL 4.1 and OpenGL ES 3.0, but don't exist on WebGL
    !crate::version::is_webgl()
        && (ctxt.get_version().0 == Api::GlEs
            || ctxt.get_version() >= &Version(Api::Gl, 4, 1)
            || ctxt.get_extensions().gl_arb_get_program_binary)
}

/// Returns true if the backend supports shader subroutines.
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
#[inline]
pub fn is_subroutine_supported<C: ?Sized>(ctxt: &C) -> bool
where
    C: CapabilitiesSource,
{
    // core in OpenGL 4.0, and not part of OpenGL ES
    ctxt.get_version().0 == Api::Gl
}

// Some shader compilers have race-condition issues, so we lock this mutex
// in the GL thread every time we compile a shader or link a program.
static COMPILER_GLOBAL_LOCK: Mutex<()> = Mutex::new(());

/// Used in ProgramCreationError::CompilationError to explain which shader stage failed compilation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShaderType {
    /// Vertex shader, maps to gl::VERTEX_SHADER
    Vertex,
    /// Geometry shader, maps to gl::GEOMETRY_SHADER
    Geometry,
    /// Fragment shader, maps to gl::FRAGMENT_SHADER
    Fragment,
    /// Tesselation control shader, maps to gl::TESS_CONTROL_SHADER
    TesselationControl,
    /// Tesselation evaluation shader, maps to gl::TESS_EVALUATION_SHADER
    TesselationEvaluation,
    /// Compute shader, maps to gl::COMPUTE_SHADER
    Compute,
}

/// A precision qualifier of GLSL ES and of GLSL 4.10 and above, for
/// `Context::shader_precision_format`.
#[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrecisionQualifier {
    /// `lowp float`.
    LowFloat,
    /// `mediump float`.
    MediumFloat,
    /// `highp float`.
    HighFloat,
    /// `lowp int`.
    LowInt,
    /// `mediump int`.
    MediumInt,
    /// `highp int`.
    HighInt,
}

#[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
impl PrecisionQualifier {
    pub(crate) fn to_glenum(self) -> gl::types::GLenum {
        match self {
            PrecisionQualifier::LowFloat => gl::LOW_FLOAT,
            PrecisionQualifier::MediumFloat => gl::MEDIUM_FLOAT,
            PrecisionQualifier::HighFloat => gl::HIGH_FLOAT,
            PrecisionQualifier::LowInt => gl::LOW_INT,
            PrecisionQualifier::MediumInt => gl::MEDIUM_INT,
            PrecisionQualifier::HighInt => gl::HIGH_INT,
        }
    }
}

/// The range and the precision of the numbers of a precision qualifier in a shader stage
/// (`glGetShaderPrecisionFormat`).
#[cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30, safegl_webgl))))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ShaderPrecision {
    /// The base 2 logarithms of the absolute values of the smallest and the largest numbers: the
    /// numbers are in `-2^range.0` to `2^range.1`.
    pub range: (i32, i32),
    /// The number of bits of precision of floats: their relative precision is `2^-precision`. 0
    /// for integers.
    pub precision: i32,
}

impl ShaderType {
    /// Creates an instance of gl::types::GLenum corresponding to the given ShaderType
    pub fn to_opengl_type(self) -> gl::types::GLenum {
        match self {
            ShaderType::Vertex => gl::VERTEX_SHADER,
            ShaderType::Geometry => gl::GEOMETRY_SHADER,
            ShaderType::Fragment => gl::FRAGMENT_SHADER,
            ShaderType::TesselationControl => gl::TESS_CONTROL_SHADER,
            ShaderType::TesselationEvaluation => gl::TESS_EVALUATION_SHADER,
            ShaderType::Compute => gl::COMPUTE_SHADER,
        }
    }
    /// Creates an instance of ShaderType corresponding to the given gl::types::GLenum.
    /// This routine will panic if the given shadertype is not supported by SafeGL.
    pub fn from_opengl_type(gl_type: gl::types::GLenum) -> Self {
        match gl_type {
            gl::VERTEX_SHADER => ShaderType::Vertex,
            gl::GEOMETRY_SHADER => ShaderType::Geometry,
            gl::FRAGMENT_SHADER => ShaderType::Fragment,
            gl::TESS_CONTROL_SHADER => ShaderType::TesselationControl,
            gl::TESS_EVALUATION_SHADER => ShaderType::TesselationEvaluation,
            gl::COMPUTE_SHADER => ShaderType::Compute,
            _ => {
                panic!("Unsupported shader type")
            }
        }
    }
}

/// Error that can be triggered when creating a `Program`.
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ProgramCreationError {
    /// Error while compiling one of the shaders.
    CompilationError(String, ShaderType),

    /// Error while linking the program.
    LinkingError(String),

    /// One of the requested shader types is not supported by the backend.
    ///
    /// Usually the case for geometry shaders.
    ShaderTypeNotSupported,

    /// The OpenGL implementation doesn't provide a compiler.
    CompilationNotSupported,

    /// The SafeGL-specific binary header was not found or is corrupt.
    #[cfg(any(safegl_gl41, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
    BinaryHeaderError,

    /// The backend doesn't support loading program binaries (WebGL). See `is_binary_supported`.
    BinaryNotSupported,

    /// A location of `LinkLocations` can't be given: it is too large for the implementation,
    /// the index of a fragment output isn't 0 or 1, or the name starts with `gl_` or holds a nul
    /// character.
    InvalidLinkLocation {
        /// The name of the attribute or output.
        name: String,
    },

    /// Locations of fragment outputs can't be given before linking: OpenGL ES needs
    /// `GL_EXT_blend_func_extended`, and WebGL can't. Declare them with `layout(location = N)`.
    LinkLocationsNotSupported,
}

impl fmt::Display for ProgramCreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use self::ProgramCreationError::*;
        let desc = match *self {
            CompilationError(_, typ) => match typ {
                ShaderType::Vertex => "Compilation error in vertex shader",
                ShaderType::Geometry => "Compilation error in geometry shader",
                ShaderType::Fragment => "Compilation error in fragment shader",
                ShaderType::TesselationControl => "Compilation error in tesselation control shader",
                ShaderType::TesselationEvaluation => {
                    "Compilation error in tesselation evaluation shader"
                }
                ShaderType::Compute => "Compilation error in compute shader",
            },
            LinkingError(_) => "Error while linking shaders together",
            ShaderTypeNotSupported => {
                "One of the request shader type is not supported by the backend"
            }
            CompilationNotSupported => "The backend doesn't support shaders compilation",
            #[cfg(any(safegl_gl41, safegl_gles30))]
            BinaryHeaderError => "The SafeGL-specific binary header was not found or is corrupt.",
            BinaryNotSupported => "Program binaries are not supported by the backend.",
            InvalidLinkLocation { .. } => "A location given before linking is invalid",
            LinkLocationsNotSupported => {
                "Fragment output locations can't be given before linking by the backend"
            }
        };
        match *self {
            CompilationError(ref s, _) => write!(fmt, "{}: {}", desc, s),
            LinkingError(ref s) => write!(fmt, "{}: {}", desc, s),
            InvalidLinkLocation { ref name } => write!(fmt, "{}: `{}`", desc, name),
            _ => write!(fmt, "{}", desc),
        }
    }
}

impl Error for ProgramCreationError {}

/// Error type that is returned by the `program!` macro.
#[derive(Clone, Debug)]
pub enum ProgramChooserCreationError {
    /// No available version has been found.
    NoVersion,

    /// A version has been found but it triggered the given error.
    ProgramCreationError(ProgramCreationError),
}

impl fmt::Display for ProgramChooserCreationError {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use self::ProgramChooserCreationError::*;
        match *self {
            ProgramCreationError(ref err) => write!(fmt, "{}", err),
            NoVersion => fmt.write_str(
                "No version of the program has been found for the current OpenGL version.",
            ),
        }
    }
}

impl Error for ProgramChooserCreationError {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::ProgramChooserCreationError::*;
        match *self {
            ProgramCreationError(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<ProgramCreationError> for ProgramChooserCreationError {
    fn from(err: ProgramCreationError) -> ProgramChooserCreationError {
        ProgramChooserCreationError::ProgramCreationError(err)
    }
}

/// Error while retrieving the binary representation of a program.
#[derive(Copy, Clone, Debug)]
#[cfg(any(safegl_gl41, safegl_gles30))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
pub enum GetBinaryError {
    /// The backend doesn't support binary.
    NotSupported,
    /// The backend does not supply any binary formats.
    NoFormats,
}

#[cfg(any(safegl_gl41, safegl_gles30))]
impl fmt::Display for GetBinaryError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::GetBinaryError::*;
        let desc = match *self {
            NotSupported => "The backend doesn't support binary",
            NoFormats => "The backend does not supply any binary formats.",
        };
        fmt.write_str(desc)
    }
}

#[cfg(any(safegl_gl41, safegl_gles30))]
impl Error for GetBinaryError {}

/// The source code of a geometry or tessellation shader in `ProgramCreationInput::SourceCode`
/// and `Program::from_source`.
///
/// These stages need a desktop OpenGL level or `gles32`. At the other levels this is
/// [`NotInLevel`], which has no values: only `None` can be given, and a source code doesn't
/// compile.
#[cfg(any(safegl_desktop, safegl_gles32))]
pub type StageSource<'a> = &'a str;

/// The source code of a geometry or tessellation shader in `ProgramCreationInput::SourceCode`
/// and `Program::from_source`.
///
/// These stages need a desktop OpenGL level or `gles32`. At the other levels this is
/// [`NotInLevel`], which has no values: only `None` can be given, and a source code doesn't
/// compile.
#[cfg(not(any(safegl_desktop, safegl_gles32)))]
pub type StageSource<'a> = NotInLevel;

/// A shader stage that the selected levels don't have. It has no values, so that only `None`
/// can be given for such a stage.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NotInLevel {}

/// The source code of an optional shader stage.
#[cfg(any(safegl_desktop, safegl_gles32))]
pub(crate) fn stage_source(source: StageSource<'_>) -> &str {
    source
}

/// The source code of an optional shader stage, which can't exist at these levels.
#[cfg(not(any(safegl_desktop, safegl_gles32)))]
pub(crate) fn stage_source<'a>(source: StageSource<'a>) -> &'a str {
    match source {}
}

/// Input when creating a program.
pub enum ProgramCreationInput<'a> {
    /// Use GLSL source code.
    SourceCode {
        /// Source code of the vertex shader.
        vertex_shader: &'a str,

        /// Source code of the optional tessellation control shader. See [`StageSource`].
        tessellation_control_shader: Option<StageSource<'a>>,

        /// Source code of the optional tessellation evaluation shader. See [`StageSource`].
        tessellation_evaluation_shader: Option<StageSource<'a>>,

        /// Source code of the optional geometry shader. See [`StageSource`].
        geometry_shader: Option<StageSource<'a>>,

        /// Source code of the fragment shader.
        fragment_shader: &'a str,

        /// The list of variables and mode to use for transform feedback.
        ///
        /// The information specified here will be passed to the OpenGL linker. If you pass
        /// `None`, then you won't be able to use transform feedback.
        transform_feedback_varyings: Option<(Vec<String>, TransformFeedbackMode)>,

        /// Whether the fragment shader outputs colors in `sRGB` or `RGB`. This is true by default,
        /// meaning that the program is responsible for outputting correct `sRGB` values.
        ///
        /// If this is false, then `GL_FRAMEBUFFER_SRGB` will be enabled when this program is used,
        /// and the colors that it writes to sRGB targets are converted from linear RGB. OpenGL ES
        /// always converts them without `GL_EXT_sRGB_write_control`. See [the sRGB section of
        /// the textures](crate::texture#about-srgb).
        outputs_srgb: bool,

        /// Whether the shader uses point size.
        uses_point_size: bool,

        /// The locations that the linker gives to attributes and fragment outputs that the
        /// shaders don't declare with `layout(location = N)`. See [`LinkLocations`].
        locations: LinkLocations<'a>,
    },

    /// Use a precompiled binary.
    #[cfg(any(safegl_gl41, safegl_gles30))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
    Binary {
        /// The data.
        data: Binary,

        /// See [`ProgramCreationInput::SourceCode::outputs_srgb`].
        outputs_srgb: bool,

        /// Whether the shader uses point size.
        uses_point_size: bool,
    },

    /// Use a SPIR-V binary.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    SpirV(SpirvProgram<'a>),
}

/// Represents a SPIR-V program. The shaders can refer to entry points in the same binary.
#[cfg(safegl_gl46)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
#[derive(Clone)]
pub struct SpirvProgram<'a> {
    /// The vertex shader.
    pub vertex_shader: SpirvEntryPoint<'a>,

    /// The fragment shader.
    pub fragment_shader: SpirvEntryPoint<'a>,

    /// Optional tessellation control shader.
    pub tessellation_control_shader: Option<SpirvEntryPoint<'a>>,

    /// Optional tessellation evaluation shader.
    pub tessellation_evaluation_shader: Option<SpirvEntryPoint<'a>>,

    /// Optional geometry shader.
    pub geometry_shader: Option<SpirvEntryPoint<'a>>,

    /// The list of variables and mode to use for transform feedback.
    ///
    /// The information specified here will be passed to the OpenGL linker. If you pass
    /// `None`, then you won't be able to use transform feedback.
    pub transform_feedback_varyings: Option<(Vec<String>, TransformFeedbackMode)>,

    /// See [`ProgramCreationInput::SourceCode::outputs_srgb`].
    pub outputs_srgb: bool,

    /// Whether the shader uses point size.
    pub uses_point_size: bool,
}

#[cfg(safegl_gl46)]
impl<'a> SpirvProgram<'a> {
    /// Create new `SpirvProgram` from vertex and fragment shaders.
    pub fn from_vs_and_fs(
        vertex_shader: SpirvEntryPoint<'a>,
        fragment_shader: SpirvEntryPoint<'a>,
    ) -> Self {
        Self {
            vertex_shader,
            fragment_shader,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
        }
    }

    /// Builder method to set `tessellation_control_shader`.
    pub fn tessellation_control_shader(
        mut self,
        tessellation_control_shader: Option<SpirvEntryPoint<'a>>,
    ) -> Self {
        self.tessellation_control_shader = tessellation_control_shader;
        self
    }

    /// Builder method to set `tessellation_evaluation_shader`.
    pub fn tessellation_evaluation_shader(
        mut self,
        tessellation_evaluation_shader: Option<SpirvEntryPoint<'a>>,
    ) -> Self {
        self.tessellation_evaluation_shader = tessellation_evaluation_shader;
        self
    }

    /// Builder method to set `geometry_shader`.
    pub fn geometry_shader(mut self, geometry_shader: Option<SpirvEntryPoint<'a>>) -> Self {
        self.geometry_shader = geometry_shader;
        self
    }

    /// Builder method to set `transform_feedback_varyings`.
    pub fn transform_feedback_varyings(
        mut self,
        transform_feedback_varyings: Option<(Vec<String>, TransformFeedbackMode)>,
    ) -> Self {
        self.transform_feedback_varyings = transform_feedback_varyings;
        self
    }

    /// Builder method to set `outputs_srgb`.
    pub fn outputs_srgb(mut self, outputs_srgb: bool) -> Self {
        self.outputs_srgb = outputs_srgb;
        self
    }

    /// Builder method to set `uses_point_size`.
    pub fn uses_point_size(mut self, uses_point_size: bool) -> Self {
        self.uses_point_size = uses_point_size;
        self
    }
}

/// Locations that the linker gives to the attributes and the fragment outputs of a program whose
/// shaders don't declare them with `layout(location = N)` (`glBindAttribLocation` and
/// `glBindFragDataLocationIndexed`). The locations declared in the shaders win.
///
/// ```no_run
/// # fn example(display: &safegl::Display<safegl::glutin::surface::WindowSurface>,
/// #            vertex_shader: &str, fragment_shader: &str) {
/// use safegl::program::{LinkLocations, ProgramCreationInput};
///
/// let input = ProgramCreationInput::SourceCode {
///     vertex_shader,
///     fragment_shader,
///     tessellation_control_shader: None,
///     tessellation_evaluation_shader: None,
///     geometry_shader: None,
///     transform_feedback_varyings: None,
///     outputs_srgb: true,
///     uses_point_size: false,
///     locations: LinkLocations {
///         attributes: &[("position", 0), ("normal", 1)],
///         // the two colors of dual-source blending
///         fragment_outputs: &[("color", 0, 0), ("blend_factor", 0, 1)],
///     },
/// };
/// let program = safegl::Program::new(display, input).unwrap();
/// # }
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkLocations<'a> {
    /// The names of attributes and their locations, smaller than
    /// `Limits::max_vertex_attribs`. A matrix takes a location per column.
    pub attributes: &'a [(&'a str, u32)],

    /// The names of fragment outputs, their locations and their indices: index 1 is the second
    /// color of dual-source blending, at a location smaller than
    /// `Limits::max_dual_source_draw_buffers`, and index 0 the other outputs, at locations
    /// smaller than the number of draw buffers. An array takes a location per element.
    ///
    /// The outputs that have no location get the free ones, in their declaration order.
    /// OpenGL ES needs `GL_EXT_blend_func_extended`, which a GLSL ES fragment shader with
    /// several outputs without a location enables with `#extension`, and WebGL can't give them.
    pub fragment_outputs: &'a [(&'a str, u32, u32)],
}

/// Represents an entry point of a binary SPIR-V module.
#[cfg(safegl_gl46)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
#[derive(Copy, Clone)]
pub struct SpirvEntryPoint<'a> {
    /// The binary module data.
    pub binary: &'a [u8],

    /// The entry point to use, e.g. "main".
    pub entry_point: &'a str,
}

/// Represents the source code of a program.
pub struct SourceCode<'a> {
    /// Source code of the vertex shader.
    pub vertex_shader: &'a str,

    /// Source code of the optional tessellation control shader.
    pub tessellation_control_shader: Option<StageSource<'a>>,

    /// Source code of the optional tessellation evaluation shader.
    pub tessellation_evaluation_shader: Option<StageSource<'a>>,

    /// Source code of the optional geometry shader.
    pub geometry_shader: Option<StageSource<'a>>,

    /// Source code of the fragment shader.
    pub fragment_shader: &'a str,
}

impl<'a> From<SourceCode<'a>> for ProgramCreationInput<'a> {
    #[inline]
    fn from(code: SourceCode<'a>) -> ProgramCreationInput<'a> {
        let SourceCode {
            vertex_shader,
            fragment_shader,
            geometry_shader,
            tessellation_control_shader,
            tessellation_evaluation_shader,
        } = code;

        ProgramCreationInput::SourceCode {
            vertex_shader,
            tessellation_control_shader,
            tessellation_evaluation_shader,
            geometry_shader,
            fragment_shader,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
            locations: LinkLocations::default(),
        }
    }
}

/// Represents the compiled binary data of a program.
#[cfg(any(safegl_gl41, safegl_gles30))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl41, safegl_gles30))))]
pub struct Binary {
    /// An implementation-defined format.
    pub format: u32,

    /// The binary data.
    pub content: Vec<u8>,
}

#[cfg(any(safegl_gl41, safegl_gles30))]
impl<'a> From<Binary> for ProgramCreationInput<'a> {
    #[inline]
    fn from(binary: Binary) -> ProgramCreationInput<'a> {
        ProgramCreationInput::Binary {
            data: binary,
            outputs_srgb: true,
            uses_point_size: false,
        }
    }
}
