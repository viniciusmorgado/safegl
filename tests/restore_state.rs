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

#[test]
fn foreign_code_changing_safegl_objects() {
    use safegl::GlObject;

    let display = support::build_display();

    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = program!(&display,
        330 => {
            vertex: "#version 330\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 330\nuniform sampler2D tex;\nuniform vec4 tint;\nout vec4 f_color;\nvoid main() { f_color = texture(tex, vec2(0.5)) * tint; }",
        },
        300 es => {
            vertex: "#version 300 es\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 300 es\nprecision highp float;\nuniform sampler2D tex;\nuniform vec4 tint;\nout vec4 f_color;\nvoid main() { f_color = texture(tex, vec2(0.5)) * tint; }",
        },
    )
    .unwrap();
    let green = support::build_unicolor_texture2d(&display, 0.0, 1.0, 0.0);
    let target = support::build_renderable_texture(&display);
    let tint = [1.0f32, 1.0, 1.0, 1.0];

    let draw = || {
        target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        target
            .as_surface()
            .draw(&vb, &ib, &program, &uniform! { tex: &green, tint: tint }, &Default::default())
            .unwrap();
        let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
        data[0][0]
    };
    assert_eq!(draw(), (0, 255, 0, 255));

    // a library that calls OpenGL directly while SafeGL's vertex array object is still bound
    unsafe {
        let gl = display.glow_context();
        let index_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        gl.buffer_data_size(glow::ELEMENT_ARRAY_BUFFER, 64, glow::STATIC_DRAW);

        gl.use_program(Some(program.get_id()));
        let tint_location = gl.get_uniform_location(program.get_id(), "tint");
        gl.uniform_4_f32(tint_location.as_ref(), 0.0, 0.0, 0.0, 0.0);

        let black = gl.create_texture().unwrap();
        for unit in 0..4 {
            gl.active_texture(glow::TEXTURE0 + unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(black));
        }
        let sampler = gl.create_sampler().unwrap();
        gl.bind_sampler(0, Some(sampler));
        gl.active_texture(glow::TEXTURE3);
    }
    display.restore_state();

    // the indices, the uniform and the texture of SafeGL are used again
    assert_eq!(draw(), (0, 255, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn indexed_draws_across_frames() {
    // drawing an indexed mesh, then uploading another index buffer, then drawing the mesh again
    let display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);

    for frame in 0..3 {
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        texture
            .as_surface()
            .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
            .unwrap();
        let _other = safegl::IndexBuffer::new(
            &display,
            safegl::index::PrimitiveType::TrianglesList,
            &[frame as u16, 1, 2],
        )
        .unwrap();
        let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
        assert_eq!(data[0][0], (255, 0, 0, 255));
    }

    display.assert_no_error(None);
}

/// The winding of the front faces, the sample coverage and the logic operation go back to what
/// SafeGL set.
#[test]
fn rasterization_state_after_foreign_changes() {
    let display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    let parameters = safegl::DrawParameters {
        multisample: safegl::draw_parameters::MultisampleState {
            sample_coverage: Some(safegl::draw_parameters::SampleCoverage {
                value: 0.25,
                invert: true,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! {}, &parameters)
        .unwrap();

    unsafe {
        display.exec_in_context(|| {
            let gl = display.glow_context();
            gl.front_face(glow::CW);
            gl.sample_coverage(1.0, false);
            gl.disable(glow::SAMPLE_COVERAGE);
            if !support::is_gles(&display) {
                gl.enable(glow::SAMPLE_ALPHA_TO_ONE);
                gl.enable(glow::COLOR_LOGIC_OP);
            }
        });
    }
    display.restore_state();

    let state = unsafe {
        let gl = display.glow_context();
        (
            gl.get_parameter_i32(glow::FRONT_FACE) as u32,
            gl.is_enabled(glow::SAMPLE_COVERAGE),
            gl.get_parameter_f32(glow::SAMPLE_COVERAGE_VALUE),
            gl.get_parameter_i32(glow::SAMPLE_COVERAGE_INVERT) != 0,
            !support::is_gles(&display) && gl.is_enabled(glow::SAMPLE_ALPHA_TO_ONE),
            !support::is_gles(&display) && gl.is_enabled(glow::COLOR_LOGIC_OP),
        )
    };
    assert_eq!(state, (glow::CCW, true, 0.25, true, false, false));
    display.assert_no_error(None);
}
