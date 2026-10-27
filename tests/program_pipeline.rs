//! Separable programs combined into program pipelines: uniforms of several stages, transform
//! feedback, subroutines, and the errors of pipelines.
#![cfg(any(safegl_gl41, safegl_gles31))]

#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::buffer::{Buffer, BufferMode, BufferType};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::{
    ProgramPipeline, ProgramPipelineCreationError, SeparableProgram, SeparableSource, ShaderType,
    TransformFeedbackMode,
};
use safegl::uniforms::Uniforms;
use safegl::vertex::TransformFeedback;
use safegl::{CapabilitiesSource, DrawError, Program, Surface, Texture2d, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;
type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Captured {
    doubled: [f32; 2],
}
implement_vertex!(Captured, doubled);
implement_any_bit_pattern!(Captured, doubled);

/// The header of the shaders, or `None` without program pipelines.
fn header(display: &Display) -> Option<&'static str> {
    let context = display.get_context();
    let version = *context.get_version();
    if support::is_gles(display) {
        (version >= safegl::Version(safegl::Api::GlEs, 3, 1))
            .then_some("#version 310 es\nprecision highp float;\nprecision highp int;\n")
    } else if support::glsl_supported(display, 410, 0) {
        Some("#version 410 core\n")
    } else if context.get_extensions().gl_arb_separate_shader_objects {
        Some("#version 400 core\n#extension GL_ARB_separate_shader_objects : require\n")
    } else {
        None
    }
}

/// The declaration of the outputs of the stages before rasterization: desktop OpenGL wants
/// separable programs to redeclare `gl_PerVertex`.
fn per_vertex(display: &Display) -> &'static str {
    if support::is_gles(display) {
        ""
    } else {
        "out gl_PerVertex { vec4 gl_Position; };\n"
    }
}

/// A vertex program that writes the green channel of `shift` into `v_green`.
fn vertex_program(display: &Display, header: &str) -> SeparableProgram {
    let source = format!(
        "{header}{}layout(location = 0) in vec2 position;\n\
         layout(location = 0) out float v_green;\n\
         uniform sampler2D shift;\n\
         void main() {{\n\
             v_green = texelFetch(shift, ivec2(0), 0).g;\n\
             gl_Position = vec4(position, 0.0, 1.0);\n\
         }}",
        per_vertex(display)
    );
    SeparableProgram::from_source(display, ShaderType::Vertex, &source).unwrap()
}

/// A fragment program that writes the red channel of `tex` and `v_green`, times `tint`.
fn fragment_program(display: &Display, header: &str) -> SeparableProgram {
    let source = format!(
        "{header}layout(location = 0) in float v_green;\n\
         layout(location = 0) out vec4 color;\n\
         uniform sampler2D tex;\n\
         uniform vec4 tint;\n\
         void main() {{\n\
             color = vec4(texture(tex, vec2(0.5)).r, v_green, 0.0, 1.0) * tint;\n\
         }}"
    );
    SeparableProgram::from_source(display, ShaderType::Fragment, &source).unwrap()
}

fn triangle(display: &Display) -> VertexBuffer<Vertex> {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

fn draw<'a>(
    display: &Display,
    program: impl Into<safegl::program::DrawProgram<'a>>,
    uniforms: &impl Uniforms,
) -> Result<Pixel, DrawError> {
    let target = Texture2d::empty(display, 2, 2).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    target.as_surface().draw(
        &triangle(display),
        NoIndices(PrimitiveType::TrianglesList),
        program,
        uniforms,
        &Default::default(),
    )?;
    let data: Vec<Vec<Pixel>> = target.read().unwrap();
    Ok(data[0][0])
}

fn unicolor(display: &Display, color: (u8, u8, u8, u8)) -> Texture2d {
    Texture2d::new(display, vec![vec![color; 2]; 2]).unwrap()
}

