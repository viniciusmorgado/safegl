//! Rasterization states that desktop OpenGL has and that WebGL has through extensions: polygon
//! modes and clip distances.
#![cfg(any(safegl_desktop, safegl_webgl))]

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::PolygonMode;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::{DrawError, DrawParameters, Program, Surface, Texture2d, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// Draws a triangle over most of an 8 × 8 target with `program` and `parameters`, and returns the
/// pixels, or the error of the draw.
fn draw(
    display: &support::Backend,
    program: &Program,
    parameters: &DrawParameters<'_>,
) -> Result<Vec<Vec<(u8, u8, u8, u8)>>, DrawError> {
    let positions = [[-0.875, -0.875], [0.875, -0.875], [-0.875, 0.875]];
    let triangle = VertexBuffer::new(display, &positions.map(|position| Vertex { position }))
        .unwrap();
    let target = Texture2d::empty(display, 8, 8).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    target.as_surface().draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        &uniform! {},
        parameters,
    )?;
    Ok(target.read().unwrap())
}

/// The polygon modes draw the edges of the triangles (`Line`), on desktop OpenGL and on WebGL with
/// `WEBGL_polygon_mode`, and their vertices (`Point`) on desktop OpenGL.
#[test]
fn polygon_modes() {
    let display = support::build_display();
    let (_, _, program) = support::build_fullscreen_red_pipeline(&display);
    let mode = |polygon_mode| {
        draw(
            &display,
            &program,
            &DrawParameters {
                polygon_mode,
                ..Default::default()
            },
        )
    };

    // filled, the inside of the triangle is drawn
    let filled = mode(PolygonMode::Fill).unwrap();
    assert_eq!(filled[2][2], (255, 0, 0, 255));

    let desktop = !support::is_gles(&display);
    match mode(PolygonMode::Line) {
        Ok(lines) => {
            assert!(desktop || support::extensions(&display).webgl_polygon_mode);
            // the inside is empty, an edge is drawn
            assert_eq!(lines[2][2], (0, 0, 0, 0));
            assert!(lines[0].iter().any(|&pixel| pixel == (255, 0, 0, 255)));
        }
        Err(DrawError::PolygonModeNotSupported) => {
            assert!(!desktop && !support::extensions(&display).webgl_polygon_mode)
        }
        Err(error) => panic!("{error}"),
    }
    match mode(PolygonMode::Point) {
        Ok(_) => assert!(desktop),
        Err(DrawError::PolygonModeNotSupported) => assert!(!desktop),
        Err(error) => panic!("{error}"),
    }
    // filled again
    assert_eq!(mode(PolygonMode::Fill).unwrap()[2][2], (255, 0, 0, 255));

    display.assert_no_error(None);
}

/// A clip distance clips the half of the triangle where it is negative, on desktop OpenGL, on
/// OpenGL ES with `GL_EXT_clip_cull_distance` and on WebGL with `WEBGL_clip_cull_distance`.
#[test]
fn clip_distances() {
    let display = support::build_display();
    let extensions = support::extensions(&display);
    let header = if !support::is_gles(&display) {
        "#version 330\n"
    } else if extensions.webgl_clip_cull_distance {
        "#version 300 es\n#extension GL_ANGLE_clip_cull_distance : require\n"
    } else if extensions.gl_ext_clip_cull_distance {
        "#version 300 es\n#extension GL_EXT_clip_cull_distance : require\n"
    } else {
        // the plane can't be enabled
        let (_, _, program) = support::build_fullscreen_red_pipeline(&display);
        let parameters = DrawParameters {
            clip_planes_bitmask: 1,
            ..Default::default()
        };
        assert!(matches!(
            draw(&display, &program, &parameters),
            Err(DrawError::ClipPlaneIndexOutOfBounds)
        ));
        return;
    };
    let program = Program::from_source(
        &display,
        &format!(
            "{header}in vec2 position;\n\
             void main() {{ gl_ClipDistance[0] = position.x; \
             gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        // GLSL 3.30 accepts the precisions of GLSL ES
        &format!(
            "{header}precision mediump float;\nout vec4 color;\n\
             void main() {{ color = vec4(1.0, 0.0, 0.0, 1.0); }}"
        ),
        None,
    )
    .unwrap();

    let parameters = DrawParameters {
        clip_planes_bitmask: 1,
        ..Default::default()
    };
    let clipped = draw(&display, &program, &parameters).unwrap();
    // the left of the triangle, where x < 0, is clipped; the right isn't
    assert_eq!(clipped[2][1], (0, 0, 0, 0));
    assert_eq!(clipped[1][5], (255, 0, 0, 255));
    // without the plane, the whole triangle is drawn; llvmpipe (Mesa 26) clips with the written
    // distances on desktop OpenGL even when the plane is disabled, so only the disabled state is
    // checked there
    let whole = draw(&display, &program, &Default::default()).unwrap();
    assert!(!unsafe {
        use safegl::glow::HasContext;
        display.glow_context().is_enabled(safegl::glow::CLIP_DISTANCE0)
    });
    let llvmpipe_desktop =
        !support::is_gles(&display) && display.get_opengl_renderer_string().contains("llvmpipe");
    if !llvmpipe_desktop {
        assert_eq!(whole[2][1], (255, 0, 0, 255));
    }

    display.assert_no_error(None);
}
