//! Several ranges of vertices or indices drawn with one call, and the draw parameters that
//! shaders read (`gl_DrawID`, `gl_BaseVertex`, `gl_BaseInstance`).

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndicesRanges, PrimitiveType};
use safegl::{Api, CapabilitiesSource, DrawError, IndexBuffer, Program, Surface, Version};
use safegl::VertexBuffer;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// Three quads that cover the left, middle and right thirds of the target, as triangle strips
/// of 4 vertices.
fn thirds(display: &Display) -> VertexBuffer<Vertex> {
    let mut vertices = Vec::new();
    for third in 0..3 {
        let left = third as f32 * 2.0 / 3.0 - 1.0;
        let right = left + 2.0 / 3.0;
        for position in [[left, -1.0], [right, -1.0], [left, 1.0], [right, 1.0]] {
            vertices.push(Vertex { position });
        }
    }
    VertexBuffer::new(display, &vertices).unwrap()
}

fn red_program(display: &Display) -> Program {
    support::program_from_legacy(
        display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0); }",
    )
    .unwrap()
}

/// Draws `indices` over a black target of 6 pixels, and returns the color of each third.
fn draw<'a>(
    display: &Display,
    vertices: impl safegl::vertex::MultiVerticesSource<'a>,
    indices: impl Into<safegl::index::IndicesSource<'a>>,
    program: &Program,
) -> Result<[(u8, u8, u8, u8); 3], DrawError> {
    let target = safegl::Texture2d::empty(display, 6, 1).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);
    target.as_surface().draw(
        vertices,
        indices,
        program,
        &uniform! {},
        &Default::default(),
    )?;
    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    Ok([0, 2, 4].map(|x| data[0][x]))
}

const BLACK: (u8, u8, u8, u8) = (0, 0, 0, 255);
const RED: (u8, u8, u8, u8) = (255, 0, 0, 255);

#[test]
fn ranges_of_vertices() {
    let display = support::build_display();
    let vertices = thirds(&display);
    let program = red_program(&display);

    let ranges = [0..4, 8..12];
    match draw(
        &display,
        &vertices,
        NoIndicesRanges(PrimitiveType::TriangleStrip, &ranges),
        &program,
    ) {
        Ok(thirds) => assert_eq!(thirds, [RED, BLACK, RED]),
        Err(error) => panic!("{error}"),
    }

    // a slice of the vertices: the ranges start at its first vertex
    let middle = vertices.slice(4..12).unwrap();
    let ranges = [0..4];
    assert_eq!(
        draw(
            &display,
            middle,
            NoIndicesRanges(PrimitiveType::TriangleStrip, &ranges),
            &program
        )
        .unwrap(),
        [BLACK, RED, BLACK]
    );

    let outside = [8..13];
    assert!(matches!(
        draw(
            &display,
            &vertices,
            NoIndicesRanges(PrimitiveType::TriangleStrip, &outside),
            &program
        ),
        Err(DrawError::DrawRangeOutOfBounds)
    ));
    let instances = safegl::vertex::EmptyInstanceAttributes { len: 2 };
    assert!(matches!(
        draw(
            &display,
            (&vertices, instances),
            NoIndicesRanges(PrimitiveType::TriangleStrip, &ranges),
            &program
        ),
        Err(DrawError::InstancedDrawRangesNotSupported)
    ));
    assert_eq!(
        draw(
            &display,
            &vertices,
            NoIndicesRanges(PrimitiveType::TriangleStrip, &[]),
            &program
        )
        .unwrap(),
        [BLACK; 3]
    );

    display.assert_no_error(None);
}

#[test]
fn ranges_of_indices() {
    // OpenGL ES draws them with `GL_EXT_multi_draw_arrays` and WebGL with `WEBGL_multi_draw`, or
    // one by one
    let display = support::build_display();
    let vertices = thirds(&display);
    let program = red_program(&display);

    // the quads as two triangles each
    let indices: Vec<u16> = (0..3u16)
        .flat_map(|third| [0, 1, 2, 1, 3, 2].map(|index| third * 4 + index))
        .collect();
    let indices = IndexBuffer::new(&display, PrimitiveType::TrianglesList, &indices).unwrap();
    let ranges = [6..12];
    assert_eq!(
        draw(&display, &vertices, indices.ranges(&ranges), &program).unwrap(),
        [BLACK, RED, BLACK]
    );
    let outside = [12..19];
    assert!(matches!(
        draw(&display, &vertices, indices.ranges(&outside), &program),
        Err(DrawError::DrawRangeOutOfBounds)
    ));

    // the indices of one quad, moved to the first and the last third
    let quad = IndexBuffer::new(
        &display,
        PrimitiveType::TrianglesList,
        &[0u32, 1, 2, 1, 3, 2],
    )
    .unwrap();
    let ranges = [0..6, 0..6];
    let source = quad.ranges_with_base_vertices(&ranges, &[0, 8]).unwrap();
    // OpenGL ES draws base vertices from version 3.2 or with an extension, and browsers with
    // `WEBGL_draw_instanced_base_vertex_base_instance`
    let extensions = support::extensions(&display);
    let base_vertices = if cfg!(target_arch = "wasm32") {
        extensions.webgl_draw_instanced_base_vertex_base_instance
    } else {
        !support::is_gles(&display)
            || *display.get_opengl_version() >= safegl::Version(safegl::Api::GlEs, 3, 2)
            || extensions.gl_ext_draw_elements_base_vertex
            || extensions.gl_oes_draw_elements_base_vertex
    };
    match draw(&display, &vertices, source, &program) {
        Ok(pixels) => {
            assert!(base_vertices);
            assert_eq!(pixels, [RED, BLACK, RED]);
        }
        Err(DrawError::DrawRangesNotSupported) => assert!(!base_vertices),
        Err(error) => panic!("{error}"),
    }
    assert!(quad.ranges_with_base_vertices(&ranges, &[0]).is_none());

    display.assert_no_error(None);
}

