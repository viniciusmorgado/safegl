//! Contexts created with robust buffer access, lost on GPU resets, or without error checking.
//!
//! GPU resets can't be caused on purpose, so these tests check the strategies and the status of
//! contexts that aren't lost. A driver that can't create such a context skips the test.

#[macro_use]
extern crate safegl;

use safegl::glutin::context::Robustness;
use safegl::{CapabilitiesSource, ResetNotificationStrategy, Surface};

mod support;

/// A display whose context has `robustness`, or `None` if the driver can't create one.
fn display_with(robustness: Robustness) -> Option<support::WindowDisplay> {
    support::context_apis().into_iter().find_map(|api| {
        let attributes =
            safegl::backend::glutin::context_attributes_builder(api).with_robustness(robustness);
        support::try_build_display_with(attributes).map(|display| display.unwrap())
    })
}

#[test]
fn context_is_not_lost() {
    let display = support::build_display();

    assert!(!display.is_context_lost());
    #[cfg(any(safegl_gl45, safegl_gles32))]
    assert_eq!(display.graphics_reset_status(), None);

    display.assert_no_error(None);
}

#[test]
fn robust_context_lost_on_reset() {
    let display = match display_with(Robustness::RobustLoseContextOnReset) {
        Some(display) => display,
        None => return,
    };
    let capabilities = display.get_capabilities();

    assert!(capabilities.robustness);
    assert_eq!(
        capabilities.reset_notification,
        ResetNotificationStrategy::LoseContextOnReset
    );
    assert!(display.is_context_loss_possible());
    assert!(!display.is_context_lost());
    #[cfg(any(safegl_gl45, safegl_gles32))]
    assert_eq!(display.graphics_reset_status(), None);

    display.assert_no_error(None);
}

#[test]
fn robust_context_without_reset_notification() {
    let display = match display_with(Robustness::RobustNoResetNotification) {
        Some(display) => display,
        None => return,
    };
    let capabilities = display.get_capabilities();

    assert!(capabilities.robustness);
    assert_eq!(
        capabilities.reset_notification,
        ResetNotificationStrategy::NoResetNotification
    );
    assert!(!display.is_context_loss_possible());
    assert!(!display.is_context_lost());

    display.assert_no_error(None);
}

/// A context without error checking draws like any other: SafeGL doesn't rely on OpenGL errors.
#[test]
fn context_without_error_checking_draws() {
    let display = match display_with(Robustness::NoError) {
        Some(display) => display,
        None => return,
    };

    #[cfg(safegl_gl46)]
    {
        use safegl::{Api, Version};
        let no_error = display.get_opengl_version() >= &Version(Api::Gl, 4, 6)
            || display.get_extensions().gl_khr_no_error;
        assert_eq!(display.is_no_error(), no_error);
    }

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }
}
