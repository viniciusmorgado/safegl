//! Indexed draws that start new primitives at a fixed or chosen index.

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::PrimitiveRestart;
use safegl::index::{Index, PrimitiveType};
use safegl::{DrawError, IndexBuffer, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// Draws a line strip through a lower and an upper horizontal line with `indices`, and returns
/// whether the strip connected them: its diagonal crosses the center of the target.
fn connects<I: Index>(
    display: &Display,
    indices: &[I],
    restart: PrimitiveRestart,
) -> Result<bool, DrawError> {
    let positions = [[-0.5, -0.5], [0.5, -0.5], [-0.5, 0.5], [0.5, 0.5]];
    // the vertex at index 7 is the end of the lower line too, for restarts at a chosen index
    let mut vertices = positions.map(|position| Vertex { position }).to_vec();
    vertices.resize(
        8,
        Vertex {
            position: [0.5, -0.5],
        },
    );
    let vertices = VertexBuffer::new(display, &vertices).unwrap();
    let indices = IndexBuffer::new(display, PrimitiveType::LineStrip, indices).unwrap();
    let program = support::program_from_legacy(
        display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0); }",
    )
    .unwrap();

    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(1.0, 1.0, 1.0, 1.0);
    let parameters = safegl::DrawParameters {
        primitive_restart: restart,
        ..Default::default()
    };
    texture
        .as_surface()
        .draw(&vertices, &indices, &program, &uniform! {}, &parameters)?;

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    let (width, height) = (data[0].len(), data.len());
    let center = (height / 2 - 2..height / 2 + 2)
        .flat_map(|y| (width / 2 - 2..width / 2 + 2).map(move |x| (x, y)));
    let mut colors = center.map(|(x, y)| data[y][x]);
    Ok(colors.any(|color| color != (255, 255, 255, 255)))
}

#[test]
fn fixed_index_of_each_type() {
    let display = support::build_display();

    assert!(!connects(
        &display,
        &[0u8, 1, u8::MAX, 2, 3],
        PrimitiveRestart::FixedIndex
    )
    .unwrap());
    assert!(!connects(
        &display,
        &[0u16, 1, u16::MAX, 2, 3],
        PrimitiveRestart::FixedIndex
    )
    .unwrap());
    assert!(!connects(
        &display,
        &[0u32, 1, u32::MAX, 2, 3],
        PrimitiveRestart::FixedIndex
    )
    .unwrap());

    display.assert_no_error(None);
}

#[test]
fn disabled_restart_connects() {
    let display = support::build_display();

    assert!(connects(&display, &[0u16, 1, 2, 3], PrimitiveRestart::Disabled).unwrap());
    // switching between the modes
    assert!(!connects(
        &display,
        &[0u16, 1, u16::MAX, 2, 3],
        PrimitiveRestart::FixedIndex
    )
    .unwrap());
    assert!(connects(&display, &[0u16, 1, 2, 3], PrimitiveRestart::Disabled).unwrap());

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn chosen_index() {
    let display = support::build_display();

    match connects(&display, &[0u16, 1, 7, 2, 3], PrimitiveRestart::Index(7)) {
        Ok(connected) => assert!(!connected),
        Err(DrawError::PrimitiveRestartIndexNotSupported) => {
            assert!(support::is_gles(&display));
            return;
        }
        Err(error) => panic!("{error}"),
    }
    // index 7 is an ordinary vertex again, which joins the lines through the lower right
    assert!(connects(&display, &[0u16, 1, 7, 2, 3], PrimitiveRestart::Disabled).unwrap());
    assert!(!connects(
        &display,
        &[0u8, 1, u8::MAX, 2, 3],
        PrimitiveRestart::FixedIndex
    )
    .unwrap());
    assert!(!connects(&display, &[0u32, 1, 7, 2, 3], PrimitiveRestart::Index(7)).unwrap());

    display.assert_no_error(None);
}

/// Primitive restart enabled behind SafeGL's back is disabled by `restore_state`.
#[cfg(safegl_desktop)]
#[test]
fn restart_after_foreign_state_changes() {
    use safegl::glow::HasContext;
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }

    unsafe {
        display.exec_in_context(|| {
            let gl = display.glow_context();
            gl.enable(safegl::glow::PRIMITIVE_RESTART);
            if display.get_opengl_version() >= &safegl::Version(safegl::Api::Gl, 4, 3)
                || display.get_extensions().gl_arb_es3_compatibility
            {
                gl.enable(safegl::glow::PRIMITIVE_RESTART_FIXED_INDEX);
            }
        });
    }
    display.restore_state();

    // the strip goes through vertex 0, the foreign restart index, on its diagonal
    assert!(connects(&display, &[3u16, 0, 1], PrimitiveRestart::Disabled).unwrap());

    display.assert_no_error(None);
}
