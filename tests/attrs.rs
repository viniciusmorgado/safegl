#[macro_use]
extern crate safegl;

use safegl::index::PrimitiveType;
use safegl::Surface;

mod support;

#[test]
fn attribute_types_mismatch() {
    let display = support::build_display();

    #[derive(Copy, Clone)]
    struct Vertex {
        field1: [f32; 4],
    }

    implement_vertex!(Vertex, field1);

    let vertex_buffer = safegl::VertexBuffer::new(&display, &Vec::<Vertex>::new()).unwrap();
    let index_buffer =
        safegl::IndexBuffer::new(&display, PrimitiveType::Points, &Vec::<u16>::new()).unwrap();

    let program = program!(&display,
        140 => {
            vertex: "
                #version 140

                attribute vec2 field1;

                void main() {
                    gl_Position = vec4(field1, 0.0, 1.0);
                }
            ",
            fragment: "
                #version 140
                out vec4 color;
                void main() {
                    color = vec4(0.0, 0.0, 0.0, 1.0);
                }
            "
        },
        100 => {
            vertex: "
                #version 100

                attribute mediump vec2 field1;

                void main() {
                    gl_Position = vec4(field1, 0.0, 1.0);
                }
            ",
            fragment: "
                #version 100
                void main() {
                    gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
                }
            "
        }
    )
    .unwrap();

    // drawing a frame
    let mut target = display.draw();
    let result = target.draw(
        &vertex_buffer,
        &index_buffer,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &Default::default(),
    );
    target.finish().unwrap();
    assert!(matches!(result, Err(safegl::DrawError::AttributeTypeMismatch)));

    display.assert_no_error(None);
}

#[test]
fn missing_attribute() {
    let display = support::build_display();

    #[derive(Copy, Clone)]
    struct Vertex {
        field1: [f32; 4],
    }

    implement_vertex!(Vertex, field1);

    let vertex_buffer = safegl::VertexBuffer::new(&display, &Vec::<Vertex>::new()).unwrap();
    let index_buffer =
        safegl::IndexBuffer::new(&display, PrimitiveType::Points, &Vec::<u16>::new()).unwrap();

    let program = program!(&display,
        140 => {
            vertex: "
                #version 140

                attribute vec2 field2;

                void main() {
                    gl_Position = vec4(field2, 0.0, 1.0);
                }
            ",
            fragment: "
                #version 140
                out vec4 color;
                void main() {
                    color = vec4(0.0, 0.0, 0.0, 1.0);
                }
            "
        },
        100 => {
            vertex: "
                #version 100

                attribute mediump vec2 field2;

                void main() {
                    gl_Position = vec4(field2, 0.0, 1.0);
                }
            ",
            fragment: "
                #version 100
                void main() {
                    gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
                }
            "
        }
    )
    .unwrap();

    // drawing a frame
    let mut target = display.draw();
    let result = target.draw(
        &vertex_buffer,
        &index_buffer,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &Default::default(),
    );
    target.finish().unwrap();
    assert!(matches!(result, Err(safegl::DrawError::AttributeMissing)));

    display.assert_no_error(None);
}

