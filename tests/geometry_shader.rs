#![cfg(any(safegl_desktop, safegl_gles32))]

//! Geometry shaders that emit primitives (glium#544).

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::PrimitivesGeneratedQuery;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::{OutputPrimitives, ProgramCreationInput};
use safegl::{DrawParameters, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program whose geometry shader turns each point into `layout` with `body`, or `None` without
/// geometry shaders.
fn geometry_program(display: &Display, layout: &str, body: &str) -> Option<safegl::Program> {
    let header = if support::is_gles(display) {
        if *display.get_opengl_version() < safegl::Version(safegl::Api::GlEs, 3, 2) {
            return None;
        }
        "#version 320 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    };

    let vertex = format!(
        "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
    );
    let geometry = format!("{header}layout(points) in;\n{layout}\nvoid main() {{\n{body}\n}}");
    let fragment =
        format!("{header}out vec4 color;\nvoid main() {{ color = vec4(1.0, 0.0, 0.0, 1.0); }}");

    let program = safegl::Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: &vertex,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: Some(&geometry),
            fragment_shader: &fragment,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap();
    Some(program)
}

/// Emits a quad covering the target, as a strip of two triangles, for each point.
const QUAD: (&str, &str) = (
    "layout(triangle_strip, max_vertices = 4) out;",
    "gl_Position = vec4(-1.0, -1.0, 0.0, 1.0); EmitVertex();
     gl_Position = vec4(1.0, -1.0, 0.0, 1.0); EmitVertex();
     gl_Position = vec4(-1.0, 1.0, 0.0, 1.0); EmitVertex();
     gl_Position = vec4(1.0, 1.0, 0.0, 1.0); EmitVertex();
     EndPrimitive();",
);

fn points(display: &Display, count: usize) -> VertexBuffer<Vertex> {
    VertexBuffer::new(
        display,
        &vec![
            Vertex {
                position: [0.0, 0.0]
            };
            count
        ],
    )
    .unwrap()
}

#[test]
fn emit_triangles() {
    let display = support::build_display();
    let program = match geometry_program(&display, QUAD.0, QUAD.1) {
        Some(program) => program,
        None => return,
    };
    assert_eq!(
        program.get_output_primitives(),
        Some(OutputPrimitives::Triangles)
    );

    // a single point fills the target
    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            &points(&display, 1),
            NoIndices(PrimitiveType::Points),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert!(data.iter().flatten().all(|&p| p == (255, 0, 0, 255)));

    display.assert_no_error(None);
}

#[test]
fn emit_lines() {
    let display = support::build_display();
    // a horizontal line through the middle of the target
    let program = match geometry_program(
        &display,
        "layout(line_strip, max_vertices = 2) out;",
        "gl_Position = vec4(-1.0, 0.0, 0.0, 1.0); EmitVertex();
         gl_Position = vec4(1.0, 0.0, 0.0, 1.0); EmitVertex();
         EndPrimitive();",
    ) {
        Some(program) => program,
        None => return,
    };
    assert_eq!(
        program.get_output_primitives(),
        Some(OutputPrimitives::Lines)
    );

    let texture = safegl::Texture2d::empty(&display, 16, 16).unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            &points(&display, 1),
            NoIndices(PrimitiveType::Points),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    // only the rows next to the middle of the target are drawn
    for (y, row) in data.iter().enumerate() {
        let drawn = row.iter().filter(|&&p| p == (255, 0, 0, 255)).count();
        if y == 7 || y == 8 {
            continue;
        }
        assert_eq!(drawn, 0, "row {y}");
    }
    assert!(data[7..9].iter().flatten().any(|&p| p == (255, 0, 0, 255)));

    display.assert_no_error(None);
}

#[test]
fn emit_points() {
    let display = support::build_display();
    let program = match geometry_program(
        &display,
        "layout(points, max_vertices = 1) out;",
        "gl_Position = gl_in[0].gl_Position; EmitVertex(); EndPrimitive();",
    ) {
        Some(program) => program,
        None => return,
    };
    assert_eq!(
        program.get_output_primitives(),
        Some(OutputPrimitives::Points)
    );

    display.assert_no_error(None);
}

#[test]
fn primitives_generated_by_the_geometry_shader() {
    let display = support::build_display();
    let program = match geometry_program(&display, QUAD.0, QUAD.1) {
        Some(program) => program,
        None => return,
    };
    let query = PrimitivesGeneratedQuery::new(&display).unwrap();

    let texture = support::build_renderable_texture(&display);
    texture
        .as_surface()
        .draw(
            &points(&display, 3),
            NoIndices(PrimitiveType::Points),
            &program,
            &uniform! {},
            &DrawParameters {
                primitives_generated_queries: &[&query],
                ..Default::default()
            },
        )
        .unwrap();

    // two triangles for each of the three points
    assert_eq!(query.get().unwrap(), 6);

    display.assert_no_error(None);
}
