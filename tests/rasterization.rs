//! The rasterization and per-fragment state of draws: front faces, multisampling, logic
//! operations, dual-source blending, points and cull distances.

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::{Stencil, StencilTest};
use safegl::framebuffer::{DepthStencilRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::DepthStencilFormat;
use safegl::VertexBuffer;
use safegl::{BackfaceCullingMode, DrawParameters, FrontFace, Program, Surface, Texture2d};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A triangle covering the whole target, counter-clockwise on it, or clockwise if `reversed`.
fn fullscreen_triangle(display: &Display, reversed: bool) -> VertexBuffer<Vertex> {
    let mut positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    if reversed {
        positions.reverse();
    }
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

/// A program whose fragments are green when they face the camera and red otherwise.
fn facing_program(display: &Display) -> Program {
    support::program_from_legacy(
        display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nvoid main() {\n\
             gl_FragColor = gl_FrontFacing ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0, 0.0, 0.0, 1.0);\n\
         }",
    )
    .unwrap()
}

/// Draws `triangle` over black and returns the color of the target.
fn draw(
    display: &Display,
    triangle: &VertexBuffer<Vertex>,
    program: &Program,
    parameters: &DrawParameters<'_>,
) -> (u8, u8, u8, u8) {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);
    target
        .as_surface()
        .draw(
            triangle,
            NoIndices(PrimitiveType::TrianglesList),
            program,
            &uniform! {},
            parameters,
        )
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    data[0][0]
}

const BLACK: (u8, u8, u8, u8) = (0, 0, 0, 255);
const RED: (u8, u8, u8, u8) = (255, 0, 0, 255);
const GREEN: (u8, u8, u8, u8) = (0, 255, 0, 255);

#[test]
fn front_faces() {
    let display = support::build_display();
    let program = facing_program(&display);
    let counter_clockwise = fullscreen_triangle(&display, false);
    let clockwise = fullscreen_triangle(&display, true);
    let with = |front_face| DrawParameters {
        front_face,
        ..Default::default()
    };

    assert_eq!(
        draw(
            &display,
            &counter_clockwise,
            &program,
            &with(FrontFace::CounterClockwise)
        ),
        GREEN
    );
    assert_eq!(
        draw(
            &display,
            &clockwise,
            &program,
            &with(FrontFace::CounterClockwise)
        ),
        RED
    );
    assert_eq!(
        draw(
            &display,
            &counter_clockwise,
            &program,
            &with(FrontFace::Clockwise)
        ),
        RED
    );
    assert_eq!(
        draw(&display, &clockwise, &program, &with(FrontFace::Clockwise)),
        GREEN
    );

    display.assert_no_error(None);
}

/// Culling names the triangles by their winding, whichever faces the camera.
#[test]
fn culling_by_winding() {
    let display = support::build_display();
    let program = facing_program(&display);
    let counter_clockwise = fullscreen_triangle(&display, false);
    let clockwise = fullscreen_triangle(&display, true);

    for front_face in [FrontFace::CounterClockwise, FrontFace::Clockwise] {
        let parameters = DrawParameters {
            front_face,
            backface_culling: BackfaceCullingMode::CullCounterClockwise,
            ..Default::default()
        };
        assert_eq!(
            draw(&display, &counter_clockwise, &program, &parameters),
            BLACK
        );
        assert_ne!(draw(&display, &clockwise, &program, &parameters), BLACK);
    }

    display.assert_no_error(None);
}

/// The two-sided stencil settings are named by winding too.
#[test]
fn stencil_by_winding() {
    let display = support::build_display();
    let program = facing_program(&display);
    let counter_clockwise = fullscreen_triangle(&display, false);
    let clockwise = fullscreen_triangle(&display, true);
    let color = Texture2d::empty(&display, 4, 4).unwrap();
    let depth_stencil =
        DepthStencilRenderBuffer::new(&display, DepthStencilFormat::I24I8, 4, 4).unwrap();

    let drawn = |triangle: &VertexBuffer<Vertex>, front_face| {
        let mut framebuffer =
            SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &depth_stencil).unwrap();
        framebuffer.clear_color_and_stencil((0.0, 0.0, 0.0, 1.0), 0);
        framebuffer
            .draw(
                triangle,
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &DrawParameters {
                    front_face,
                    stencil: Stencil {
                        test_clockwise: StencilTest::AlwaysPass,
                        test_counter_clockwise: StencilTest::AlwaysFail,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        let data: Vec<Vec<(u8, u8, u8, u8)>> = color.read().unwrap();
        data[0][0] != BLACK
    };

    for front_face in [FrontFace::CounterClockwise, FrontFace::Clockwise] {
        assert!(!drawn(&counter_clockwise, front_face));
        assert!(drawn(&clockwise, front_face));
    }

    display.assert_no_error(None);
}