macro_rules! attribute_test(
    ($name:ident, $attr_ty:ty, $glsl_ty:expr, $value:expr, $gl_pos:expr) => (
        #[test]
        fn $name() {
            let display = support::build_display();

            #[derive(Copy, Clone)]
            struct Vertex {
                field1: $attr_ty,
            }

            implement_vertex!(Vertex, field1);

            let vertex_buffer = safegl::VertexBuffer::new(&display, &[
                                                             Vertex { field1: $value }
                                                         ]).unwrap();
            let index_buffer = safegl::IndexBuffer::new(&display, PrimitiveType::Points,
                                                       &[0u16]).unwrap();

            let program = program!(&display,
                140 => {
                    vertex: &format!("
                        #version 140

                        in {} field1;

                        void main() {{
                            gl_Position = {};
                        }}
                    ", $glsl_ty, $gl_pos),
                    fragment: "
                        #version 140
                        out vec4 color;
                        void main() {
                            color = vec4(0.0, 0.0, 0.0, 1.0);
                        }
                    "
                },
                100 => {
                    vertex: &format!("
                        #version 100

                        attribute lowp {} field1;

                        void main() {{
                            gl_Position = {};
                        }}
                    ", $glsl_ty, $gl_pos),
                    fragment: "
                        #version 100
                        void main() {
                            gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
                        }
                    "
                }
            ).unwrap();

            // drawing a frame
            let mut target = display.draw();
            target.draw(&vertex_buffer, &index_buffer, &program, &safegl::uniforms::EmptyUniforms,
                        &Default::default()).unwrap();
            target.finish().unwrap();

            display.assert_no_error(None);
        }

    )
);

attribute_test!(
    attribute_float_f32,
    f32,
    "float",
    0.0,
    "vec4(field1, 0.0, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_f32,
    [f32; 2],
    "vec2",
    [0.0, 0.0],
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_tuple_f32,
    (f32, f32),
    "vec2",
    (0.0, 0.0),
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec3_f32,
    [f32; 3],
    "vec3",
    [0.0, 0.0, 0.0],
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec3_tuple_f32,
    (f32, f32, f32),
    "vec3",
    (0.0, 0.0, 0.0),
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec4_f32,
    [f32; 4],
    "vec4",
    [0.0, 0.0, 0.0, 0.0],
    "field1"
);
attribute_test!(
    attribute_vec4_tuple_f32,
    (f32, f32, f32, f32),
    "vec4",
    (0.0, 0.0, 0.0, 0.0),
    "field1"
);

attribute_test!(
    attribute_float_u8,
    u8,
    "float",
    0,
    "vec4(field1, 0.0, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_u8,
    [u8; 2],
    "vec2",
    [0, 0],
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_tuple_u8,
    (u8, u8),
    "vec2",
    (0, 0),
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec3_u8,
    [u8; 3],
    "vec3",
    [0, 0, 0],
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec3_tuple_u8,
    (u8, u8, u8),
    "vec3",
    (0, 0, 0),
    "vec4(field1, 1.0)"
);
attribute_test!(attribute_vec4_u8, [u8; 4], "vec4", [0, 0, 0, 0], "field1");
attribute_test!(
    attribute_vec4_tuple_u8,
    (u8, u8, u8, u8),
    "vec4",
    (0, 0, 0, 0),
    "field1"
);

attribute_test!(
    attribute_float_i8,
    i8,
    "float",
    0,
    "vec4(field1, 0.0, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_i8,
    [i8; 2],
    "vec2",
    [0, 0],
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_tuple_i8,
    (i8, i8),
    "vec2",
    (0, 0),
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec3_i8,
    [i8; 3],
    "vec3",
    [0, 0, 0],
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec3_tuple_i8,
    (i8, i8, i8),
    "vec3",
    (0, 0, 0),
    "vec4(field1, 1.0)"
);
attribute_test!(attribute_vec4_i8, [i8; 4], "vec4", [0, 0, 0, 0], "field1");
attribute_test!(
    attribute_vec4_tuple_i8,
    (i8, i8, i8, i8),
    "vec4",
    (0, 0, 0, 0),
    "field1"
);

attribute_test!(
    attribute_float_u16,
    u16,
    "float",
    0,
    "vec4(field1, 0.0, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_u16,
    [u16; 2],
    "vec2",
    [0, 0],
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_tuple_u16,
    (u16, u16),
    "vec2",
    (0, 0),
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec3_u16,
    [u16; 3],
    "vec3",
    [0, 0, 0],
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec3_tuple_u16,
    (u16, u16, u16),
    "vec3",
    (0, 0, 0),
    "vec4(field1, 1.0)"
);
attribute_test!(attribute_vec4_u16, [u16; 4], "vec4", [0, 0, 0, 0], "field1");
attribute_test!(
    attribute_vec4_tuple_u16,
    (u16, u16, u16, u16),
    "vec4",
    (0, 0, 0, 0),
    "field1"
);

attribute_test!(
    attribute_float_i16,
    i16,
    "float",
    0,
    "vec4(field1, 0.0, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_i16,
    [i16; 2],
    "vec2",
    [0, 0],
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_tuple_i16,
    (i16, i16),
    "vec2",
    (0, 0),
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec3_i16,
    [i16; 3],
    "vec3",
    [0, 0, 0],
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec3_tuple_i16,
    (i16, i16, i16),
    "vec3",
    (0, 0, 0),
    "vec4(field1, 1.0)"
);
attribute_test!(attribute_vec4_i16, [i16; 4], "vec4", [0, 0, 0, 0], "field1");
attribute_test!(
    attribute_vec4_tuple_i16,
    (i16, i16, i16, i16),
    "vec4",
    (0, 0, 0, 0),
    "field1"
);

attribute_test!(
    attribute_float_u32,
    u32,
    "float",
    0,
    "vec4(field1, 0.0, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_u32,
    [u32; 2],
    "vec2",
    [0, 0],
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_tuple_u32,
    (u32, u32),
    "vec2",
    (0, 0),
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec3_u32,
    [u32; 3],
    "vec3",
    [0, 0, 0],
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec3_tuple_u32,
    (u32, u32, u32),
    "vec3",
    (0, 0, 0),
    "vec4(field1, 1.0)"
);
attribute_test!(attribute_vec4_u32, [u32; 4], "vec4", [0, 0, 0, 0], "field1");
attribute_test!(
    attribute_vec4_tuple_u32,
    (u32, u32, u32, u32),
    "vec4",
    (0, 0, 0, 0),
    "field1"
);

attribute_test!(
    attribute_float_i32,
    i32,
    "float",
    0,
    "vec4(field1, 0.0, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_i32,
    [i32; 2],
    "vec2",
    [0, 0],
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec2_tuple_i32,
    (i32, i32),
    "vec2",
    (0, 0),
    "vec4(field1, 0.0, 1.0)"
);
attribute_test!(
    attribute_vec3_i32,
    [i32; 3],
    "vec3",
    [0, 0, 0],
    "vec4(field1, 1.0)"
);
attribute_test!(
    attribute_vec3_tuple_i32,
    (i32, i32, i32),
    "vec3",
    (0, 0, 0),
    "vec4(field1, 1.0)"
);
attribute_test!(attribute_vec4_i32, [i32; 4], "vec4", [0, 0, 0, 0], "field1");
attribute_test!(
    attribute_vec4_tuple_i32,
    (i32, i32, i32, i32),
    "vec4",
    (0, 0, 0, 0),
    "field1"
);

/// A BGRA color reaches the shader as RGBA.
#[cfg(safegl_desktop)]
#[test]
fn bgra_attribute() {
    use safegl::vertex::Bgra8;

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }

    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
        color: Bgra8,
    }
    implement_vertex!(Vertex, position, color);

    // blue, green, red, alpha: red
    let color = Bgra8([0, 0, 255, 255]);
    let vertices =
        [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]].map(|position| Vertex { position, color });
    let vertices = safegl::VertexBuffer::new(&display, &vertices).unwrap();
    let draw = |attribute: &str| {
        let program = safegl::Program::from_source(
            &display,
            &format!(
                "#version 140\nin vec2 position;\nin {attribute} color;\nflat out vec4 v_color;\n\
                 void main() {{ v_color = vec4(color); gl_Position = vec4(position, 0.0, 1.0); }}"
            ),
            "#version 140\nflat in vec4 v_color;\nout vec4 f_color;\n\
             void main() { f_color = v_color; }",
            None,
        )
        .unwrap();
        let target = support::build_renderable_texture(&display);
        target
            .as_surface()
            .draw(
                &vertices,
                safegl::index::NoIndices(safegl::index::PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &Default::default(),
            )
            .map(|()| {
                let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
                data[0][0]
            })
    };

    assert_eq!(draw("vec4").unwrap(), (255, 0, 0, 255));
    // OpenGL only normalizes BGRA attributes to floats
    assert!(matches!(
        draw("ivec4"),
        Err(safegl::DrawError::AttributeTypeMismatch)
    ));

    display.assert_no_error(None);
}
