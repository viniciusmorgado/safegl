use crate::gl;
#[cfg(safegl_gl46)]
use crate::handles::RawName;
use crate::version::Api;
use crate::version::Version;

use crate::backend::Facade;
use crate::context::Context;
use crate::CapabilitiesSource;
use crate::ContextExt;

use std::rc::Rc;
#[cfg(safegl_gl46)]
use std::ffi;

use glow::HasContext;

use crate::GlObject;

#[cfg(safegl_gl46)]
use crate::program::spirv::{ExecutionModel, Module};
#[cfg(safegl_gl46)]
use crate::program::SpirvEntryPoint;
use crate::program::{ProgramCreationError, ShaderType};

/// A single, compiled but unlinked, shader.
pub struct Shader {
    context: Rc<Context>,
    id: glow::Shader,
}

impl GlObject for Shader {
    type Id = glow::Shader;

    #[inline]
    fn get_id(&self) -> glow::Shader {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        let ctxt = self.context.make_current();

        unsafe {
            ctxt.glow.delete_shader(self.id);
        }
    }
}

/// Builds an individual shader.
pub fn build_shader<F: ?Sized>(
    facade: &F,
    shader_type: gl::types::GLenum,
    source_code: &str,
) -> Result<Shader, ProgramCreationError>
where
    F: Facade,
{
    unsafe {
        let ctxt = facade.get_context().make_current();

        if ctxt.capabilities.supported_glsl_versions.is_empty() {
            return Err(ProgramCreationError::CompilationNotSupported);
        }

        if !check_shader_type_compatibility(&ctxt, shader_type) {
            return Err(ProgramCreationError::ShaderTypeNotSupported);
        }

        let shader = match ctxt.glow.create_shader(shader_type) {
            Ok(shader) => shader,
            Err(_) => return Err(ProgramCreationError::ShaderTypeNotSupported),
        };

        ctxt.glow.shader_source(shader, source_code);

        // compiling
        {
            ctxt.report_debug_output_errors.set(false);
            ctxt.glow.compile_shader(shader);
            ctxt.report_debug_output_errors.set(true);
        }

        // checking compilation success by reading a flag on the shader
        if ctxt.glow.get_shader_compile_status(shader) {
            Ok(Shader {
                context: facade.get_context().clone(),
                id: shader,
            })
        } else {
            // compilation error
            let msg = ctxt.glow.get_shader_info_log(shader);

            Err(ProgramCreationError::CompilationError(
                msg,
                ShaderType::from_opengl_type(shader_type),
            ))
        }
    }
}

