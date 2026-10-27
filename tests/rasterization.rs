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

/// Draws a fullscreen triangle of the color `color` into `surface`, with `logic_op`.
#[cfg(safegl_desktop)]
fn draw_with_logic_op<S: Surface>(
    display: &Display,
    surface: &mut S,
    integer: bool,
    color: [u32; 4],
    parameters: &DrawParameters<'_>,
) -> Result<(), safegl::DrawError> {
    let fragment = if integer {
        "#version 330 core\nuniform uvec4 color;\nout uvec4 f_color;\nvoid main() { f_color = color; }"
    } else {
        "#version 330 core\nuniform vec4 color;\nout vec4 f_color;\nvoid main() { f_color = color; }"
    };
    let program = Program::from_source(
        display,
        "#version 330 core\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        fragment,
        None,
    )
    .unwrap();
    let triangle = fullscreen_triangle(display, false);
    let indices = NoIndices(PrimitiveType::TrianglesList);
    if integer {
        surface.draw(&triangle, indices, &program, &uniform! { color: color }, parameters)
    } else {
        let color = color.map(|c| c as f32 / 255.0);
        surface.draw(&triangle, indices, &program, &uniform! { color: color }, parameters)
    }
}

#[cfg(safegl_desktop)]
#[test]
fn logic_ops_on_normalized_targets() {
    use safegl::LogicOp;

    let display = support::build_display();
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    let draw = |logic_op| {
        target
            .as_surface()
            .clear_color(0x0F as f32 / 255.0, 0xF0 as f32 / 255.0, 0x33 as f32 / 255.0, 1.0);
        let parameters = DrawParameters {
            logic_op,
            ..Default::default()
        };
        let result = draw_with_logic_op(
            &display,
            &mut target.as_surface(),
            false,
            [0xFF, 0xFF, 0x55, 0xFF],
            &parameters,
        );
        result.map(|()| {
            let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
            data[0][0]
        })
    };

    match draw(Some(LogicOp::Xor)) {
        Err(safegl::DrawError::LogicOpNotSupported) => {
            assert!(support::is_gles(&display));
            return;
        }
        result => assert_eq!(result.unwrap(), (0xF0, 0x0F, 0x66, 0x00)),
    }
    assert_eq!(draw(Some(LogicOp::And)).unwrap(), (0x0F, 0xF0, 0x11, 0xFF));
    assert_eq!(draw(Some(LogicOp::Noop)).unwrap(), (0x0F, 0xF0, 0x33, 0xFF));
    assert_eq!(draw(Some(LogicOp::Invert)).unwrap(), (0xF0, 0x0F, 0xCC, 0x00));
    assert_eq!(draw(Some(LogicOp::Clear)).unwrap(), (0, 0, 0, 0));
    // without a logic operation, the color is written again
    assert_eq!(draw(None).unwrap(), (0xFF, 0xFF, 0x55, 0xFF));

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn logic_ops_on_integer_targets() {
    use safegl::texture::UnsignedTexture2d;
    use safegl::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
    use safegl::LogicOp;

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let texels = vec![vec![(0x0Fu8, 0xF0u8, 0x33u8, 0x01u8); 4]; 4];
    let target = UnsignedTexture2d::new(&display, texels).unwrap();
    draw_with_logic_op(
        &display,
        &mut SimpleFrameBuffer::new(&display, &target).unwrap(),
        true,
        [0xFF, 0x0F, 0x55, 0x03],
        &DrawParameters {
            logic_op: Some(LogicOp::Or),
            ..Default::default()
        },
    )
    .unwrap();

    // the integers are copied into a normalized texture to be read
    let program = Program::from_source(
        &display,
        "#version 330 core\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 330 core\nuniform usampler2D tex;\nout vec4 f_color;\n\
         void main() { f_color = vec4(texelFetch(tex, ivec2(0), 0)) / 255.0; }",
        None,
    )
    .unwrap();
    let copy = Texture2d::empty(&display, 4, 4).unwrap();
    let sampled = target
        .sampled()
        .magnify_filter(MagnifySamplerFilter::Nearest)
        .minify_filter(MinifySamplerFilter::Nearest);
    copy.as_surface()
        .draw(
            &fullscreen_triangle(&display, false),
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &uniform! { tex: sampled },
            &Default::default(),
        )
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = copy.read().unwrap();
    assert_eq!(data[0][0], (0xFF, 0xFF, 0x77, 0x03));

    display.assert_no_error(None);
}

/// OpenGL ignores blending on the buffers that a logic operation writes.
#[cfg(safegl_desktop)]
#[test]
fn logic_ops_refuse_blending() {
    use safegl::{LogicOp, OutputState};

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    let outputs = [OutputState {
        blend: safegl::Blend::alpha_blending(),
        ..Default::default()
    }];
    let refused = [
        DrawParameters {
            logic_op: Some(LogicOp::Xor),
            blend: safegl::Blend::alpha_blending(),
            ..Default::default()
        },
        DrawParameters {
            logic_op: Some(LogicOp::Xor),
            outputs: &outputs,
            ..Default::default()
        },
    ];
    for parameters in &refused {
        assert!(matches!(
            draw_with_logic_op(&display, &mut target.as_surface(), false, [0; 4], parameters),
            Err(safegl::DrawError::LogicOpWithBlending)
        ));
    }

    display.assert_no_error(None);
}
