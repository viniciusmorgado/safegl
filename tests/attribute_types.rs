//! Vertex attributes of the types of OpenGL 4.1 and 4.4 read from vertex buffers: doubles
//! (`glVertexAttribLPointer`) and packed unsigned floats (`GL_UNSIGNED_INT_10F_11F_11F_REV`).
#![cfg(safegl_gl41)]

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::{Program, Surface, Texture2d, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

/// The corners of a triangle that covers the whole target.
const TRIANGLE: [[f32; 2]; 3] = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];

/// Draws `vertices` with `program` on a cleared 4x4 target and returns its first pixel.
fn draw<V: safegl::Vertex>(
    display: &Display,
    vertices: &VertexBuffer<V>,
    program: &Program,
) -> (u8, u8, u8, u8) {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    target
        .as_surface()
        .draw(
            vertices,
            NoIndices(PrimitiveType::TrianglesList),
            program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    pixels[0][0]
}

/// A program with a vertex shader whose inputs are `inputs` and that writes `position` and
/// `color`, in GLSL `version` or a lower version with `extensions`.
fn program(
    display: &Display,
    version: u32,
    extensions: &[&str],
    inputs: &str,
    body: &str,
) -> Option<Program> {
    let extensions: Vec<(&str, bool)> = extensions
        .iter()
        .map(|&name| (name, support::has_extension(display, name)))
        .collect();
    let header = support::glsl_header(display, version, &extensions)?;
    let vertex = format!(
        "{header}{inputs}\nout vec3 v_color;\nvoid main() {{ {body} }}"
    );
    let fragment = format!(
        "{header}in vec3 v_color;\nout vec4 color;\nvoid main() {{ color = vec4(v_color, 1.0); }}"
    );
    Some(Program::from_source(display, &vertex, &fragment, None).unwrap())
}

/// Doubles read from a vertex buffer as doubles, not converted to floats.
#[test]
fn double_attributes() {
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f64; 2],
    }
    implement_vertex!(Vertex, position);

    let display = support::build_display();
    let Some(program) = program(
        &display,
        410,
        &["GL_ARB_vertex_attrib_64bit"],
        "in dvec2 position;",
        "gl_Position = vec4(vec2(position), 0.0, 1.0); v_color = vec3(0.0, 1.0, 0.0);",
    ) else {
        return;
    };
    let vertices: Vec<Vertex> = TRIANGLE
        .iter()
        .map(|&[x, y]| Vertex {
            position: [x as f64, y as f64],
        })
        .collect();
    let vertices = VertexBuffer::new(&display, &vertices).unwrap();
    assert_eq!(draw(&display, &vertices, &program), (0, 255, 0, 255));
    display.assert_no_error(None);
}

/// Three unsigned floats of 11, 11 and 10 bits packed in 32 bits, in a type of the user.
#[cfg(safegl_gl44)]
#[test]
fn packed_float_attributes() {
    use safegl::vertex::{Attribute, AttributeType};

    /// Red in the low 11 bits, green in the next 11, blue in the high 10.
    #[derive(Copy, Clone)]
    // read by the GPU only
    struct Packed(#[allow(dead_code)] u32);
    unsafe impl Attribute for Packed {
        const TYPE: AttributeType = AttributeType::F10F11F11UnsignedIntReversed;
    }

    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
        color: Packed,
    }
    implement_vertex!(Vertex, position, color);

    let display = support::build_display();
    let Some(program) = program(
        &display,
        330,
        &[],
        "in vec2 position;\nin vec3 color;",
        "gl_Position = vec4(position, 0.0, 1.0); v_color = color;",
    ) else {
        return;
    };
    // 1.0 as an 11-bit float: the exponent 15 above the 6 bits of the mantissa
    let green = Packed(0x3c0 << 11);
    let vertices: Vec<Vertex> = TRIANGLE
        .iter()
        .map(|&position| Vertex {
            position,
            color: green,
        })
        .collect();
    let vertices = VertexBuffer::new(&display, &vertices);
    // the type is core in OpenGL 4.4, or `GL_ARB_vertex_type_10f_11f_11f_rev`
    let supported = support::gl_at_least(&display, 4, 4)
        || support::extensions(&display).gl_arb_vertex_type_10f_11f_11f_rev;
    match vertices {
        Ok(vertices) => {
            assert!(supported);
            assert_eq!(draw(&display, &vertices, &program), (0, 255, 0, 255));
        }
        Err(_) => assert!(!supported),
    }
    display.assert_no_error(None);
}
