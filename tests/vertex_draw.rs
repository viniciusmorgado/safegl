#[macro_use]
extern crate safegl;

use std::borrow::Cow;

use safegl::draw_parameters::{Stencil, StencilOperation, StencilTest};
use safegl::framebuffer::{DepthStencilRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::DepthStencilFormat;
use safegl::vertex::{AttributeType, VertexBufferAny};
use safegl::{DrawError, Surface, Texture2d, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone, Debug, PartialEq)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);
implement_any_bit_pattern!(Vertex, position);

#[derive(Copy, Clone)]
struct Instance {
    color: [f32; 4],
}
implement_vertex!(Instance, color);
implement_any_bit_pattern!(Instance, color);

/// The `#version` line for desktop OpenGL, or for OpenGL ES with a default precision.
fn header(display: &Display) -> &'static str {
    if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    }
}

fn build_program(display: &Display, vertex: &str, fragment: &str) -> safegl::Program {
    let vertex = format!("{}{}", header(display), vertex);
    let fragment = format!("{}out vec4 f_color;\n{}", header(display), fragment);
    safegl::Program::from_source(display, &vertex, &fragment, None).unwrap()
}

/// Two triangles covering the target.
const QUAD: [Vertex; 6] = [
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
];

fn pixel(texture: &Texture2d) -> (u8, u8, u8, u8) {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[0][0]
}

#[test]
fn clear_stencil_after_masked_draw() {
    let display = support::build_display();
    let (vb, ib, red) = support::build_fullscreen_red_pipeline(&display);
    let color = Texture2d::empty(&display, 2, 2).unwrap();
    let stencil = DepthStencilRenderBuffer::new(&display, DepthStencilFormat::I24I8, 2, 2).unwrap();
    let mut target =
        SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &stencil).unwrap();

    // a draw call that uses the stencil buffer and leaves a write mask of zero
    let masked = safegl::DrawParameters {
        stencil: Stencil {
            depth_pass_operation_clockwise: StencilOperation::Replace,
            depth_pass_operation_counter_clockwise: StencilOperation::Replace,
            write_mask_clockwise: 0,
            write_mask_counter_clockwise: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    target
        .draw(&vb, &ib, &red, &safegl::uniforms::EmptyUniforms, &masked)
        .unwrap();

    // clearing writes the stencil value anyway, so only the second draw passes the test
    target.clear_color_and_stencil((0.0, 0.0, 0.0, 0.0), 5);
    let if_five = safegl::DrawParameters {
        stencil: Stencil {
            test_clockwise: StencilTest::IfEqual { mask: 0xff },
            reference_value_clockwise: 5,
            test_counter_clockwise: StencilTest::IfEqual { mask: 0xff },
            reference_value_counter_clockwise: 5,
            ..Default::default()
        },
        ..Default::default()
    };
    target
        .draw(&vb, &ib, &red, &safegl::uniforms::EmptyUniforms, &if_five)
        .unwrap();

    assert_eq!(pixel(&color), (255, 0, 0, 255));
    display.assert_no_error(None);
}

#[test]
fn instanced_draw_from_a_vertex_slice() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "in vec2 position; in vec4 color; out vec4 v_color;
         void main() { v_color = color; gl_Position = vec4(position, 0.0, 1.0); }",
        "in vec4 v_color; void main() { f_color = v_color; }",
    );

    // the quad starts at the third vertex of the buffer
    let mut vertices = vec![
        Vertex {
            position: [0.0, 0.0]
        };
        2
    ];
    vertices.extend_from_slice(&QUAD);
    let vertices = VertexBuffer::new(&display, &vertices).unwrap();
    let instances = VertexBuffer::new(
        &display,
        &[
            Instance {
                color: [0.0, 0.0, 0.0, 0.0],
            },
            Instance {
                color: [0.0, 1.0, 0.0, 1.0],
            },
        ],
    )
    .unwrap();

    let texture = Texture2d::empty(&display, 2, 2).unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            (
                vertices.slice(2..8).unwrap(),
                instances.slice(1..2).unwrap().per_instance().unwrap(),
            ),
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    assert_eq!(pixel(&texture), (0, 255, 0, 255));
    display.assert_no_error(None);
}

#[test]
fn raw_vertex_data_uses_the_given_stride() {
    let display = support::build_display();
    let (_, _, red) = support::build_fullscreen_red_pipeline(&display);

    // the positions of the quad as bytes: eight bytes per vertex
    let bytes: Vec<u8> = QUAD
        .iter()
        .flat_map(|v| v.position.iter().flat_map(|c| c.to_ne_bytes()))
        .collect();
    const BINDINGS: safegl::vertex::VertexFormat = &[(
        Cow::Borrowed("position"),
        0,
        -1,
        AttributeType::F32F32,
        false,
    )];
    let vertices = unsafe { VertexBuffer::new_raw(&display, &bytes, BINDINGS, 8) }.unwrap();

    let texture = Texture2d::empty(&display, 2, 2).unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            &vertices,
            NoIndices(PrimitiveType::TrianglesList),
            &red,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    assert_eq!(pixel(&texture), (255, 0, 0, 255));
    display.assert_no_error(None);
}

