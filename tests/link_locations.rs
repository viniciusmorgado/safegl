//! Locations given to attributes and fragment outputs before linking.

#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::{LinkLocations, ProgramCreationError, ProgramCreationInput};
use safegl::{CapabilitiesSource, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}
implement_vertex!(Vertex, position, color);

/// The header of the shaders: GLSL 3.30 or GLSL ES 3.00.
fn header(display: &Display) -> &'static str {
    if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330 core\n"
    }
}

fn program(
    display: &Display,
    vertex_shader: &str,
    fragment_shader: &str,
    locations: LinkLocations<'_>,
) -> Result<Program, ProgramCreationError> {
    Program::new(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader,
            fragment_shader,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
            locations,
        },
    )
}

/// A vertex shader with the attributes `position` and `color`, and `layout` before `position`.
fn vertex_shader(display: &Display, layout: &str) -> String {
    format!(
        "{}{layout} in vec2 position;\nin vec4 color;\nout vec4 v_color;\n\
         void main() {{ v_color = color; gl_Position = vec4(position, 0.0, 1.0); }}",
        header(display)
    )
}

/// A fragment shader that writes `v_color`.
fn fragment_shader(display: &Display) -> String {
    format!(
        "{}in vec4 v_color;\nout vec4 f_color;\nvoid main() {{ f_color = v_color; }}",
        header(display)
    )
}

#[test]
fn attribute_locations() {
    let display = support::build_display();
    let locations = LinkLocations {
        attributes: &[("position", 3), ("color", 5)],
        ..Default::default()
    };
    let program = program(
        &display,
        &vertex_shader(&display, ""),
        &fragment_shader(&display),
        locations,
    )
    .unwrap();
    assert_eq!(program.get_attribute("position").unwrap().location, 3);
    assert_eq!(program.get_attribute("color").unwrap().location, 5);

    // the vertices reach the attributes at their locations
    let vertices = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]].map(|position| Vertex {
        position,
        color: [0.0, 1.0, 0.0, 1.0],
    });
    let vertices = VertexBuffer::new(&display, &vertices).unwrap();
    let target = support::build_renderable_texture(&display);
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    target
        .as_surface()
        .draw(
            &vertices,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));

    // the locations of the shader win
    let declared = program_with_declared_location(&display, locations);
    assert_eq!(declared.get_attribute("position").unwrap().location, 1);
    assert_eq!(declared.get_attribute("color").unwrap().location, 5);

    display.assert_no_error(None);
}

fn program_with_declared_location(display: &Display, locations: LinkLocations<'_>) -> Program {
    program(
        display,
        &vertex_shader(display, "layout(location = 1)"),
        &fragment_shader(display),
        locations,
    )
    .unwrap()
}

#[test]
fn invalid_locations() {
    let display = support::build_display();
    let vertex = vertex_shader(&display, "");
    let fragment = fragment_shader(&display);
    let invalid = |locations| {
        matches!(
            program(&display, &vertex, &fragment, locations),
            Err(ProgramCreationError::InvalidLinkLocation { .. })
        )
    };

    let max = display
        .get_context()
        .get_capabilities()
        .limits
        .max_vertex_attribs as u32;
    let too_large = [("position", max)];
    assert!(invalid(LinkLocations {
        attributes: &too_large,
        ..Default::default()
    }));
    assert!(invalid(LinkLocations {
        attributes: &[("gl_Position", 0)],
        ..Default::default()
    }));
    assert!(invalid(LinkLocations {
        attributes: &[("posi\0tion", 0)],
        ..Default::default()
    }));

    if support::is_gles(&display)
        && !display
            .get_context()
            .get_extensions()
            .gl_ext_blend_func_extended
    {
        assert!(matches!(
            program(
                &display,
                &vertex,
                &fragment,
                LinkLocations {
                    fragment_outputs: &[("f_color", 0, 0)],
                    ..Default::default()
                }
            ),
            Err(ProgramCreationError::LinkLocationsNotSupported)
        ));
        return;
    }
    let draw_buffers = display.get_context().get_capabilities().max_draw_buffers as u32;
    let too_large = [("f_color", draw_buffers, 0)];
    assert!(invalid(LinkLocations {
        fragment_outputs: &too_large,
        ..Default::default()
    }));
    assert!(invalid(LinkLocations {
        fragment_outputs: &[("f_color", 0, 2)],
        ..Default::default()
    }));

    display.assert_no_error(None);
}

#[test]
fn fragment_output_locations() {
    let display = support::build_display();
    if support::is_gles(&display)
        && !display
            .get_context()
            .get_extensions()
            .gl_ext_blend_func_extended
    {
        return;
    }
    let vertex = vertex_shader(&display, "");
    // GLSL ES needs the extension for several outputs without a location
    let header = if support::is_gles(&display) {
        "#version 300 es\n#extension GL_EXT_blend_func_extended : require\nprecision highp float;\n"
    } else {
        header(&display)
    };
    let fragment = format!(
        "{}in vec4 v_color;\nout vec4 first;\nout vec4 second;\nout vec4 third;\n\
         void main() {{ first = v_color; second = v_color; third = v_color; }}",
        header
    );

    // the outputs without a location take the free ones, in their declaration order
    let locations = LinkLocations {
        fragment_outputs: &[("second", 0, 0)],
        ..Default::default()
    };
    let outputs = program(&display, &vertex, &fragment, locations).unwrap();
    assert_eq!(outputs.get_frag_data_location("second"), Some(0));
    assert_eq!(outputs.get_frag_data_location("first"), Some(1));
    assert_eq!(outputs.get_frag_data_location("third"), Some(2));

    // the second color of dual-source blending
    let dual = format!(
        "{}in vec4 v_color;\nout vec4 color;\nout vec4 factor;\n\
         void main() {{ color = v_color; factor = v_color; }}",
        header
    );
    let locations = LinkLocations {
        fragment_outputs: &[("color", 0, 0), ("factor", 0, 1)],
        ..Default::default()
    };
    let dual = program(&display, &vertex, &dual, locations).unwrap();
    assert_eq!(dual.get_frag_data_location("factor"), Some(0));
    assert_eq!(dual.get_frag_data_index("factor"), Some(1));
    assert_eq!(dual.get_frag_data_index("color"), Some(0));

    display.assert_no_error(None);
}
