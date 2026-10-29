//! The displays of the tests without window system: pbuffers of an EGL device, the one whose
//! name, vendor or DRM node contains `SAFEGL_EGL_DEVICE` (`software` for the software renderer
//! of Mesa), or else the first one that creates a context of the selected levels.

use super::{parse_robustness, parse_version};
use safegl::backend::glutin::headless::{devices, Headless};
use safegl::backend::{Context, Facade};
use safegl::Display;

use glutin::prelude::*;
use glutin::surface::PbufferSurface;

use std::env;
use std::ops::Deref;
use std::rc::Rc;

/// The size of the pbuffers, the one of the windows of the windowed tests.
const SIZE: (u32, u32) = (800, 600);

/// The EGL display of the tests.
fn headless() -> Headless {
    let Ok(wanted) = env::var("SAFEGL_EGL_DEVICE") else {
        return Headless::first().expect("no EGL device creates a context for the tests");
    };
    let device = devices()
        .expect("the EGL devices can't be listed")
        .into_iter()
        .find(|device| {
            if wanted == "software" {
                return device.extensions().contains("EGL_MESA_device_software");
            }
            let path = device.drm_device_node_path().map(|path| path.display().to_string());
            [device.name().map(str::to_owned), device.vendor().map(str::to_owned), path]
                .into_iter()
                .flatten()
                .any(|text| text.contains(&wanted))
        })
        .unwrap_or_else(|| panic!("no EGL device matches SAFEGL_EGL_DEVICE={wanted}"));
    Headless::new(&device).expect("the EGL device can't draw for the tests")
}

/// The context attributes of the tests for `api`, with the robustness of `SAFEGL_ROBUSTNESS`.
fn attributes(api: glutin::context::ContextApi) -> glutin::context::ContextAttributesBuilder {
    let attributes = safegl::backend::glutin::context_attributes_builder(api);
    match parse_robustness() {
        Some(robustness) => attributes.with_robustness(robustness),
        None => attributes,
    }
}

/// The display of a test.
pub struct WindowDisplay {
    display: Display<PbufferSurface>,
}

impl Deref for WindowDisplay {
    type Target = Display<PbufferSurface>;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl Facade for WindowDisplay {
    fn get_context(&self) -> &Rc<Context> {
        self.display.get_context()
    }
}

/// Builds a display for tests.
pub fn build_display() -> WindowDisplay {
    build_display_with_debug(Default::default())
}

/// Builds a display for tests, with the given debug callback.
pub fn build_display_with_debug(debug: safegl::debug::DebugCallbackBehavior) -> WindowDisplay {
    let headless = headless();
    let context = parse_version()
        .into_iter()
        .find_map(|api| headless.create_context(attributes(api)).ok())
        .expect("failed to create a context");
    let surface = headless.create_pbuffer(SIZE.0, SIZE.1).unwrap();
    let context = context.make_current(&surface).unwrap();
    WindowDisplay {
        display: Display::from_pbuffer(context, surface, debug).unwrap(),
    }
}

/// Builds a display whose context is created with the given attributes, or returns `None` if
/// the driver can't create such a context.
pub fn try_build_display_with(
    attributes: glutin::context::ContextAttributesBuilder,
) -> Option<Result<WindowDisplay, safegl::IncompatibleOpenGl>> {
    let headless = headless();
    let context = headless.create_context(attributes).ok()?;
    let surface = headless.create_pbuffer(SIZE.0, SIZE.1).unwrap();
    let context = context.make_current(&surface).unwrap();
    Some(
        Display::from_pbuffer(context, surface, Default::default())
            .map(|display| WindowDisplay { display }),
    )
}

/// Moves an existing display to a new pbuffer, with a new context that shares its objects.
pub fn rebuild_display(display: &mut WindowDisplay) {
    try_rebuild_display(display, parse_version())
        .expect("failed to create context")
        .unwrap();
}

/// Same as `rebuild_display`, with a context of the first of `apis` that the driver creates.
/// Returns `None` if it creates none, otherwise the result of `Display::rebuild`.
pub fn try_rebuild_display(
    display: &mut WindowDisplay,
    apis: Vec<glutin::context::ContextApi>,
) -> Option<Result<(), safegl::IncompatibleOpenGl>> {
    let headless = headless();
    let context = apis.into_iter().find_map(|api| {
        let attributes = attributes(api).with_sharing(&*display.display.gl_context());
        headless.create_context(attributes).ok()
    })?;
    let surface = headless.create_pbuffer(SIZE.0, SIZE.1).unwrap();
    let context = context.make_current(&surface).unwrap();
    Some(unsafe { display.display.rebuild(context, surface) })
}

/// The display type that the helpers of the tests take.
pub type Backend = Display<PbufferSurface>;

/// The display of a test.
pub type TestDisplay = WindowDisplay;