/// The vertex shader header that gives `gl_DrawID`, `gl_BaseVertex` and `gl_BaseInstance`, and
/// the prefix of their names, or `None` without OpenGL 4.6 or `GL_ARB_shader_draw_parameters`.
fn draw_parameters_header(display: &Display) -> Option<(&'static str, &'static str)> {
    if support::is_gles(display) {
        None
    } else if display.get_opengl_version() >= &Version(Api::Gl, 4, 6)
        && support::glsl_supported(display, 460, 0)
    {
        Some(("#version 460\n", ""))
    } else if display.get_extensions().gl_arb_shader_draw_parameters {
        Some((
            "#version 400\n#extension GL_ARB_shader_draw_parameters : require\n",
            "ARB",
        ))
    } else {
        None
    }
}

/// A program that colors each draw of a multidraw by `gl_DrawID`: red, green and blue.
fn draw_id_program(display: &Display) -> Option<Program> {
    let (header, suffix) = draw_parameters_header(display)?;
    let vertex = format!(
        "{header}in vec2 position;\nflat out vec4 v_color;\n\
         void main() {{\n\
             int id = gl_DrawID{suffix};\n\
             v_color = vec4(id == 0 ? 1.0 : 0.0, id == 1 ? 1.0 : 0.0, id == 2 ? 1.0 : 0.0, 1.0);\n\
             gl_Position = vec4(position, 0.0, 1.0);\n\
         }}"
    );
    let version = header.lines().next().unwrap();
    let fragment = format!(
        "{version}\nflat in vec4 v_color;\nout vec4 color;\nvoid main() {{ color = v_color; }}"
    );
    Some(Program::from_source(display, &vertex, &fragment, None).unwrap())
}

#[test]
fn draw_ids_of_ranges() {
    let display = support::build_display();
    let program = match draw_id_program(&display) {
        Some(program) => program,
        None => return,
    };
    let vertices = thirds(&display);

    let ranges = [0..4, 4..8, 8..12];
    assert_eq!(
        draw(
            &display,
            &vertices,
            NoIndicesRanges(PrimitiveType::TriangleStrip, &ranges),
            &program
        )
        .unwrap(),
        [(255, 0, 0, 255), (0, 255, 0, 255), (0, 0, 255, 255)]
    );

    display.assert_no_error(None);
}

/// `gl_DrawID`, `gl_BaseVertex` and `gl_BaseInstance` of the commands of a multidraw.
#[cfg(safegl_gl43)]
#[test]
fn draw_parameters_of_indirect_commands() {
    use safegl::index::{DrawCommandIndices, DrawCommandsIndicesBuffer};
    use safegl::vertex::VertexBufferAny;

    let display = support::build_display();
    let (header, suffix) = match draw_parameters_header(&display) {
        Some(header) => header,
        None => return,
    };
    // the base instances of the commands need OpenGL 4.2 or `GL_ARB_base_instance`
    let extensions = display.get_extensions();
    if !(display.get_opengl_version() >= &Version(Api::Gl, 4, 3)
        || (extensions.gl_arb_multi_draw_indirect && extensions.gl_arb_base_instance))
    {
        return;
    }

    // each command draws the quad of its base vertex, colored by its parameters
    let vertex = format!(
        "{header}in vec2 position;\nflat out vec4 v_color;\n\
         void main() {{\n\
             bool good = gl_BaseVertex{suffix} == gl_DrawID{suffix} * 4\n\
                 && gl_BaseInstance{suffix} == 10 + gl_DrawID{suffix};\n\
             v_color = good ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0, 0.0, 0.0, 1.0);\n\
             gl_Position = vec4(position, 0.0, 1.0);\n\
         }}"
    );
    let version = header.lines().next().unwrap();
    let fragment = format!(
        "{version}\nflat in vec4 v_color;\nout vec4 color;\nvoid main() {{ color = v_color; }}"
    );
    let program = Program::from_source(&display, &vertex, &fragment, None).unwrap();

    let quad = IndexBuffer::new(
        &display,
        PrimitiveType::TrianglesList,
        &[0u16, 1, 2, 1, 3, 2],
    )
    .unwrap();
    let commands = DrawCommandsIndicesBuffer::empty(&display, 3).unwrap();
    let list: Vec<DrawCommandIndices> = (0..3)
        .map(|draw| DrawCommandIndices {
            count: 6,
            instance_count: 1,
            first_index: 0,
            base_vertex: draw * 4,
            base_instance: 10 + draw as u32,
        })
        .collect();
    commands.write(&list);

    let vertices: VertexBufferAny = thirds(&display).into();
    let green = (0, 255, 0, 255);
    assert_eq!(
        draw(
            &display,
            &vertices,
            commands.with_index_buffer(&quad),
            &program
        )
        .unwrap(),
        [green; 3]
    );

    display.assert_no_error(None);
}
