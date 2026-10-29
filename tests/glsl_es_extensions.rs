//! The extensions of OpenGL ES and WebGL that only exist in GLSL ES: conservative depth, sample
//! variables and multisample interpolation. Desktop OpenGL has them in GLSL 4.x, which the
//! `glsl_coverage` tests cover.

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::DepthFormat;
use safegl::{Depth, DepthTest, DrawParameters, Program, Surface, Texture2d, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Pixel = (u8, u8, u8, u8);

const GREEN: Pixel = (0, 255, 0, 255);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program of GLSL ES 3.00 whose vertex shader draws a fullscreen triangle whose `v_one` is 1,
/// and whose fragment shader `fragment` has the `#extension` directive of `extension`; `None` on
/// desktop OpenGL and without the extension.
fn program(
    display: &support::Backend,
    extension: &str,
    has: bool,
    fragment: &str,
) -> Option<Program> {
    versioned_program(display, 300, extension, has, None, fragment)
}

/// Like `program`, in GLSL ES `version`, and with the vertex shader `vertex` that writes `v_one`
/// and has the `#extension` directive too, if it is given; `None` below that version.
fn versioned_program(
    display: &support::Backend,
    version: u32,
    extension: &str,
    has: bool,
    vertex: Option<&str>,
    fragment: &str,
) -> Option<Program> {
    if !support::is_gles(display) || !has || !support::glsl_supported(display, 0, version) {
        return None;
    }
    let header = |extension: &str| {
        format!("#version {version} es\n{extension}precision highp float;\nprecision highp int;\n")
    };
    let directive = format!("#extension {extension} : require\n");
    // the default vertex shader gives `v_one`; a custom one declares its own outputs
    let (vertex, inputs) = match vertex {
        Some(vertex) => (format!("{}in vec2 position;\n{vertex}", header(&directive)), ""),
        None => (
            format!(
                "{}in vec2 position;\nout float v_one;\n\
                 void main() {{ v_one = 1.0; gl_Position = vec4(position, 0.0, 1.0); }}",
                header("")
            ),
            "in float v_one;\n",
        ),
    };
    let fragment = format!("{}{inputs}out vec4 color;\n{fragment}", header(&directive));
    Some(Program::from_source(display, &vertex, &fragment, None).unwrap())
}

/// Draws the fullscreen triangle with `program` and returns true if every pixel is green.
fn draws_green(display: &support::Backend, program: &Program) -> bool {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    draw(display, &mut target.as_surface(), program, &Default::default());
    all_green(&target)
}

/// Draws the fullscreen triangle with `program` on `surface`.
fn draw<S: Surface>(
    display: &support::Backend,
    surface: &mut S,
    program: &Program,
    parameters: &DrawParameters<'_>,
) {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    surface
        .draw(
            &triangle,
            NoIndices(PrimitiveType::TrianglesList),
            program,
            &uniform! {},
            parameters,
        )
        .unwrap();
}

/// Returns true if every pixel of `texture` is green.
fn all_green(texture: &Texture2d) -> bool {
    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    pixels.iter().flatten().all(|&pixel| pixel == GREEN)
}

/// `GL_EXT_conservative_depth` (`EXT_conservative_depth` on WebGL): `gl_FragDepth` declared
/// `depth_greater`, written above the depth of the fragment and below the cleared depth.
#[test]
fn conservative_depth() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_conservative_depth;
    let Some(program) = program(
        &display,
        "GL_EXT_conservative_depth",
        has,
        "layout(depth_greater) out highp float gl_FragDepth;\n\
         void main() {\n\
             gl_FragDepth = gl_FragCoord.z + 0.25;\n\
             color = vec4(0.0, v_one, 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    let color = Texture2d::empty(&display, 4, 4).unwrap();
    let depth = DepthRenderBuffer::new(&display, DepthFormat::I24, 4, 4).unwrap();
    let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
    let parameters = DrawParameters {
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };
    // drawn at 0.5 + 0.25, in front of the cleared depth
    draw(&display, &mut framebuffer, &program, &parameters);
    assert!(all_green(&color));

    display.assert_no_error(None);
}

/// `GL_OES_sample_variables`: a single-sample target has one sample, of index 0.
#[test]
fn sample_variables() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_oes_sample_variables;
    let Some(program) = program(
        &display,
        "GL_OES_sample_variables",
        has,
        "void main() {\n\
             bool single = gl_SampleID == 0 && gl_NumSamples == 1;\n\
             color = single ? vec4(0.0, v_one, 0.0, 1.0) : vec4(1.0, 0.0, 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    draw(&display, &mut target.as_surface(), &program, &Default::default());
    assert!(all_green(&target));

    display.assert_no_error(None);
}

/// `GL_OES_shader_multisample_interpolation`: an input interpolated at an offset of a constant
/// value keeps it.
#[test]
fn multisample_interpolation() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_oes_shader_multisample_interpolation;
    let Some(program) = program(
        &display,
        "GL_OES_shader_multisample_interpolation",
        has,
        "void main() {\n\
             float one = interpolateAtOffset(v_one, vec2(0.25, -0.25));\n\
             color = vec4(0.0, one, 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    draw(&display, &mut target.as_surface(), &program, &Default::default());
    assert!(all_green(&target));

    display.assert_no_error(None);
}

/// `GL_EXT_gpu_shader5` and `GL_OES_gpu_shader5`: `precise` and `fma`.
#[test]
fn gpu_shader5() {
    let display = support::build_display();
    let extensions = support::extensions(&display);
    for (name, has) in [
        ("GL_EXT_gpu_shader5", extensions.gl_ext_gpu_shader5),
        ("GL_OES_gpu_shader5", extensions.gl_oes_gpu_shader5),
    ] {
        let fragment = "void main() {\n\
                            precise float one = fma(v_one, 1.0, 0.0);\n\
                            color = vec4(0.0, one, 0.0, 1.0);\n\
                        }";
        if let Some(program) = versioned_program(&display, 310, name, has, None, fragment) {
            assert!(draws_green(&display, &program), "{name}");
        }
    }
    display.assert_no_error(None);
}

/// `GL_EXT_shader_io_blocks` and `GL_OES_shader_io_blocks`: blocks of outputs and inputs.
#[test]
fn shader_io_blocks() {
    let display = support::build_display();
    let extensions = support::extensions(&display);
    for (name, has) in [
        ("GL_EXT_shader_io_blocks", extensions.gl_ext_shader_io_blocks),
        ("GL_OES_shader_io_blocks", extensions.gl_oes_shader_io_blocks),
    ] {
        let vertex = "out Data { float one; } data;\n\
                      void main() { data.one = 1.0; gl_Position = vec4(position, 0.0, 1.0); }";
        let fragment = "in Data { float one; } data;\n\
                        void main() { color = vec4(0.0, data.one, 0.0, 1.0); }";
        if let Some(program) = versioned_program(&display, 310, name, has, Some(vertex), fragment)
        {
            assert!(draws_green(&display, &program), "{name}");
        }
    }
    display.assert_no_error(None);
}

/// `GL_EXT_shader_non_constant_global_initializers`: a global initialized from another one.
#[test]
fn non_constant_global_initializers() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_shader_non_constant_global_initializers;
    let Some(program) = program(
        &display,
        "GL_EXT_shader_non_constant_global_initializers",
        has,
        "float zero = 0.0;\nfloat one = zero + 1.0;\n\
         void main() { color = vec4(0.0, one * v_one, 0.0, 1.0); }",
    ) else {
        return;
    };
    assert!(draws_green(&display, &program));
    display.assert_no_error(None);
}

