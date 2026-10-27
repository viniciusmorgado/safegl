//! Attributes that share a location with `layout(component = N)`, OpenGL 4.4 or
//! `GL_ARB_enhanced_layouts`.
#![cfg(safegl_desktop)]

#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::vertex::{ConstantAttribute, MultiVerticesSource};
use safegl::{CapabilitiesSource, DrawError, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// `first` and `second` in the order of their components.
#[derive(Copy, Clone)]
struct Split {
    first: [f32; 2],
    second: [f32; 2],
}
implement_vertex!(Split, first, second);

/// `second` before `first`.
#[derive(Copy, Clone)]
struct Swapped {
    second: [f32; 2],
    first: [f32; 2],
}
implement_vertex!(Swapped, second, first);

/// One member for the whole location.
#[derive(Copy, Clone)]
struct Whole {
    both: [f32; 4],
}
implement_vertex!(Whole, both location(1));

#[derive(Copy, Clone)]
struct First {
    first: [f32; 2],
}
implement_vertex!(First, first);

#[derive(Copy, Clone)]
struct Second {
    second: [f32; 2],
}
implement_vertex!(Second, second);

/// A program whose `first` and `second` attributes share the location 1, or `None` without
/// `layout(component)`.
fn program(display: &Display) -> Option<Program> {
    let header = if support::glsl_supported(display, 440, 0) {
        "#version 440 core\n"
    } else if display
        .get_context()
        .get_extensions()
        .gl_arb_enhanced_layouts
    {
        "#version 400 core\n#extension GL_ARB_enhanced_layouts : require\n\
         #extension GL_ARB_separate_shader_objects : enable\n\
         #extension GL_ARB_explicit_attrib_location : enable\n"
    } else {
        return None;
    };
    Some(
        Program::from_source(
            display,
            &format!(
                "{header}layout(location = 0) in vec2 position;\n\
                 layout(location = 1) in vec2 first;\n\
                 layout(location = 1, component = 2) in vec2 second;\n\
                 flat out vec4 v_color;\n\
                 void main() {{\n\
                     v_color = vec4(first, second);\n\
                     gl_Position = vec4(position, 0.0, 1.0);\n\
                 }}"
            ),
            &format!(
                "{header}flat in vec4 v_color;\nout vec4 color;\n\
                 void main() {{ color = v_color; }}"
            ),
            None,
        )
        .unwrap(),
    )
}

fn triangle(display: &Display) -> VertexBuffer<Vertex> {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

fn draw<'a>(
    display: &Display,
    program: &Program,
    sources: impl MultiVerticesSource<'a>,
) -> Result<(u8, u8, u8, u8), DrawError> {
    let target = support::build_renderable_texture(display);
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    target.as_surface().draw(
        sources,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        &uniform! {},
        &Default::default(),
    )?;
    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    Ok(data[0][0])
}

#[test]
fn components_from_a_vertex() {
    let display = support::build_display();
    let Some(program) = program(&display) else {
        return;
    };
    let positions = triangle(&display);

    // the members follow each other like the components
    let split = Split {
        first: [1.0, 0.0],
        second: [0.2, 1.0],
    };
    let split = VertexBuffer::new(&display, &[split; 3]).unwrap();
    assert_eq!(
        draw(&display, &program, (&positions, &split)).unwrap(),
        (255, 0, 51, 255)
    );

    // one member with the location of the attributes gives all their components
    let whole = Whole {
        both: [0.0, 1.0, 0.2, 1.0],
    };
    let whole = VertexBuffer::new(&display, &[whole; 3]).unwrap();
    assert_eq!(
        draw(&display, &program, (&positions, &whole)).unwrap(),
        (0, 255, 51, 255)
    );

    // the members are in another order than the components
    let swapped = Swapped {
        second: [0.2, 1.0],
        first: [1.0, 0.0],
    };
    let swapped = VertexBuffer::new(&display, &[swapped; 3]).unwrap();
    assert!(matches!(
        draw(&display, &program, (&positions, &swapped)),
        Err(DrawError::AttributeComponentsMismatch)
    ));

    // the members are in different buffers
    let first = VertexBuffer::new(&display, &[First { first: [1.0, 0.0] }; 3]).unwrap();
    let second = VertexBuffer::new(&display, &[Second { second: [0.2, 1.0] }; 3]).unwrap();
    assert!(matches!(
        draw(&display, &program, (&positions, &first, &second)),
        Err(DrawError::AttributeComponentsMismatch)
    ));

    display.assert_no_error(None);
}

#[test]
fn components_from_constants() {
    let display = support::build_display();
    let Some(program) = program(&display) else {
        return;
    };
    let positions = triangle(&display);

    // each constant gives the components of its attribute
    let first = ConstantAttribute::new("first", [0.0f32, 1.0]);
    let second = ConstantAttribute::new("second", [0.2f32, 1.0]);
    assert_eq!(
        draw(&display, &program, (&positions, first, second)).unwrap(),
        (0, 255, 51, 255)
    );
    let other = ConstantAttribute::new("first", [1.0f32, 0.0]);
    assert_eq!(
        draw(&display, &program, (&positions, other, second)).unwrap(),
        (255, 0, 51, 255)
    );

    // a constant and a buffer can't give the same location
    let buffer = VertexBuffer::new(&display, &[Second { second: [0.2, 1.0] }; 3]).unwrap();
    assert!(matches!(
        draw(&display, &program, (&positions, first, &buffer)),
        Err(DrawError::AttributeProvidedTwice)
    ));

    display.assert_no_error(None);
}
