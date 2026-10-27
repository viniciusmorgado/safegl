#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

#[test]
fn get_opengl_version() {
    let display = support::build_display();
    let version = display.get_opengl_version();
    display.assert_no_error(None);

    assert!(version.1 >= 1);
}

#[test]
fn clear_color() {
    let display = support::build_display();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();

    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn clear_color_rect() {
    let display = support::build_display();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);

    let rect = safegl::Rect {
        left: 512,
        bottom: 0,
        width: 512,
        height: 1024,
    };
    texture
        .as_surface()
        .clear(Some(&rect), Some((0.0, 1.0, 0.0, 1.0)), false, None, None);

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();

    for row in data.iter() {
        for (col, pixel) in row.iter().enumerate() {
            if col >= 512 {
                assert_eq!(pixel, &(0, 255, 0, 255));
            } else {
                assert_eq!(pixel, &(255, 0, 0, 255));
            }
        }
    }

    display.assert_no_error(None);
}

#[test]
fn release_shader_compiler() {
    let display = support::build_display();
    display.release_shader_compiler();
    display.assert_no_error(None);
}

#[test]
fn timestamp_query() {
    let display = support::build_display();

    let query1 = safegl::debug::TimestampQuery::new(&display);
    let query1 = query1.map(|q| q.get().unwrap());

    let query2 = safegl::debug::TimestampQuery::new(&display);
    let query2 = query2.map(|q| q.get().unwrap());

    match (query1, query2) {
        (Some(q1), Some(q2)) => assert!(q2 >= q1 && q1 != 0),
        _ => (),
    };

    display.assert_no_error(None);
}

#[test]
fn sync() {
    let display = support::build_display();

    safegl::SyncFence::new(&display).wait();

    display.assert_no_error(None);
}

#[test]
fn multiple_displays() {
    let display1 = support::build_display();
    let display2 = support::build_display();

    let (vb1, ib1, program1) = support::build_fullscreen_red_pipeline(&display1);
    let (vb2, ib2, program2) = support::build_fullscreen_red_pipeline(&display2);

    let texture1 = support::build_renderable_texture(&display1);
    let texture2 = support::build_renderable_texture(&display2);

    texture1.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture2.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    texture1
        .as_surface()
        .draw(
            &vb1,
            &ib1,
            &program1,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    texture2
        .as_surface()
        .draw(
            &vb2,
            &ib2,
            &program2,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();
    texture2.as_surface().clear_color(0.0, 1.0, 0.0, 0.0);

    let read_back: Vec<Vec<(u8, u8, u8, u8)>> = texture1.read().unwrap();
    for row in read_back.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    let read_back: Vec<Vec<(u8, u8, u8, u8)>> = texture2.read().unwrap();
    for row in read_back.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(0, 255, 0, 0));
        }
    }

    display1.assert_no_error(None);
    display2.assert_no_error(None);
}

#[test]
fn debug_string() {
    // tests that `insert_debug_marker` doesn't trigger an OpenGL error
    let display = support::build_display();
    display.insert_debug_marker("Hello world").ok();
    display.assert_no_error(None);
}

#[test]
fn is_context_lost() {
    // tests that `is_context_lost` doesn't trigger an OpenGL error
    let display = support::build_display();
    display.is_context_lost();
    display.assert_no_error(None);
}

#[test]
fn one_frame_at_a_time() {
    let display = support::build_display();

    // a second frame would draw to the same back buffer and swap it again
    let frame = display.draw();
    assert_eq!(
        display.try_draw().err(),
        Some(safegl::FrameError::AlreadyDrawing)
    );
    frame.finish().unwrap();

    // once finished, a new frame can start, and a frame dropped during a panic ends too
    display.try_draw().unwrap().finish().unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _frame = display.draw();
        panic!("dropping the frame");
    }));
    display.draw().finish().unwrap();

    display.assert_no_error(None);
}

#[test]
fn compatibility_profile_is_refused() {
    use safegl::glutin::context::{ContextApi, ContextAttributesBuilder, GlProfile, Version};

    let attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 0))))
        .with_profile(GlProfile::Compatibility);
    let result = match support::try_build_display_with(attributes) {
        Some(result) => result,
        // the driver has no compatibility profile
        None => return,
    };

    let error = result.err().expect("a compatibility profile context was accepted");
    assert!(error.to_string().contains("core profile"), "{error}");
}

#[test]
fn core_profile_is_accepted() {
    use safegl::glutin::context::{ContextApi, Version};

    let attributes = safegl::backend::glutin::context_attributes_builder(ContextApi::OpenGl(
        Some(Version::new(4, 0)),
    ));
    if let Some(result) = support::try_build_display_with(attributes) {
        let display = result.unwrap();
        assert!(*display.get_opengl_version() >= safegl::Version(safegl::Api::Gl, 4, 0));
    }
}
