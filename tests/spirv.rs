//! Programs linked from SPIR-V modules, with and without their debug names: specialization,
//! reflection by location and binding, compute shaders and transform feedback.
#![cfg(safegl_gl46)]

#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::buffer::{Buffer, BufferMode, BufferType};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::{
    ComputeShader, ProgramCreationError, SpecializationConstant, SpirvEntryPoint, SpirvProgram,
};
use safegl::uniforms::{UniformBuffer, Uniforms};
use safegl::vertex::TransformFeedback;
use safegl::{CapabilitiesSource, Program, Surface, Texture2d, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;
type Pixel = (u8, u8, u8, u8);

/// A vertex whose position goes to the location 0, whatever its name.
#[derive(Copy, Clone, Debug, PartialEq)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position location(0));

#[derive(Copy, Clone, Debug, PartialEq)]
struct Captured {
    captured: [f32; 4],
}
implement_vertex!(Captured, captured);
implement_any_bit_pattern!(Captured, captured);

/// Returns a module of `tests/fixture/spirv`.
fn module(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixture/spirv/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// Returns true if the context links SPIR-V modules.
fn spirv_supported(display: &Display) -> bool {
    let context = display.get_context();
    *context.get_version() >= safegl::Version(safegl::Api::Gl, 4, 6)
        || context.get_extensions().gl_arb_gl_spirv
}

fn triangle(display: &Display) -> VertexBuffer<Vertex> {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

fn draw(display: &Display, program: &Program, uniforms: &impl Uniforms) -> Pixel {
    let target = Texture2d::empty(display, 2, 2).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    target
        .as_surface()
        .draw(
            &triangle(display),
            NoIndices(PrimitiveType::TrianglesList),
            program,
            uniforms,
            &Default::default(),
        )
        .unwrap();
    let data: Vec<Vec<Pixel>> = target.read().unwrap();
    data[0][0]
}

/// The textured program, from the modules with names or without.
fn textured(
    display: &Display,
    suffix: &str,
    specialization: &[(u32, SpecializationConstant)],
) -> Result<Program, ProgramCreationError> {
    let vertex = module(&format!("textured.vert.{suffix}"));
    let fragment = module(&format!("textured.frag.{suffix}"));
    Program::new(
        display,
        SpirvProgram::from_vs_and_fs(
            SpirvEntryPoint::new(&vertex, "main"),
            SpirvEntryPoint::new(&fragment, "main").specialized(specialization),
        ),
    )
}

#[test]
fn names_and_locations() {
    let display = support::build_display();
    if !spirv_supported(&display) {
        return;
    }
    let texture = Texture2d::new(&display, vec![vec![(255u8, 0u8, 0u8, 255u8); 2]; 2]).unwrap();
    let factors = UniformBuffer::new(&display, 1.0f32).unwrap();
    let tint = [0.4f32, 1.0, 1.0, 1.0];

    // the names of the modules, whether the driver keeps them or not
    let named = textured(&display, "spv", &[]).unwrap();
    let uniforms = uniform! { tint: tint, tex: &texture, Factors: &factors };
    assert_eq!(draw(&display, &named, &uniforms), (102, 0, 0, 255));
    assert_eq!(named.get_frag_data_location("color"), Some(0));
    assert_eq!(named.get_attribute("position").unwrap().location, 0);

    // the names from the locations and bindings, with names or without
    for suffix in ["spv", "stripped.spv"] {
        let program = textured(&display, suffix, &[]).unwrap();
        let uniforms = uniform! {
            "location=1": tint,
            "location=2": &texture,
            "binding=1": &factors,
        };
        assert_eq!(draw(&display, &program, &uniforms), (102, 0, 0, 255));
        assert_eq!(program.get_frag_data_location("location=0"), Some(0));
        assert_eq!(program.get_frag_data_index("location=0"), Some(0));
        assert_eq!(program.get_attribute("location=0").unwrap().location, 0);
    }

    display.assert_no_error(None);
}

#[test]
fn specialization() {
    let display = support::build_display();
    if !spirv_supported(&display) {
        return;
    }
    let texture = Texture2d::new(&display, vec![vec![(255u8, 0u8, 0u8, 255u8); 2]; 2]).unwrap();
    let factors = UniformBuffer::new(&display, 1.0f32).unwrap();
    let uniforms = uniform! {
        "location=1": [0.4f32, 1.0, 1.0, 1.0],
        "location=2": &texture,
        "binding=1": &factors,
    };

    // the constant 7 is the brightness
    let brighter = [(7, SpecializationConstant::Float(2.0))];
    let program = textured(&display, "stripped.spv", &brighter).unwrap();
    assert_eq!(draw(&display, &program, &uniforms), (204, 0, 0, 255));

    // constants that the module doesn't have, or of another type
    let invalid = |specialization: &[(u32, SpecializationConstant)]| {
        matches!(
            textured(&display, "spv", specialization),
            Err(ProgramCreationError::InvalidSpirvModule(_))
        )
    };
    assert!(invalid(&[(8, SpecializationConstant::Float(2.0))]));
    assert!(invalid(&[(7, SpecializationConstant::Int(2))]));

    display.assert_no_error(None);
}

#[test]
fn invalid_modules() {
    let display = support::build_display();
    if !spirv_supported(&display) {
        return;
    }
    let vertex = module("textured.vert.spv");
    let fragment = module("textured.frag.spv");
    let invalid = |vertex: SpirvEntryPoint<'_>, fragment: SpirvEntryPoint<'_>| {
        matches!(
            Program::new(&display, SpirvProgram::from_vs_and_fs(vertex, fragment)),
            Err(ProgramCreationError::InvalidSpirvModule(_))
        )
    };

    // not a module, and a truncated one
    assert!(invalid(
        SpirvEntryPoint::new(&[0; 64], "main"),
        SpirvEntryPoint::new(&fragment, "main")
    ));
    assert!(invalid(
        SpirvEntryPoint::new(&vertex[..vertex.len() - 4], "main"),
        SpirvEntryPoint::new(&fragment, "main")
    ));
    // no such entry point, and an entry point of another stage
    assert!(invalid(
        SpirvEntryPoint::new(&vertex, "other"),
        SpirvEntryPoint::new(&fragment, "main")
    ));
    assert!(invalid(
        SpirvEntryPoint::new(&fragment, "main"),
        SpirvEntryPoint::new(&fragment, "main")
    ));

    display.assert_no_error(None);
}

#[test]
fn compute_shaders() {
    let display = support::build_display();
    if !spirv_supported(&display) {
        return;
    }
    let values = Buffer::new(
        &display,
        &[0u32; 16][..],
        BufferType::ShaderStorageBuffer,
        BufferMode::Default,
    )
    .unwrap();

    // `local_size_x_id = 3` gives the width of the work groups
    for (suffix, name) in [("spv", "Values"), ("stripped.spv", "binding=0")] {
        let binary = module(&format!("work_group.comp.{suffix}"));
        let width = [(3, SpecializationConstant::UnsignedInt(8))];
        let shader = ComputeShader::from_spirv(
            &display,
            &SpirvEntryPoint::new(&binary, "main").specialized(&width),
        )
        .unwrap();
        assert_eq!(shader.work_group_size(), [8, 1, 1]);

        let values_ref = &values;
        let mut uniforms = safegl::uniforms::DynamicUniforms::new();
        uniforms.add(name, &values_ref);
        shader.execute(uniforms, 2, 1, 1).unwrap();
        assert_eq!(values.read().unwrap(), vec![8u32; 16]);
        values.write(&[0u32; 16][..]);
    }

    display.assert_no_error(None);
}

#[test]
fn transform_feedback() {
    let display = support::build_display();
    if !spirv_supported(&display) {
        return;
    }
    for suffix in ["spv", "stripped.spv"] {
        let vertex = module(&format!("capture.vert.{suffix}"));
        let fragment = module(&format!("capture.frag.{suffix}"));
        let program = Program::new(
            &display,
            SpirvProgram::from_vs_and_fs(
                SpirvEntryPoint::new(&vertex, "main"),
                SpirvEntryPoint::new(&fragment, "main"),
            ),
        )
        .unwrap();

        // the varying of `xfb_buffer = 0` and `xfb_offset = 0`
        let mut captured: Buffer<[Captured]> = Buffer::empty_array(
            &display,
            BufferType::TransformFeedbackBuffer,
            3,
            BufferMode::Default,
        )
        .unwrap();
        {
            let transform_feedback = TransformFeedback::new(&display, &program)
                .unwrap()
                .with_buffer(&mut captured)
                .unwrap();
            let target = support::build_renderable_texture(&display);
            target
                .as_surface()
                .draw(
                    &triangle(&display),
                    NoIndices(PrimitiveType::TrianglesList),
                    &program,
                    &uniform! {},
                    &safegl::DrawParameters {
                        transform_feedback: Some(&transform_feedback),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        let captured: Vec<[f32; 4]> = captured
            .read()
            .unwrap()
            .iter()
            .map(|c| c.captured)
            .collect();
        assert_eq!(
            captured,
            [
                [-1.0, -1.0, 1.0, 2.0],
                [3.0, -1.0, 1.0, 2.0],
                [-1.0, 3.0, 1.0, 2.0]
            ]
        );
    }

    display.assert_no_error(None);
}

#[test]
fn spirv_extensions() {
    let display = support::build_display();
    let context = display.get_context();
    let extensions = &context.get_capabilities().spirv_extensions;
    let supported = *context.get_version() >= safegl::Version(safegl::Api::Gl, 4, 6)
        || context.get_extensions().gl_arb_spirv_extensions;
    if !supported {
        assert!(extensions.is_empty());
    }
    // the names of SPIR-V extensions start with `SPV_`
    assert!(extensions.iter().all(|name| name.starts_with("SPV_")));
}
