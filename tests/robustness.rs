//! Contexts created with robust buffer access, lost on GPU resets, or without error checking.
//!
//! GPU resets can't be caused on purpose, so these tests check the strategies and the status of
//! contexts that aren't lost. A driver that can't create such a context skips the test.

#[macro_use]
extern crate safegl;

#[cfg(not(target_arch = "wasm32"))]
use safegl::glutin::context::Robustness;
use safegl::CapabilitiesSource;
#[cfg(not(target_arch = "wasm32"))]
use safegl::{ResetNotificationStrategy, Surface};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// A display whose context has `robustness`, or `None` if the driver can't create one.
#[cfg(not(target_arch = "wasm32"))]
fn display_with(robustness: Robustness) -> Option<support::WindowDisplay> {
    let display = support::context_apis().into_iter().find_map(|api| {
        let attributes =
            safegl::backend::glutin::context_attributes_builder(api).with_robustness(robustness);
        support::try_build_display_with(attributes).map(|display| display.unwrap())
    })?;
    // the robustness of a context is only known with OpenGL 4.5 or a robustness extension: a
    // driver without them doesn't make robust contexts, and Mesa makes them anyway when tests
    // hide the extensions
    let extensions = support::extensions(&display);
    let known = support::is_gles(&display)
        || support::gl_at_least(&display, 4, 5)
        || extensions.gl_khr_robustness
        || extensions.gl_arb_robustness;
    known.then_some(display)
}

#[test]
fn context_is_not_lost() {
    let display = support::build_display();
    // out-of-bounds accesses are only defined in robust contexts
    let capabilities = display.get_capabilities();
    assert!(!capabilities.robust_buffer_access || capabilities.robustness);

    assert!(!display.is_context_lost());
    assert_eq!(display.graphics_reset_status(), None);

    display.assert_no_error(None);
}

#[cfg(not(target_arch = "wasm32"))]
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
    assert_eq!(display.graphics_reset_status(), None);

    display.assert_no_error(None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn robust_context_without_reset_notification() {
    let display = match display_with(Robustness::RobustNoResetNotification) {
        Some(display) => display,
        None => return,
    };
    let capabilities = display.get_capabilities();

    assert!(capabilities.robustness);
    // the behavior of out-of-bounds accesses is defined from OpenGL 4.3 or its extensions
    let extensions = support::extensions(&display);
    if support::gl_at_least(&display, 4, 3)
        || extensions.gl_arb_robust_buffer_access_behavior
        || extensions.gl_khr_robust_buffer_access_behavior
    {
        assert!(capabilities.robust_buffer_access);
    }
    assert_eq!(
        capabilities.reset_notification,
        ResetNotificationStrategy::NoResetNotification
    );
    assert!(!display.is_context_loss_possible());
    assert!(!display.is_context_lost());

    display.assert_no_error(None);
}

/// A context without error checking draws like any other: SafeGL doesn't rely on OpenGL errors.
#[cfg(not(target_arch = "wasm32"))]
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

/// Contexts flush their commands when another context is made current, unless they are created
/// to do nothing; the query is core in OpenGL 4.5.
#[test]
fn contexts_flush_when_released() {
    let display = support::build_display();
    assert_eq!(
        display.get_capabilities().release_behavior,
        safegl::ReleaseBehavior::Flush
    );
    display.assert_no_error(None);
}
