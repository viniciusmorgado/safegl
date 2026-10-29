//! Viewport arrays: several viewports, depth ranges and scissor boxes among which a geometry
//! shader chooses with `gl_ViewportIndex`.
#![cfg(any(safegl_gl41, safegl_gles32))]

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::DepthTest;
use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::DepthFormat;
use safegl::{
    CapabilitiesSource, Depth, DrawError, DrawParameters, Program, Rect, Surface, Texture2d,
    VertexBuffer, ViewportState,
};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

const SIZE: u32 = 8;

/// A program whose geometry shader sends the triangle to the viewports 0 to 3, each in its own
/// color, or to the viewport `index` only if it isn't negative. `None` without viewport arrays.
fn program(display: &Display) -> Option<Program> {
    program_reading(display, false)
}

/// Like `program`, with a fragment shader that reads `gl_ViewportIndex` itself if
/// `read_index`, which needs GLSL 4.30 or `GL_ARB_fragment_layer_viewport`.
fn program_reading(display: &Display, read_index: bool) -> Option<Program> {
    display.get_capabilities().limits.max_viewports?;
    const VERTEX: &str = "
        in vec2 position;
        void main() { gl_Position = vec4(position, 0.0, 1.0); }";
    const GEOMETRY: &str = "
        layout(triangles, invocations = 4) in;
        layout(triangle_strip, max_vertices = 3) out;
        uniform int index;
        flat out int viewport;
        void main() {
            if (index >= 0 && gl_InvocationID != 0) {
                return;
            }
            for (int i = 0; i < 3; i++) {
                gl_Position = gl_in[i].gl_Position;
                gl_ViewportIndex = index >= 0 ? index : gl_InvocationID;
                viewport = gl_ViewportIndex;
                EmitVertex();
            }
            EndPrimitive();
        }";
    const FRAGMENT: &str = "
        #ifndef READ_INDEX
        flat in int viewport;
        #else
        #define viewport gl_ViewportIndex
        #endif
        out vec4 color;
        void main() {
            vec4 colors[4] = vec4[4](
                vec4(1.0, 0.0, 0.0, 1.0),
                vec4(0.0, 1.0, 0.0, 1.0),
                vec4(0.0, 0.0, 1.0, 1.0),
                vec4(1.0, 1.0, 1.0, 1.0)
            );
            color = colors[viewport];
        }";
    // OpenGL ES has `gl_ViewportIndex` with `GL_OES_viewport_array`, and desktop OpenGL below
    // 4.1 with `GL_ARB_viewport_array` in GLSL 4.00
    let es = support::is_gles(display);
    let es_header = |extensions: &str| {
        format!(
            "#version 320 es\n#extension GL_OES_viewport_array : require\n{extensions}\
             precision highp float;\nprecision highp int;\n"
        )
    };
    let header = if es {
        es_header("")
    } else if support::glsl_supported(display, 410, 0) {
        "#version 410 core\n".to_owned()
    } else {
        "#version 400 core\n#extension GL_ARB_viewport_array : require\n".to_owned()
    };
    // fragment shaders read it from GLSL 4.30, or with `GL_ARB_fragment_layer_viewport`
    let fragment_header = if read_index && es {
        // NVIDIA's compiler gives it to fragment shaders with its own extension
        let nv = if support::has_extension(display, "GL_NV_viewport_array") {
            "#extension GL_NV_viewport_array : enable\n"
        } else {
            ""
        };
        es_header(nv) + "#define READ_INDEX\n"
    } else if read_index {
        let extension = "GL_ARB_fragment_layer_viewport";
        let available = support::has_extension(display, extension);
        let mut header = support::glsl_header(display, 430, &[(extension, available)])?;
        if !support::glsl_supported(display, 430, 0) {
            header.push_str("#extension GL_ARB_viewport_array : require\n");
        }
        header + "#define READ_INDEX\n"
    } else {
        header.clone()
    };
    let [vertex, geometry] = [VERTEX, GEOMETRY].map(|s| format!("{header}{s}"));
    let fragment = format!("{fragment_header}{FRAGMENT}");
    let program = Program::from_source(display, &vertex, &fragment, Some(geometry.as_str()));
    Some(program.unwrap())
}

fn fullscreen_triangle(display: &Display) -> VertexBuffer<Vertex> {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

/// The four quadrants of the target: lower left, lower right, upper left, upper right.
fn quadrants() -> [ViewportState; 4] {
    let half = SIZE / 2;
    [(0, 0), (half, 0), (0, half), (half, half)].map(|(left, bottom)| {
        ViewportState::from(Rect {
            left,
            bottom,
            width: half,
            height: half,
        })
    })
}

const BLACK: (u8, u8, u8, u8) = (0, 0, 0, 255);
const RED: (u8, u8, u8, u8) = (255, 0, 0, 255);
const GREEN: (u8, u8, u8, u8) = (0, 255, 0, 255);
const BLUE: (u8, u8, u8, u8) = (0, 0, 255, 255);
const WHITE: (u8, u8, u8, u8) = (255, 255, 255, 255);

/// Draws into a cleared target with a depth buffer cleared to 0.75, and returns its pixels.
fn draw(
    display: &Display,
    program: &Program,
    index: i32,
    parameters: &DrawParameters<'_>,
) -> Result<Vec<Vec<(u8, u8, u8, u8)>>, DrawError> {
    let color = Texture2d::empty(display, SIZE, SIZE).unwrap();
    let depth = DepthRenderBuffer::new(display, DepthFormat::I24, SIZE, SIZE).unwrap();
    let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(display, &color, &depth).unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 0.75);
    framebuffer.draw(
        &fullscreen_triangle(display),
        NoIndices(PrimitiveType::TrianglesList),
        program,
        &uniform! { index: index },
        parameters,
    )?;
    Ok(color.read().unwrap())
}