#[test]
fn uniforms_of_every_stage() {
    let display = support::build_display();
    let Some(header) = header(&display) else {
        return;
    };
    let vertex = vertex_program(&display, header);
    let fragment = fragment_program(&display, header);
    let pipeline = ProgramPipeline::new(&display, &[&vertex, &fragment]).unwrap();
    let red = unicolor(&display, (255, 0, 0, 255));
    let green = unicolor(&display, (0, 255, 0, 255));

    // the samplers of the two programs get their own texture units
    let white = [1.0f32; 4];
    let uniforms = uniform! { shift: &green, tex: &red, tint: white };
    assert_eq!(
        draw(&display, &pipeline, &uniforms).unwrap(),
        (255, 255, 0, 255)
    );
    let uniforms = uniform! { shift: &red, tex: &green, tint: white };
    assert_eq!(
        draw(&display, &pipeline, &uniforms).unwrap(),
        (0, 0, 0, 255)
    );

    // every uniform of every program needs a value
    let uniforms = uniform! { shift: &green, tint: white };
    assert!(matches!(
        draw(&display, &pipeline, &uniforms),
        Err(DrawError::UniformBindingMissing { name }) if name == "tex"
    ));

    // a program drawn between two draws of the pipeline
    let program = Program::from_source(
        &display,
        &format!(
            "{header}layout(location = 0) in vec2 position;\n\
             void main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        &format!(
            "{header}layout(location = 0) out vec4 color;\n\
             void main() {{ color = vec4(0.0, 0.0, 1.0, 1.0); }}"
        ),
        None,
    )
    .unwrap();
    assert_eq!(
        draw(&display, &program, &uniform! {}).unwrap(),
        (0, 0, 255, 255)
    );
    let half = [0.6f32, 0.6, 1.0, 1.0];
    let uniforms = uniform! { shift: &green, tex: &red, tint: half };
    assert_eq!(
        draw(&display, &pipeline, &uniforms).unwrap(),
        (153, 153, 0, 255)
    );

    display.assert_no_error(None);
}

#[test]
fn pipeline_errors() {
    let display = support::build_display();
    let Some(header) = header(&display) else {
        return;
    };
    let vertex = vertex_program(&display, header);
    let other_vertex = vertex_program(&display, header);
    let fragment = fragment_program(&display, header);

    assert_eq!(
        ProgramPipeline::new(&display, &[&fragment]).unwrap_err(),
        ProgramPipelineCreationError::NoVertexShader
    );
    assert_eq!(
        ProgramPipeline::new(&display, &[&vertex, &other_vertex, &fragment]).unwrap_err(),
        ProgramPipelineCreationError::StageGivenTwice
    );
    assert!(vertex.has_stage(ShaderType::Vertex));
    assert!(!vertex.has_stage(ShaderType::Fragment));

    // a separable program without a fragment shader can't be drawn on its own
    let green = unicolor(&display, (0, 255, 0, 255));
    assert!(matches!(
        draw(&display, &*vertex, &uniform! { shift: &green }),
        Err(DrawError::IncompleteProgram)
    ));

    // compute shaders aren't separable programs, and a separable program needs a shader
    assert!(SeparableProgram::from_source(&display, ShaderType::Compute, "").is_err());
    assert!(SeparableProgram::new(&display, SeparableSource::default()).is_err());

    display.assert_no_error(None);
}

#[test]
fn several_stages_in_a_program() {
    let display = support::build_display();
    let Some(header) = header(&display) else {
        return;
    };
    // a program with both stages is a pipeline on its own, and can be drawn on its own
    let vertex = format!(
        "{header}{}layout(location = 0) in vec2 position;\n\
         void main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
        per_vertex(&display)
    );
    let fragment = format!(
        "{header}layout(location = 0) out vec4 color;\nuniform vec4 tint;\n\
         void main() {{ color = tint; }}"
    );
    let program = SeparableProgram::new(
        &display,
        SeparableSource {
            vertex_shader: Some(&vertex),
            fragment_shader: Some(&fragment),
            ..Default::default()
        },
    )
    .unwrap();
    let tint = [0.0f32, 1.0, 0.0, 1.0];
    assert_eq!(
        draw(&display, &*program, &uniform! { tint: tint }).unwrap(),
        (0, 255, 0, 255)
    );
    let pipeline = ProgramPipeline::new(&display, &[&program]).unwrap();
    assert_eq!(
        draw(&display, &pipeline, &uniform! { tint: tint }).unwrap(),
        (0, 255, 0, 255)
    );

    display.assert_no_error(None);
}

