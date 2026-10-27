//! Sync fences waited with a timeout and on the GPU, and the timestamp of the GPU.

#[macro_use]
extern crate safegl;

use std::time::Duration;

use safegl::{Surface, SyncFence, SyncWaitResult};

mod support;

#[test]
fn wait_with_a_timeout() {
    let display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();

    let fence = SyncFence::new(&display);
    // the draw may or may not be finished, but the wait doesn't fail
    assert_ne!(fence.wait_timeout(Duration::ZERO), SyncWaitResult::Failed);
    assert_eq!(
        fence.wait_timeout(Duration::from_secs(10)),
        SyncWaitResult::Signaled
    );
    assert!(fence.is_signaled());
    // the fence stays usable
    assert_eq!(fence.wait_timeout(Duration::MAX), SyncWaitResult::Signaled);
    fence.wait();

    display.assert_no_error(None);
}

#[test]
fn wait_on_the_gpu() {
    let display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    let fence = SyncFence::new(&display);
    fence.wait_on_gpu();
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (255, 0, 0, 255));
    fence.wait();

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn gpu_timestamps_increase() {
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    let first = match display.gpu_timestamp() {
        Some(time) => time,
        None => {
            assert!(support::is_gles(&display));
            assert!(!display.get_extensions().gl_ext_disjoint_timer_query);
            return;
        }
    };

    // a timestamp query runs after the first timestamp, on the same clock
    let query = safegl::debug::TimestampQuery::new(&display).unwrap();
    display.finish();
    let second = display.gpu_timestamp().unwrap();
    let queried = query.get().unwrap();
    assert!(first <= second);
    assert!(first <= queried && queried <= second);

    display.assert_no_error(None);
}