/// Builds an individual shader from a SPIR-V binary, after checking its entry point and
/// specialization constants. Returns it with the module, which reflection reads.
#[cfg(safegl_gl46)]
pub fn build_spirv_shader<F: ?Sized>(
    facade: &F,
    shader_type: gl::types::GLenum,
    spirv: &SpirvEntryPoint,
) -> Result<(Shader, Module), ProgramCreationError>
where
    F: Facade,
{
    let invalid = ProgramCreationError::InvalidSpirvModule;
    let mut module = Module::parse(spirv.binary).map_err(invalid)?;
    let model = ExecutionModel::from_shader_type(shader_type)
        .ok_or(ProgramCreationError::ShaderTypeNotSupported)?;
    module
        .check_entry_point(spirv.entry_point, model)
        .map_err(invalid)?;
    let values: Vec<_> = spirv
        .specialization
        .iter()
        .map(|&(id, value)| value.to_spirv(id))
        .collect();
    module.specialize(&values).map_err(invalid)?;
    // the entry point is a name of the module, which has no nul character
    let entry_point = ffi::CString::new(spirv.entry_point.as_bytes())
        .map_err(|_| invalid("the entry point has a nul character".to_owned()))?;
    let indices: Vec<gl::types::GLuint> = values.iter().map(|value| value.id).collect();
    let bits: Vec<gl::types::GLuint> = values.iter().map(|value| value.bits).collect();

    unsafe {
        let ctxt = facade.get_context().make_current();

        if ctxt.capabilities.supported_glsl_versions.is_empty() {
            return Err(ProgramCreationError::CompilationNotSupported);
        }

        if !check_shader_type_compatibility(&ctxt, shader_type) {
            return Err(ProgramCreationError::ShaderTypeNotSupported);
        }

        let shader = if ctxt.version >= &Version(Api::Gl, 4, 6)
            || (ctxt.version.0 == Api::Gl && ctxt.extensions.gl_arb_gl_spirv)
        {
            ctxt.glow.create_shader(shader_type)
        } else {
            return Err(ProgramCreationError::CompilationNotSupported);
        };

        let shader = match shader {
            Ok(shader) => shader,
            Err(_) => return Err(ProgramCreationError::ShaderTypeNotSupported),
        };
        let id = shader.raw_name();
        // deleted if it doesn't compile
        let built = Shader {
            context: facade.get_context().clone(),
            id: shader,
        };

        let binary = &spirv.binary;

        const GL_SHADER_BINARY_FORMAT_SPIR_V: gl::types::GLenum = 0x9551;

        ctxt.extra.ShaderBinary(
            1,
            &id,
            GL_SHADER_BINARY_FORMAT_SPIR_V,
            binary.as_ptr() as _,
            binary.len() as _,
        );

        // compiling
        {
            ctxt.report_debug_output_errors.set(false);

            // below OpenGL 4.6, the extension only has its own entry point
            if ctxt.version >= &Version(Api::Gl, 4, 6) {
                ctxt.extra.SpecializeShader(
                    id,
                    entry_point.as_ptr() as _,
                    values.len() as gl::types::GLuint,
                    indices.as_ptr(),
                    bits.as_ptr(),
                );
            } else {
                ctxt.extra.SpecializeShaderARB(
                    id,
                    entry_point.as_ptr() as _,
                    values.len() as gl::types::GLuint,
                    indices.as_ptr(),
                    bits.as_ptr(),
                );
            }

            ctxt.report_debug_output_errors.set(true);
        }

        // checking compilation success by reading a flag on the shader
        if ctxt.glow.get_shader_compile_status(shader) {
            Ok((built, module))
        } else {
            // compilation error
            let msg = ctxt.glow.get_shader_info_log(shader);

            Err(ProgramCreationError::CompilationError(
                msg,
                ShaderType::from_opengl_type(shader_type),
            ))
        }
    }
}

pub fn check_shader_type_compatibility<C: ?Sized>(ctxt: &C, shader_type: gl::types::GLenum) -> bool
where
    C: CapabilitiesSource,
{
    match shader_type {
        gl::VERTEX_SHADER | gl::FRAGMENT_SHADER => (),
        gl::GEOMETRY_SHADER => {
            if ctxt.get_version().0 != Api::Gl
                && !(ctxt.get_version() >= &Version(Api::GlEs, 3, 2))
                && !ctxt.get_extensions().gl_ext_geometry_shader
                && !ctxt.get_extensions().gl_oes_geometry_shader
            {
                return false;
            }
        }
        gl::TESS_CONTROL_SHADER | gl::TESS_EVALUATION_SHADER => {
            if ctxt.get_version().0 != Api::Gl
                && !(ctxt.get_version() >= &Version(Api::GlEs, 3, 2))
                && !ctxt.get_extensions().gl_ext_tessellation_shader
                && !ctxt.get_extensions().gl_oes_tessellation_shader
            {
                return false;
            }
        }
        gl::COMPUTE_SHADER => {
            if !(ctxt.get_version() >= &Version(Api::Gl, 4, 3))
                && !(ctxt.get_version() >= &Version(Api::GlEs, 3, 1))
                && !ctxt.get_extensions().gl_arb_compute_shader
            {
                return false;
            }
        }
        _ => unreachable!(),
    };

    true
}

/// A fragment output declared in GLSL source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentOutput {
    /// Name of the output.
    pub name: String,
    /// The location given with `layout(location = N)`, if any.
    pub location: Option<u32>,
    /// The number of locations of the output: the size of an array, otherwise 1.
    pub size: u32,
}