#[test]
fn viewports_by_index() {
    let display = support::build_display();
    let Some(program) = program(&display) else {
        return;
    };
    let mut viewports = quadrants();
    // the scissor box of viewport 1 keeps its left column, the depth range of viewport 3 puts
    // its triangle behind the depth buffer
    viewports[1].scissor = Some(Rect {
        left: SIZE / 2,
        bottom: 0,
        width: 1,
        height: SIZE / 2,
    });
    viewports[3].depth_range = (1.0, 1.0);
    let parameters = DrawParameters {
        viewports: &viewports,
        depth: Depth {
            test: DepthTest::IfLess,
            ..Default::default()
        },
        ..Default::default()
    };
    let pixels = draw(&display, &program, -1, &parameters).unwrap();

    let (low, high) = (0, SIZE as usize - 1);
    let half = SIZE as usize / 2;
    assert_eq!(pixels[low][low], RED);
    assert_eq!(pixels[low][half], GREEN);
    assert_eq!(pixels[low][high], BLACK);
    assert_eq!(pixels[high][low], BLUE);
    assert_eq!(pixels[high][high], BLACK);

    // a clear afterwards clears the whole target
    let color = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    color.as_surface().clear_color(1.0, 1.0, 1.0, 1.0);
    let data: Vec<Vec<(u8, u8, u8, u8)>> = color.read().unwrap();
    assert!(data.iter().flatten().all(|pixel| *pixel == WHITE));

    // a draw without the viewport array uses the whole surface at every viewport index
    let pixels = draw(&display, &program, 3, &Default::default()).unwrap();
    assert!(pixels.iter().flatten().all(|pixel| *pixel == WHITE));

    display.assert_no_error(None);
}

/// Viewports keep fractions of pixels.
#[test]
fn fractional_viewports() {
    let display = support::build_display();
    let Some(program) = program(&display) else {
        return;
    };
    let viewport = ViewportState {
        left: 0.5,
        bottom: 0.0,
        width: SIZE as f32 - 0.5,
        height: SIZE as f32,
        depth_range: (0.0, 1.0),
        scissor: None,
    };
    let pixels = draw(
        &display,
        &program,
        0,
        &DrawParameters {
            viewports: &[viewport],
            ..Default::default()
        },
    )
    .unwrap();
    // the pixels whose centers are inside the viewport are covered
    assert!(pixels
        .iter()
        .all(|row| row[1..].iter().all(|pixel| *pixel == RED)));

    display.assert_no_error(None);
}

#[test]
fn viewport_array_errors() {
    let display = support::build_display();
    let triangle = fullscreen_triangle(&display);
    let draw_with = |program: &Program, parameters: &DrawParameters<'_>| {
        let target = Texture2d::empty(&display, SIZE, SIZE).unwrap();
        target.as_surface().draw(
            &triangle,
            NoIndices(PrimitiveType::TrianglesList),
            program,
            &uniform! { index: 0 },
            parameters,
        )
    };
    let Some(program) = program(&display) else {
        // an OpenGL ES context of a build with both chains
        let program = support::build_fullscreen_red_pipeline(&display).2;
        let viewports = quadrants();
        let parameters = DrawParameters {
            viewports: &viewports,
            ..Default::default()
        };
        assert!(matches!(
            draw_with(&program, &parameters),
            Err(DrawError::ViewportArrayNotSupported)
        ));
        return;
    };

    let viewports = quadrants();
    let rect = Some(Rect {
        left: 0,
        bottom: 0,
        width: 1,
        height: 1,
    });
    let conflicts = [
        DrawParameters {
            viewports: &viewports,
            viewport: rect,
            ..Default::default()
        },
        DrawParameters {
            viewports: &viewports,
            scissor: rect,
            ..Default::default()
        },
        DrawParameters {
            viewports: &viewports,
            depth: Depth {
                range: (0.0, 0.5),
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    for parameters in &conflicts {
        assert!(matches!(
            draw_with(&program, parameters),
            Err(DrawError::ViewportArrayConflict)
        ));
    }

    let max = display.get_capabilities().limits.max_viewports.unwrap() as usize;
    let too_many = vec![viewports[0]; max + 1];
    let parameters = DrawParameters {
        viewports: &too_many,
        ..Default::default()
    };
    assert!(matches!(
        draw_with(&program, &parameters),
        Err(DrawError::ViewportIndexOutOfBounds)
    ));

    let mut negative = viewports;
    negative[2].width = -1.0;
    let parameters = DrawParameters {
        viewports: &negative,
        ..Default::default()
    };
    assert!(matches!(
        draw_with(&program, &parameters),
        Err(DrawError::InvalidViewport)
    ));

    display.assert_no_error(None);
}

/// Fragment shaders read the viewport of their primitive in `gl_ViewportIndex`.
#[test]
fn fragment_shaders_read_the_viewport_index() {
    let display = support::build_display();
    let Some(program) = program_reading(&display, true) else {
        return;
    };
    let viewports = quadrants();
    let parameters = DrawParameters {
        viewports: &viewports,
        ..Default::default()
    };
    let pixels = draw(&display, &program, -1, &parameters).unwrap();

    let (low, high) = (0, SIZE as usize - 1);
    assert_eq!(pixels[low][low], RED);
    assert_eq!(pixels[low][high], GREEN);
    assert_eq!(pixels[high][low], BLUE);
    assert_eq!(pixels[high][high], WHITE);
    display.assert_no_error(None);
}
