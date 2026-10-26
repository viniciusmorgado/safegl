#[macro_use]
extern crate safegl;

use glow::HasContext;
use safegl::Surface;

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

/// Changes the OpenGL state behind SafeGL's back, the way another library sharing the context
/// would: blending that outputs black, a closed scissor box, culling of every face, a masked color
/// output, another vertex array, no program and a bound pixel unpack buffer.
fn sabotage(display: &Display) {
    unsafe {
        display.exec_in_context(|| {
            let gl = display.glow_context();
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ZERO, glow::ZERO);
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(0, 0, 0, 0);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::FRONT_AND_BACK);
            gl.color_mask(false, false, false, false);
            gl.depth_func(glow::NEVER);
            gl.use_program(None);
            let vertex_array = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vertex_array));
            let buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, Some(buffer));
            gl.buffer_data_size(glow::PIXEL_UNPACK_BUFFER, 64, glow::STATIC_DRAW);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 8);
        });
    }
}

fn draw_red(display: &Display) -> (u8, u8, u8, u8) {
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(display);
    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    sabotage(display);
    display.restore_state();

    texture
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[0][0]
}

#[test]
fn draw_after_foreign_state_changes() {
    let display = support::build_display();
    assert_eq!(draw_red(&display), (255, 0, 0, 255));
    display.assert_no_error(None);
}

#[test]
fn upload_after_foreign_state_changes() {
    let display = support::build_display();

    // an upload would read from the foreign pixel unpack buffer if it were still bound
    let texture = safegl::Texture2d::empty(&display, 2, 2).unwrap();
    sabotage(&display);
    display.restore_state();
    texture.write(
        safegl::Rect {
            left: 0,
            bottom: 0,
            width: 2,
            height: 2,
        },
        vec![vec![(1u8, 2u8, 3u8, 4u8); 2]; 2],
    ).unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[1][1], (1, 2, 3, 4));
    display.assert_no_error(None);
}