/// Returns the outputs declared in the source of a fragment shader, in their order.
///
/// Only declarations at global scope count, so the `out` parameters of functions are left out.
/// Arrays whose size isn't a number get a size of 1.
pub fn fragment_outputs(source: &str) -> Vec<FragmentOutput> {
    // the source without comments nor preprocessor lines
    let mut code = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut line_start = true;
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                while chars.peek().is_some_and(|&c| c != '\n') {
                    chars.next();
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = ' ';
                for c in chars.by_ref() {
                    if previous == '*' && c == '/' {
                        break;
                    }
                    previous = c;
                }
                code.push(' ');
            }
            '#' if line_start => {
                while chars.peek().is_some_and(|&c| c != '\n') {
                    chars.next();
                }
            }
            _ => {
                if c == '\n' {
                    line_start = true;
                } else if !c.is_whitespace() {
                    line_start = false;
                }
                code.push(c);
            }
        }
    }

    // the declarations at global scope: the text between `;` and `}` at a depth of 0
    let mut declarations = Vec::new();
    let mut depth = 0u32;
    let mut current = String::new();
    for c in code.chars() {
        match c {
            '{' => {
                depth += 1;
                current.clear();
            }
            '}' => {
                depth = depth.saturating_sub(1);
                current.clear();
            }
            ';' if depth == 0 => declarations.push(std::mem::take(&mut current)),
            _ if depth == 0 => current.push(c),
            _ => (),
        }
    }

    let mut outputs = Vec::new();
    for declaration in declarations {
        // the location of `layout(location = N)`, and the declaration without the layout
        let mut location = None;
        let mut rest = declaration.as_str();
        let mut words = String::new();
        while let Some(start) = rest.find("layout") {
            words.push_str(&rest[..start]);
            let after = &rest[start + "layout".len()..];
            let open = match after.find('(') {
                Some(open) if after[..open].trim().is_empty() => open,
                _ => {
                    words.push_str("layout");
                    rest = after;
                    continue;
                }
            };
            let close = match after.find(')') {
                Some(close) => close,
                None => break,
            };
            for qualifier in after[open + 1..close].split(',') {
                let mut parts = qualifier.splitn(2, '=');
                if parts.next().map(str::trim) == Some("location") {
                    location = parts.next().and_then(|n| n.trim().parse().ok());
                }
            }
            rest = &after[close + 1..];
        }
        words.push_str(rest);

        // `out`, then optional qualifiers, then the type, then the names
        let mut tokens = words.split_whitespace();
        if !tokens.any(|token| token == "out") {
            continue;
        }
        let remaining: Vec<&str> = tokens.collect();
        let type_index = remaining
            .iter()
            .position(|token| !matches!(*token, "highp" | "mediump" | "lowp" | "invariant"));
        let names = match type_index {
            Some(index) => remaining[index + 1..].join(" "),
            None => continue,
        };

        for declarator in names.split(',') {
            let declarator = declarator.trim();
            let (name, size) = match declarator.find('[') {
                Some(bracket) => {
                    let size = declarator[bracket + 1..]
                        .trim_end_matches(']')
                        .trim()
                        .parse()
                        .unwrap_or(1);
                    (declarator[..bracket].trim(), size)
                }
                None => (declarator, 1),
            };
            // the built-in outputs that the shader redeclares, like `gl_FragDepth`, have no
            // location
            if !name.is_empty() && !name.starts_with("gl_") {
                outputs.push(FragmentOutput {
                    name: name.to_owned(),
                    location,
                    size,
                });
                // the following declarators of an explicit layout take the next locations
                location = location.map(|location| location + size);
            }
        }
    }

    outputs
}

#[cfg(test)]
mod tests {
    use super::{fragment_outputs, FragmentOutput};

    fn output(name: &str, location: Option<u32>, size: u32) -> FragmentOutput {
        FragmentOutput {
            name: name.to_owned(),
            location,
            size,
        }
    }

    #[test]
    fn outputs_of_fragment_shaders() {
        let source = "
            #version 330
            // out vec4 commented;
            /* out vec4 also_commented; */
            in vec2 uv;
            uniform sampler2D tex;
            layout(location = 2) out vec4 normal;
            out vec4 color;
            out highp vec2 velocity[2], extra;
            layout(depth_greater) out float gl_FragDepth;

            void helper(out float value) { value = 1.0; }

            void main() {
                float x;
                helper(x);
                color = vec4(x);
            }
        ";
        assert_eq!(
            fragment_outputs(source),
            vec![
                output("normal", Some(2), 1),
                output("color", None, 1),
                output("velocity", None, 2),
                output("extra", None, 1),
            ]
        );
    }
}
