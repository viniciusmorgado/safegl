//! Tessellation shaders (glium#319).

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::{OutputPrimitives, ProgramCreationInput};
use safegl::{DrawError, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program that tessellates triangle patches and colors them green, or `None` without
/// tessellation shaders.
fn tessellation_program(display: &Display) -> Option<safegl::Program> {
    let header = if support::is_gles(display) {
        if *display.get_opengl_version() < safegl::Version(safegl::Api::GlEs, 3, 2) {
            return None;
        }
        "#version 320 es\nprecision highp float;\n"
    } else {
        if *display.get_opengl_version() < safegl::Version(safegl::Api::Gl, 4, 0) {
            return None;
        }
        "#version 400\n"
    };

    let vertex = format!(
        "{}in vec2 position;\nout vec2 v_position;\nvoid main() {{ v_position = position; }}",
        header
    );
    let control = format!(
        "{}layout(vertices = 3) out;\nin vec2 v_position[];\nout vec2 c_position[];\n\
         void main() {{\n\
             c_position[gl_InvocationID] = v_position[gl_InvocationID];\n\
             gl_TessLevelOuter[0] = 4.0; gl_TessLevelOuter[1] = 4.0; gl_TessLevelOuter[2] = 4.0;\n\
             gl_TessLevelInner[0] = 4.0;\n\
         }}",
        header
    );
    let evaluation = format!(
        "{}layout(triangles, equal_spacing, ccw) in;\nin vec2 c_position[];\n\
         void main() {{\n\
             vec2 position = gl_TessCoord.x * c_position[0] + gl_TessCoord.y * c_position[1]\n\
                 + gl_TessCoord.z * c_position[2];\n\
             gl_Position = vec4(position, 0.0, 1.0);\n\
         }}",
        header
    );
    let fragment = format!(
        "{}out vec4 color;\nvoid main() {{ color = vec4(0.0, 1.0, 0.0, 1.0); }}",
        header
    );

    let program = safegl::Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: &vertex,
            tessellation_control_shader: Some(&control),
            tessellation_evaluation_shader: Some(&evaluation),
            geometry_shader: None,
            fragment_shader: &fragment,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
        },
    )
    .unwrap();
    Some(program)
}

fn triangle(display: &Display) -> VertexBuffer<Vertex> {
    VertexBuffer::new(
        display,
        &[
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [3.0, -1.0] },
            Vertex { position: [-1.0, 3.0] },
        ],
    )
    .unwrap()
}

#[test]
fn draw_patches() {
    let display = support::build_display();
    let program = match tessellation_program(&display) {
        Some(program) => program,
        None => return,
    };
    assert_eq!(program.get_output_primitives(), Some(OutputPrimitives::Triangles));

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            &triangle(&display),
            NoIndices(PrimitiveType::Patches {
                vertices_per_patch: 3,
            }),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    // the tessellated triangle covers the target
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert!(data.iter().flatten().all(|&p| p == (0, 255, 0, 255)));

    display.assert_no_error(None);
}

#[test]
fn tessellation_without_patches() {
    let display = support::build_display();
    let program = match tessellation_program(&display) {
        Some(program) => program,
        None => return,
    };

    let texture = support::build_renderable_texture(&display);
    let result = texture.as_surface().draw(
        &triangle(&display),
        NoIndices(PrimitiveType::TrianglesList),
        &program,
        &uniform! {},
        &Default::default(),
    );
    assert!(matches!(result, Err(DrawError::TessellationWithoutPatches)));

    display.assert_no_error(None);
}

#[test]
fn patches_without_tessellation() {
    let display = support::build_display();
    if tessellation_program(&display).is_none() {
        return;
    }

    let (_, _, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    let result = texture.as_surface().draw(
        &triangle(&display),
        NoIndices(PrimitiveType::Patches {
            vertices_per_patch: 3,
        }),
        &program,
        &uniform! {},
        &Default::default(),
    );
    assert!(matches!(result, Err(DrawError::PatchesWithoutTessellation)));

    display.assert_no_error(None);
}

#[test]
fn too_many_vertices_per_patch() {
    let display = support::build_display();
    let program = match tessellation_program(&display) {
        Some(program) => program,
        None => return,
    };

    let texture = support::build_renderable_texture(&display);
    let result = texture.as_surface().draw(
        &triangle(&display),
        NoIndices(PrimitiveType::Patches {
            vertices_per_patch: 1000,
        }),
        &program,
        &uniform! {},
        &Default::default(),
    );
    assert!(matches!(result, Err(DrawError::UnsupportedVerticesPerPatch)));

    display.assert_no_error(None);
}