#[test]
fn transform_feedback_of_the_last_stage() {
    let display = support::build_display();
    let Some(header) = header(&display) else {
        return;
    };
    let vertex = SeparableProgram::new(
        &display,
        SeparableSource {
            vertex_shader: Some(&format!(
                "{header}{}layout(location = 0) in vec2 position;\n\
                 layout(location = 0) out vec2 doubled;\n\
                 void main() {{\n\
                     doubled = position * 2.0;\n\
                     gl_Position = vec4(position, 0.0, 1.0);\n\
                 }}",
                per_vertex(&display)
            )),
            transform_feedback_varyings: Some((
                vec!["doubled".to_owned()],
                TransformFeedbackMode::Interleaved,
            )),
            ..Default::default()
        },
    )
    .unwrap();
    // OpenGL ES wants the inputs of a stage to match the outputs of the previous one
    let fragment_with = |declaration: &str| {
        SeparableProgram::from_source(
            &display,
            ShaderType::Fragment,
            &format!(
                "{header}{declaration}layout(location = 0) out vec4 color;\n\
                 void main() {{ color = vec4(1.0); }}"
            ),
        )
        .unwrap()
    };
    if support::is_gles(&display) {
        let mismatched = fragment_with("layout(location = 0) in vec4 doubled;\n");
        assert!(matches!(
            ProgramPipeline::new(&display, &[&vertex, &mismatched]),
            Err(ProgramPipelineCreationError::ValidationFailed(_))
        ));
    }
    let fragment = fragment_with("layout(location = 0) in vec2 doubled;\n");
    let pipeline = ProgramPipeline::new(&display, &[&vertex, &fragment]).unwrap();

    let mut captured: Buffer<[Captured]> = Buffer::empty_array(
        &display,
        BufferType::TransformFeedbackBuffer,
        3,
        BufferMode::Default,
    )
    .unwrap();
    {
        let transform_feedback = TransformFeedback::new(&display, &pipeline)
            .unwrap()
            .with_buffer(&mut captured)
            .unwrap();
        let target = support::build_renderable_texture(&display);
        target
            .as_surface()
            .draw(
                &triangle(&display),
                NoIndices(PrimitiveType::TrianglesList),
                &pipeline,
                &uniform! {},
                &safegl::DrawParameters {
                    transform_feedback: Some(&transform_feedback),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let captured: Vec<[f32; 2]> = captured.read().unwrap().iter().map(|c| c.doubled).collect();
    assert_eq!(captured, [[-2.0, -2.0], [6.0, -2.0], [-2.0, 6.0]]);

    display.assert_no_error(None);
}

#[cfg(safegl_gl41)]
#[test]
fn subroutines_of_every_stage() {
    use safegl::program::ShaderStage;
    use safegl::uniforms::UniformValue;

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let Some(header) = header(&display) else {
        return;
    };
    let vertex = SeparableProgram::from_source(
        &display,
        ShaderType::Vertex,
        &format!(
            "{header}{}layout(location = 0) in vec2 position;\n\
             layout(location = 0) out float v_green;\n\
             subroutine float Green();\n\
             subroutine(Green) float none() {{ return 0.0; }}\n\
             subroutine(Green) float full() {{ return 1.0; }}\n\
             subroutine uniform Green green;\n\
             void main() {{ v_green = green(); gl_Position = vec4(position, 0.0, 1.0); }}",
            per_vertex(&display)
        ),
    )
    .unwrap();
    let fragment = SeparableProgram::from_source(
        &display,
        ShaderType::Fragment,
        &format!(
            "{header}layout(location = 0) in float v_green;\n\
             layout(location = 0) out vec4 color;\n\
             subroutine float Red();\n\
             subroutine(Red) float none() {{ return 0.0; }}\n\
             subroutine(Red) float full() {{ return 1.0; }}\n\
             subroutine uniform Red red;\n\
             void main() {{ color = vec4(red(), v_green, 0.0, 1.0); }}"
        ),
    )
    .unwrap();
    let pipeline = ProgramPipeline::new(&display, &[&vertex, &fragment]).unwrap();

    // the subroutines of each stage, set again on every draw
    for (green, red, expected) in [
        ("full", "none", (0, 255, 0, 255)),
        ("none", "full", (255, 0, 0, 255)),
        ("full", "full", (255, 255, 0, 255)),
    ] {
        let uniforms = uniform! {
            green: UniformValue::Subroutine(ShaderStage::Vertex, green),
            red: UniformValue::Subroutine(ShaderStage::Fragment, red),
        };
        assert_eq!(draw(&display, &pipeline, &uniforms).unwrap(), expected);
    }

    display.assert_no_error(None);
}
