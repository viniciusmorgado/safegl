//! How the fragment outputs of a program are written to the attachments of a framebuffer
//! (glium#285).

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{MultiOutputFrameBuffer, SimpleFrameBuffer};
use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn outputs_without_locations_follow_their_order() {
    let display = support::build_display();
    if support::is_gles(&display) {
        // OpenGL ES requires a location for each output when there are several
        return;
    }

    // the names sort in the other order than the declarations
    let program = safegl::Program::from_source(
        &display,
        "#version 330\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 330\nout vec4 zebra;\nout vec4 apple;\nvoid main() { zebra = vec4(0.0, 1.0, 0.0, 1.0); apple = vec4(1.0, 0.0, 0.0, 1.0); }",
        None,
    )
    .unwrap();
    assert_eq!(program.get_frag_data_location("zebra"), Some(0));
    assert_eq!(program.get_frag_data_location("apple"), Some(1));

    // the first output goes to a target with a single color buffer
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let texture = support::build_renderable_texture(&display);
    let mut framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    framebuffer
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn explicit_locations_are_kept() {
    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }

    let program = safegl::Program::from_source(
        &display,
        "#version 330\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 330\nout vec4 first;\nlayout(location = 0) out vec4 second;\nvoid main() { first = vec4(1.0); second = vec4(0.5); }",
        None,
    )
    .unwrap();
    assert_eq!(program.get_frag_data_location("second"), Some(0));
    assert_eq!(program.get_frag_data_location("first"), Some(1));

    display.assert_no_error(None);
}

#[test]
fn missing_output_of_multi_output_framebuffer() {
    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = program!(&display,
        330 => {
            vertex: "#version 330\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 330\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
        },
        300 es => {
            vertex: "#version 300 es\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 300 es\nprecision highp float;\nout vec4 color;\nvoid main() { color = vec4(1.0); }",
        },
    )
    .unwrap();

    // the program has no `normal` output
    let color = support::build_renderable_texture(&display);
    let normal = support::build_renderable_texture(&display);
    let mut framebuffer =
        MultiOutputFrameBuffer::new(&display, [("color", &color), ("normal", &normal)]).unwrap();

    let result = framebuffer.draw(&vb, &ib, &program, &uniform! {}, &Default::default());
    assert!(matches!(
        result,
        Err(safegl::DrawError::FragmentOutputMissing { ref name }) if name == "normal"
    ));

    display.assert_no_error(None);
}