#[test]
#[should_panic(expected = "whole number of vertices")]
fn raw_vertex_data_of_a_wrong_size() {
    let display = support::build_display();
    const BINDINGS: safegl::vertex::VertexFormat = &[(
        Cow::Borrowed("position"),
        0,
        -1,
        AttributeType::F32F32,
        false,
    )];
    let _ = unsafe { VertexBuffer::new_raw(&display, &[0u8; 12], BINDINGS, 8) };
}

#[test]
fn attribute_mismatch_is_an_error() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "in vec3 position; void main() { gl_Position = vec4(position, 1.0); }",
        "void main() { f_color = vec4(1.0); }",
    );
    let vertices = VertexBuffer::new(&display, &QUAD).unwrap();
    let texture = Texture2d::empty(&display, 2, 2).unwrap();

    let result = texture.as_surface().draw(
        &vertices,
        NoIndices(PrimitiveType::TrianglesList),
        &program,
        &safegl::uniforms::EmptyUniforms,
        &Default::default(),
    );
    assert!(matches!(result, Err(DrawError::AttributeTypeMismatch)));

    let program = build_program(
        &display,
        "in vec2 position; in vec2 normal;
         void main() { gl_Position = vec4(position + normal, 0.0, 1.0); }",
        "void main() { f_color = vec4(1.0); }",
    );
    let result = texture.as_surface().draw(
        &vertices,
        NoIndices(PrimitiveType::TrianglesList),
        &program,
        &safegl::uniforms::EmptyUniforms,
        &Default::default(),
    );
    assert!(matches!(result, Err(DrawError::AttributeMissing)));
    display.assert_no_error(None);
}

#[test]
fn many_instance_buffer_offsets() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "in vec2 position; in vec4 color; out vec4 v_color;
         void main() { v_color = color; gl_Position = vec4(position, 0.0, 1.0); }",
        "in vec4 v_color; void main() { f_color = v_color; }",
    );
    let vertices = VertexBuffer::new(&display, &QUAD).unwrap();
    let instances: Vec<Instance> = (0..64)
        .map(|i| Instance {
            color: [i as f32 / 63.0, 0.0, 0.0, 1.0],
        })
        .collect();
    let instances = VertexBuffer::new(&display, &instances).unwrap();
    let texture = Texture2d::empty(&display, 2, 2).unwrap();

    // every offset needs its own vertex array object, of which only a few are kept, so the
    // first offsets are drawn again after theirs were destroyed
    for round in 0..2 {
        for i in 0..64 {
            let instance = instances.slice(i..i + 1).unwrap();
            texture
                .as_surface()
                .draw(
                    (&vertices, instance.per_instance().unwrap()),
                    NoIndices(PrimitiveType::TrianglesList),
                    &program,
                    &safegl::uniforms::EmptyUniforms,
                    &Default::default(),
                )
                .unwrap();
            if round == 1 && (i == 0 || i == 63) {
                let expected = if i == 0 { 0 } else { 255 };
                assert_eq!(pixel(&texture), (expected, 0, 0, 255));
            }
        }
    }
    display.assert_no_error(None);
}

#[test]
fn vertex_buffer_any_back_to_typed() {
    let display = support::build_display();
    let vertices = VertexBuffer::new(&display, &QUAD).unwrap();
    let any: VertexBufferAny = vertices.into();
    assert_eq!(any.get_elements_size(), std::mem::size_of::<Vertex>());

    let vertices: VertexBuffer<Vertex> = unsafe { any.into_vertex_buffer() };
    assert_eq!(vertices.read().unwrap(), QUAD.to_vec());
    display.assert_no_error(None);
}

#[test]
fn indexed_vertex_buffers_of_different_lengths() {
    // with indices, the vertex buffers don't need the same length, and all of them are bound
    let display = support::build_display();

    #[derive(Copy, Clone)]
    struct Unused {
        unused: f32,
    }
    implement_vertex!(Unused, unused);

    #[derive(Copy, Clone)]
    struct Color {
        color: [f32; 4],
    }
    implement_vertex!(Color, color);

    let positions = VertexBuffer::new(
        &display,
        &[
            Vertex { position: [-1.0, -1.0] },
            Vertex { position: [-1.0, 1.0] },
            Vertex { position: [1.0, -1.0] },
            Vertex { position: [1.0, 1.0] },
        ],
    )
    .unwrap();
    let unused = VertexBuffer::new(&display, &[Unused { unused: 0.0 }; 8]).unwrap();
    let colors = VertexBuffer::new(&display, &[Color { color: [0.0, 1.0, 0.0, 1.0] }; 4]).unwrap();
    let indices = safegl::IndexBuffer::new(
        &display,
        PrimitiveType::TriangleStrip,
        &[0u16, 1, 2, 3],
    )
    .unwrap();

    let program = build_program(
        &display,
        "in vec2 position;\nin float unused;\nin vec4 color;\nout vec4 v_color;\nvoid main() { v_color = color + vec4(unused); gl_Position = vec4(position, 0.0, 1.0); }",
        "in vec4 v_color;\nvoid main() { f_color = v_color; }",
    );

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            (&positions, &unused, &colors),
            &indices,
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));

    display.assert_no_error(None);
}