/// `GL_NV_shader_noperspective_interpolation`: inputs interpolated without perspective.
#[test]
fn noperspective_interpolation() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_nv_shader_noperspective_interpolation;
    let vertex = "noperspective out float one;\n\
                  void main() { one = 1.0; gl_Position = vec4(position, 0.0, 1.0); }";
    let fragment = "noperspective in float one;\nvoid main() { color = vec4(0.0, one, 0.0, 1.0); }";
    let Some(program) = versioned_program(
        &display,
        300,
        "GL_NV_shader_noperspective_interpolation",
        has,
        Some(vertex),
        fragment,
    ) else {
        return;
    };
    assert!(draws_green(&display, &program));
    display.assert_no_error(None);
}

/// `GL_EXT_shader_group_vote`: a vote of the invocations.
#[test]
fn shader_group_vote() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_shader_group_vote;
    let Some(program) = program(
        &display,
        "GL_EXT_shader_group_vote",
        has,
        "void main() {\n\
             bool all_one = allInvocationsEXT(v_one > 0.5);\n\
             color = all_one ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0, 0.0, 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    assert!(draws_green(&display, &program));
    display.assert_no_error(None);
}

/// `GL_EXT_shader_integer_mix`: `mix` of integers chosen by a boolean.
#[test]
fn shader_integer_mix() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_shader_integer_mix;
    let Some(program) = program(
        &display,
        "GL_EXT_shader_integer_mix",
        has,
        "void main() {\n\
             int one = mix(0, 1, v_one > 0.5);\n\
             color = vec4(0.0, float(one), 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    assert!(draws_green(&display, &program));
    display.assert_no_error(None);
}

