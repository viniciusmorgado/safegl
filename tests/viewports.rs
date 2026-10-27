//! Viewport arrays: several viewports, depth ranges and scissor boxes among which a geometry
//! shader chooses with `gl_ViewportIndex`.
#![cfg(safegl_gl41)]

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

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

const SIZE: u32 = 8;

/// A program whose geometry shader sends the triangle to the viewports 0 to 3, each in its own
/// color, or to the viewport `index` only if it isn't negative. `None` without viewport arrays.
fn program(display: &Display) -> Option<Program> {
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
        flat in int viewport;
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
    // below OpenGL 4.1, `GL_ARB_viewport_array` gives `gl_ViewportIndex` to GLSL 4.00
    let header = if support::glsl_supported(display, 410, 0) {
        "#version 410 core\n"
    } else {
        "#version 400 core\n#extension GL_ARB_viewport_array : require\n"
    };
    let [vertex, geometry, fragment] = [VERTEX, GEOMETRY, FRAGMENT].map(|s| format!("{header}{s}"));
    let program = program!(display,
        400 => {
            vertex: &vertex,
            geometry: geometry.as_str(),
            fragment: &fragment,
        },
    )
    .unwrap();
    Some(program)
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
