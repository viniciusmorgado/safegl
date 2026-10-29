//! Displays without window system, and displays that lose their surface and get another one, as
//! on Android.

#![cfg(not(target_arch = "wasm32"))]

#[macro_use]
extern crate safegl;

use safegl::backend::glutin::headless::{devices, Headless, HeadlessError};
use safegl::backend::glutin::SurfaceRemovalError;
use safegl::{FrameError, Surface, SwapBuffersError, Texture2d};

mod support;

type Pixel = (u8, u8, u8, u8);

/// Held by each test: NVIDIA's driver fails to make an EGL context current while a GLX context
/// of the windows of the other tests is made current on another thread.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A display without window draws its frames to its pbuffer, which is read back.
#[test]
fn headless_display() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let headless = match Headless::first() {
        Ok(headless) => headless,
        // EGL without devices, like the drivers of other systems
        Err(HeadlessError::NoDevice) => return,
        Err(error) => panic!("{error}"),
    };
    assert!(!devices().unwrap().is_empty());
    let display = headless.display(64, 32, Default::default()).unwrap();
    assert_eq!(display.get_framebuffer_dimensions(), (64, 32));

    let mut frame = display.draw();
    frame.clear_color(0.0, 0.0, 1.0, 1.0);
    frame.finish().unwrap();
    let pixels: Vec<Vec<Pixel>> = display.read_front_buffer().unwrap();
    assert_eq!((pixels.len(), pixels[0].len()), (32, 64));
    assert!(pixels.iter().flatten().all(|&pixel| pixel == (0, 0, 255, 255)));

    display.assert_no_error(None);
}

/// The display of a test loses its surface, as Android destroys the window of a paused
/// application, keeps its objects, and gets its surface back.
#[test]
fn surface_removed_and_given_back() {
    let _lock = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let display = support::build_display();
    let texture = Texture2d::empty(&display, 4, 4).unwrap();
    let surface = match display.remove_surface() {
        Ok(surface) => surface,
        // GLX and WGL contexts, and EGL displays without surfaceless contexts
        Err(SurfaceRemovalError::NotSupported) => {
            assert!(display.has_surface());
            display.assert_no_error(None);
            return;
        }
        Err(error) => panic!("{error}"),
    };
    assert!(!display.has_surface());
    assert_eq!(
        display.remove_surface().err(),
        Some(SurfaceRemovalError::NoSurface)
    );
    assert_eq!(display.try_draw().err(), Some(FrameError::SurfaceLost));

    // the objects live on, and framebuffers are drawn to
    texture.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);
    let other = Texture2d::empty(&display, 2, 2).unwrap();
    other.as_surface().clear_color(0.0, 1.0, 0.0, 1.0);
    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    assert!(pixels.iter().flatten().all(|&pixel| pixel == (255, 0, 0, 255)));
    display.assert_no_error(None);

    // the surface is given back, and frames are drawn again
    assert!(display.set_surface(surface).unwrap().is_none());
    assert!(display.has_surface());
    let mut frame = display.draw();
    frame.clear_color(0.0, 0.0, 1.0, 1.0);
    frame.finish().unwrap();

    // a frame whose surface is removed isn't shown
    let frame = display.draw();
    let surface = display.remove_surface().unwrap();
    assert_eq!(frame.finish(), Err(SwapBuffersError::SurfaceLost));
    display.set_surface(surface).unwrap();
    display.assert_no_error(None);
}