/// `GL_EXT_texture_shadow_lod`: `textureLod` on shadow sampler arrays, which the program
/// compiles and links with.
#[test]
fn texture_shadow_lod() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_texture_shadow_lod;
    let program = program(
        &display,
        "GL_EXT_texture_shadow_lod",
        has,
        "uniform highp sampler2DArrayShadow shadow;\n\
         void main() {\n\
             float lit = textureLod(shadow, vec4(0.5, 0.5, 0.0, 0.5), 0.0);\n\
             color = vec4(0.0, lit * v_one, 0.0, 1.0);\n\
         }",
    );
    assert_eq!(program.is_some(), has && support::is_gles(&display));
    display.assert_no_error(None);
}

/// `GL_OES_shader_image_atomic`: atomic operations on images, in a compute shader that compiles
/// and links.
#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn shader_image_atomic() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_oes_shader_image_atomic;
    if !support::is_gles(&display) || !has || !support::glsl_supported(&display, 0, 310) {
        return;
    }
    let source = "#version 310 es\n#extension GL_OES_shader_image_atomic : require\n\
                  layout(local_size_x = 1) in;\n\
                  layout(r32ui, binding = 0) uniform highp uimage2D counter;\n\
                  void main() { imageAtomicAdd(counter, ivec2(0), 1u); }";
    safegl::program::ComputeShader::from_source(&display, source).unwrap();
    display.assert_no_error(None);
}

/// `GL_EXT_geometry_point_size`: geometry shaders write `gl_PointSize`, in a program that
/// compiles and links.
#[cfg(any(safegl_desktop, safegl_gles32))]
#[test]
fn geometry_point_size() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_geometry_point_size;
    if !support::is_gles(&display) || !has || !support::glsl_supported(&display, 0, 320) {
        return;
    }
    let vertex = "#version 320 es\nin vec2 position;\n\
                  void main() { gl_Position = vec4(position, 0.0, 1.0); }";
    let geometry = "#version 320 es\n#extension GL_EXT_geometry_point_size : require\n\
                    layout(points) in;\nlayout(points, max_vertices = 1) out;\n\
                    void main() {\n\
                        gl_Position = gl_in[0].gl_Position;\n\
                        gl_PointSize = 4.0;\n\
                        EmitVertex();\n\
                    }";
    let fragment = "#version 320 es\nprecision mediump float;\nout vec4 color;\n\
                    void main() { color = vec4(0.0, 1.0, 0.0, 1.0); }";
    Program::from_source(&display, vertex, fragment, Some(geometry)).unwrap();
    display.assert_no_error(None);
}

/// `GL_EXT_tessellation_point_size`: tessellation shaders write `gl_PointSize`, in a program
/// that compiles and links.
#[cfg(any(safegl_desktop, safegl_gles32))]
#[test]
fn tessellation_point_size() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_tessellation_point_size;
    if !support::is_gles(&display) || !has || !support::glsl_supported(&display, 0, 320) {
        return;
    }
    let header = "#version 320 es\n#extension GL_EXT_tessellation_point_size : require\n";
    let vertex = "#version 320 es\nin vec2 position;\n\
                  void main() { gl_Position = vec4(position, 0.0, 1.0); }";
    let control = format!(
        "{header}layout(vertices = 1) out;\n\
         void main() {{\n\
             gl_out[gl_InvocationID].gl_Position = gl_in[gl_InvocationID].gl_Position;\n\
             gl_out[gl_InvocationID].gl_PointSize = 4.0;\n\
             gl_TessLevelOuter[0] = 1.0;\n\
             gl_TessLevelOuter[1] = 1.0;\n\
         }}"
    );
    let evaluation = format!(
        "{header}layout(isolines, point_mode) in;\n\
         void main() {{\n\
             gl_Position = gl_in[0].gl_Position;\n\
             gl_PointSize = gl_in[0].gl_PointSize;\n\
         }}"
    );
    let fragment = "#version 320 es\nprecision mediump float;\nout vec4 color;\n\
                    void main() { color = vec4(0.0, 1.0, 0.0, 1.0); }";
    Program::new(
        &display,
        safegl::program::ProgramCreationInput::SourceCode {
            vertex_shader: vertex,
            tessellation_control_shader: Some(control.as_str()),
            tessellation_evaluation_shader: Some(evaluation.as_str()),
            geometry_shader: None,
            fragment_shader: fragment,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap();
    display.assert_no_error(None);
}
