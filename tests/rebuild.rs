//! Moving a display to another window and OpenGL context with `Display::rebuild`.

#[macro_use]
extern crate safegl;

use std::cell::RefCell;
use std::rc::Rc;

use safegl::framebuffer::SimpleFrameBuffer;
use safegl::glutin::context::{ContextApi, Version};
use safegl::Surface;

mod support;

/// Draws the red pipeline of the support module into a new texture, through a framebuffer
/// object, and returns its first pixel.
fn draw_red(display: &support::WindowDisplay) -> (u8, u8, u8, u8) {
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(display);
    let texture = support::build_renderable_texture(display);
    let mut framebuffer = SimpleFrameBuffer::new(display, &texture).unwrap();
    framebuffer.clear_color(0.0, 0.0, 0.0, 0.0);
    framebuffer
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[0][0]
}

#[test]
fn objects_keep_working() {
    let mut display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    let mut framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    framebuffer
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();
    drop(framebuffer);

    support::rebuild_display(&mut display);

    // the buffers, the program and the texture are shared with the new context, while its
    // framebuffer objects and vertex array objects are created again
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (255, 0, 0, 255));
    assert_eq!(draw_red(&display), (255, 0, 0, 255));

    // and the new window can be drawn to
    let mut frame = display.draw();
    frame.clear_color(0.0, 0.0, 1.0, 1.0);
    frame
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();
    frame.finish().unwrap();

    display.assert_no_error(None);
}

#[test]
fn debug_callback_follows_the_context() {
    if !support::contexts_check_errors() {
        return;
    }
    let errors = Rc::new(RefCell::new(Vec::new()));
    let recorded = errors.clone();
    let mut display =
        support::build_display_with_debug(safegl::debug::DebugCallbackBehavior::Custom {
            callback: Box::new(move |_, ty, _, _, _, message| {
                if matches!(ty, safegl::debug::MessageType::Error) {
                    recorded.borrow_mut().push(message.to_owned());
                }
            }),
            synchronous: true,
        });

    support::rebuild_display(&mut display);

    // an invalid enum raises an error, reported by the debug output of the new context
    let error = unsafe {
        display.exec_in_context(|| {
            use safegl::glow::HasContext;
            display.glow_context().enable(0x1234_5678);
            display.glow_context().get_error()
        })
    };
    assert_eq!(error, safegl::glow::INVALID_ENUM);
    let has_debug_output = display.get_opengl_version() >= &safegl::Version(safegl::Api::Gl, 4, 3)
        || display.get_opengl_version() >= &safegl::Version(safegl::Api::GlEs, 3, 2);
    if has_debug_output {
        assert_eq!(errors.borrow().len(), 1);
    }

    display.assert_no_error(None);
}

#[test]
fn incompatible_context() {
    let mut display = support::build_display();
    if support::is_gles(&display) {
        return;
    }

    // another version than the one of the display
    let version = *display.get_opengl_version();
    let other = if version >= safegl::Version(safegl::Api::Gl, 4, 6) {
        Version::new(4, 1)
    } else {
        Version::new(4, 6)
    };
    let rebuilt = support::try_rebuild_display(&mut display, vec![ContextApi::OpenGl(Some(other))]);
    if rebuilt.is_none() {
        // the driver doesn't share objects between these versions
        return;
    }

    // the context in use has the version of the first one: the previous context if the new one
    // isn't compatible, otherwise the new one, since some drivers give their highest version for
    // any version they have
    let current_version = unsafe {
        display.exec_in_context(|| {
            use safegl::glow::HasContext;
            display
                .glow_context()
                .get_parameter_string(safegl::glow::VERSION)
        })
    };
    assert_eq!(current_version, display.get_opengl_version_string());

    // and it keeps working
    assert_eq!(draw_red(&display), (255, 0, 0, 255));
    let mut frame = display.draw();
    frame.clear_color(0.0, 0.0, 1.0, 1.0);
    frame.finish().unwrap();

    display.assert_no_error(None);
}
