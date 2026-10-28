//! Attributes with the same value for every vertex, which no buffer provides.

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::vertex::ConstantAttribute;
use safegl::{DrawError, Program, Surface, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
struct ColoredVertex {
    position: [f32; 2],
    color: [f32; 4],
}
implement_vertex!(ColoredVertex, position, color);

/// A triangle that covers the target.
fn triangle(display: &Display) -> VertexBuffer<Vertex> {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

/// A program with a `color` attribute declared as `declaration`, whose fragment color is
/// `color_expression` of it.
fn program(display: &Display, declaration: &str, color_expression: &str) -> Program {
    let (header, flat) = if support::is_gles(display) {
        ("#version 300 es\nprecision highp float;\n", "flat ")
    } else {
        ("#version 140\n", "flat ")
    };
    Program::from_source(
        display,
        &format!(
            "{header}in vec2 position;\nin {declaration} color;\n{flat}out vec4 v_color;\n\
             void main() {{ v_color = {color_expression}; gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        &format!(
            "{header}{flat}in vec4 v_color;\nout vec4 f_color;\nvoid main() {{ f_color = v_color; }}"
        ),
        None,
    )
    .unwrap()
}

/// Draws `sources` with `program` and returns the color of the target.
fn draw<'a, V>(
    display: &Display,
    sources: V,
    program: &Program,
) -> Result<(u8, u8, u8, u8), DrawError>
where
    V: safegl::vertex::MultiVerticesSource<'a>,
{
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
fn float_constants() {
    let display = support::build_display();
    let vertices = triangle(&display);
    let program = program(&display, "vec4", "color");

    let red = ConstantAttribute::new("color", [1.0f32, 0.0, 0.0, 1.0]);
    let green = ConstantAttribute::new("color", [0.0f32, 1.0, 0.0, 1.0]);
    assert_eq!(
        draw(&display, (&vertices, red), &program).unwrap(),
        (255, 0, 0, 255)
    );
    assert_eq!(
        draw(&display, (&vertices, green), &program).unwrap(),
        (0, 255, 0, 255)
    );
    assert_eq!(
        draw(&display, (&vertices, red), &program).unwrap(),
        (255, 0, 0, 255)
    );

    // fewer components: the fourth one is 1
    let program = program_vec3(&display);
    let blue = ConstantAttribute::new("color", [0.0f32, 0.0, 1.0]);
    assert_eq!(
        draw(&display, (&vertices, blue), &program).unwrap(),
        (0, 0, 255, 255)
    );

    display.assert_no_error(None);
}

fn program_vec3(display: &Display) -> Program {
    program(display, "vec3", "vec4(color, 1.0)")
}

#[test]
fn integer_constants() {
    let display = support::build_display();
    let vertices = triangle(&display);

    let signed = program(
        &display,
        "int",
        "color == -7 ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0)",
    );
    let constant = ConstantAttribute::new("color", -7i32);
    assert_eq!(
        draw(&display, (&vertices, constant), &signed).unwrap(),
        (0, 255, 0, 255)
    );

    let unsigned = program(
        &display,
        "uvec2",
        "color == uvec2(3u, 4000000000u) ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0)",
    );
    let constant = ConstantAttribute::new("color", [3u32, 4_000_000_000]);
    assert_eq!(
        draw(&display, (&vertices, constant), &unsigned).unwrap(),
        (0, 255, 0, 255)
    );

    display.assert_no_error(None);
}

#[test]
fn constants_of_the_wrong_type() {
    let display = support::build_display();
    let vertices = triangle(&display);

    let vec4 = program(&display, "vec4", "color");
    for value in [
        ConstantAttribute::new("color", [1.0f32, 0.0, 0.0]),
        ConstantAttribute::new("color", [1i32, 0, 0, 1]),
        ConstantAttribute::new("color", [1u32, 0, 0, 1]),
    ] {
        assert!(matches!(
            draw(&display, (&vertices, value), &vec4),
            Err(DrawError::AttributeTypeMismatch)
        ));
    }
    let int = program(&display, "int", "vec4(float(color))");
    let float = ConstantAttribute::new("color", 1.0f32);
    assert!(matches!(
        draw(&display, (&vertices, float), &int),
        Err(DrawError::AttributeTypeMismatch)
    ));

    display.assert_no_error(None);
}

#[test]
fn attributes_provided_twice_or_missing() {
    let display = support::build_display();
    let program = program(&display, "vec4", "color");
    let red = ConstantAttribute::new("color", [1.0f32, 0.0, 0.0, 1.0]);

    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let colored = positions.map(|position| ColoredVertex {
        position,
        color: [0.0, 1.0, 0.0, 1.0],
    });
    let colored = VertexBuffer::new(&display, &colored).unwrap();
    assert!(matches!(
        draw(&display, (&colored, red), &program),
        Err(DrawError::AttributeProvidedTwice)
    ));
    let vertices = triangle(&display);
    assert!(matches!(
        draw(&display, (&vertices, red, red), &program),
        Err(DrawError::AttributeProvidedTwice)
    ));

    // without the constant, the buffers lack the attribute, even after a draw with it
    assert_eq!(
        draw(&display, (&vertices, red), &program).unwrap(),
        (255, 0, 0, 255)
    );
    assert!(matches!(
        draw(&display, &vertices, &program),
        Err(DrawError::AttributeMissing)
    ));
    // and the buffer alone provides it
    assert_eq!(
        draw(&display, &colored, &program).unwrap(),
        (0, 255, 0, 255)
    );

    // a constant that the program doesn't have is ignored
    let unused = ConstantAttribute::new("unused", 1.0f32);
    assert_eq!(
        draw(&display, (&colored, unused), &program).unwrap(),
        (0, 255, 0, 255)
    );

    display.assert_no_error(None);
}

/// Double constants need OpenGL 4.1 or `GL_ARB_vertex_attrib_64bit`.
#[cfg(safegl_gl41)]
#[test]
fn double_constants() {
    use safegl::{Api, CapabilitiesSource, Version};

    let display = support::build_display();
    if support::is_gles(&display)
        || !(display.get_opengl_version() >= &Version(Api::Gl, 4, 1)
            || display.get_extensions().gl_arb_vertex_attrib_64bit)
        || !support::glsl_supported(&display, 410, 0)
    {
        return;
    }
    let vertices = triangle(&display);
    let program = Program::from_source(
        &display,
        "#version 410\nin vec2 position;\nin dvec2 color;\nflat out vec4 v_color;\n\
         void main() {\n\
             v_color = color == dvec2(1.0lf + 1.0lf / 1073741824.0lf, 1099511627776.0lf)
                 ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);\n\
             gl_Position = vec4(position, 0.0, 1.0);\n\
         }",
        "#version 410\nflat in vec4 v_color;\nout vec4 f_color;\nvoid main() { f_color = v_color; }",
        None,
    )
    .unwrap();

    // 1 + 2^-30 has more precision than a float, and 2^40 more range than an integer
    let constant = ConstantAttribute::new("color", [1.0 + 2f64.powi(-30), 2f64.powi(40)]);
    assert_eq!(
        draw(&display, (&vertices, constant), &program).unwrap(),
        (0, 255, 0, 255)
    );

    display.assert_no_error(None);
}

/// A value changed behind SafeGL's back is set again by `restore_state`.
#[test]
fn constants_after_foreign_state_changes() {
    use safegl::glow::HasContext;

    let display = support::build_display();
    let vertices = triangle(&display);
    let program = program(&display, "vec4", "color");
    let red = ConstantAttribute::new("color", [1.0f32, 0.0, 0.0, 1.0]);
    assert_eq!(
        draw(&display, (&vertices, red), &program).unwrap(),
        (255, 0, 0, 255)
    );

    let location = program.get_attribute("color").unwrap().location as u32;
    unsafe {
        display.exec_in_context(|| {
            display
                .glow_context()
                .vertex_attrib_4_f32(location, 0.0, 0.0, 1.0, 1.0);
        });
    }
    display.restore_state();

    assert_eq!(
        draw(&display, (&vertices, red), &program).unwrap(),
        (255, 0, 0, 255)
    );

    display.assert_no_error(None);
}
