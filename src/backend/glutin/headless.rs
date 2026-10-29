/*!

Displays without window: an EGL display of a device (`EGL_EXT_platform_device`), which needs
neither X11 nor Wayland, and draws its frames to a pbuffer of a fixed size. They run on servers,
in CI, and on embedded boards without a window system, where the content of the frames is read
back or written to images.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use safegl::backend::glutin::headless::Headless;
use safegl::Surface;

let headless = Headless::first()?;
let display = headless.display(256, 256, Default::default())?;
let mut frame = display.draw();
frame.clear_color(0.0, 0.0, 1.0, 1.0);
frame.finish()?;
let pixels: Vec<Vec<(u8, u8, u8, u8)>> = display.read_front_buffer()?;
# Ok(())
# }
```

*/

use std::error::Error;
use std::fmt;
use std::num::NonZeroU32;

use glutin::api::egl::device::Device;
use glutin::config::{Api, Config, ConfigSurfaceTypes, ConfigTemplateBuilder};
use glutin::context::{ContextAttributesBuilder, NotCurrentContext};
use glutin::display::{GetDisplayExtensions, GlDisplay};
use glutin::prelude::*;
use glutin::surface::{PbufferSurface, Surface, SurfaceAttributesBuilder};

use super::{context_attributes_builder, level_context_apis, Display};
use crate::debug::DebugCallbackBehavior;
use crate::IncompatibleOpenGl;

/// Error that can happen while creating a display without window.
#[derive(Debug)]
#[non_exhaustive]
pub enum HeadlessError {
    /// glutin or EGL returned an error.
    Glutin(glutin::error::Error),
    /// No EGL device can draw for SafeGL: there is none, or none creates a context of the
    /// selected levels.
    NoDevice,
    /// The device has no configuration of pbuffers with 8 bits for each color, the alpha
    /// included.
    NoConfig,
    /// The driver can't create a context of the selected levels.
    NoContext,
    /// The OpenGL implementation is too old.
    IncompatibleOpenGl(IncompatibleOpenGl),
}

impl fmt::Display for HeadlessError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeadlessError::Glutin(error) => write!(fmt, "{error}"),
            HeadlessError::NoDevice => fmt.write_str("No EGL device creates a context for SafeGL"),
            HeadlessError::NoConfig => {
                fmt.write_str("The EGL device has no configuration of pbuffers for SafeGL")
            }
            HeadlessError::NoContext => {
                fmt.write_str("The EGL device can't create a context of the selected levels")
            }
            HeadlessError::IncompatibleOpenGl(error) => write!(fmt, "{error}"),
        }
    }
}

impl Error for HeadlessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HeadlessError::Glutin(error) => Some(error),
            HeadlessError::IncompatibleOpenGl(error) => Some(error),
            _ => None,
        }
    }
}

impl From<glutin::error::Error> for HeadlessError {
    fn from(error: glutin::error::Error) -> HeadlessError {
        HeadlessError::Glutin(error)
    }
}

impl From<IncompatibleOpenGl> for HeadlessError {
    fn from(error: IncompatibleOpenGl) -> HeadlessError {
        HeadlessError::IncompatibleOpenGl(error)
    }
}

/// Returns the EGL devices of the system (`EGL_EXT_device_enumeration`): the GPUs, and the
/// software renderer of Mesa (`EGL_MESA_device_software`). `Device::name`, `Device::vendor`,
/// `Device::extensions` and `Device::drm_device_node_path` tell them apart.
pub fn devices() -> Result<Vec<Device>, HeadlessError> {
    Ok(Device::query_devices()?.collect())
}

/// The EGL display of a device, and the configuration of its pbuffers, from which displays
/// without window are created.
pub struct Headless {
    display: glutin::display::Display,
    config: Config,
}

impl Headless {
    /// The EGL display of `device`, with a configuration of pbuffers that have 8 bits for each
    /// color and the alpha, and depth and stencil buffers when the device has them.
    pub fn new(device: &Device) -> Result<Headless, HeadlessError> {
        let display = unsafe { glutin::api::egl::display::Display::with_device(device, None)? };
        let display = glutin::display::Display::Egl(display);
        let apis = level_context_apis()
            .iter()
            .fold(Api::empty(), |apis, api| match api {
                glutin::context::ContextApi::OpenGl(_) => apis | Api::OPENGL,
                glutin::context::ContextApi::Gles(_) => apis | Api::GLES3,
            });
        let template = ConfigTemplateBuilder::new()
            .with_surface_type(ConfigSurfaceTypes::PBUFFER)
            .with_api(apis)
            .with_alpha_size(8)
            .build();
        // the configuration with the most depth and stencil bits
        let config = unsafe { display.find_configs(template)? }
            .filter(|config| config.color_buffer_type().is_some())
            .max_by_key(|config| (config.depth_size(), config.stencil_size()))
            .ok_or(HeadlessError::NoConfig)?;
        Ok(Headless { display, config })
    }

    /// The EGL display of the first device that creates a context of the selected levels.
    pub fn first() -> Result<Headless, HeadlessError> {
        devices()?
            .iter()
            .filter_map(|device| Headless::new(device).ok())
            .find(|headless| {
                level_context_apis().into_iter().any(|api| {
                    headless
                        .create_context(context_attributes_builder(api))
                        .is_ok()
                })
            })
            .ok_or(HeadlessError::NoDevice)
    }

    /// The glutin display.
    pub fn gl_display(&self) -> &glutin::display::Display {
        &self.display
    }

    /// The configuration of the pbuffers and the contexts.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns true if the display has `extension`, such as `EGL_KHR_surfaceless_context`.
    pub fn has_extension(&self, extension: &str) -> bool {
        match &self.display {
            glutin::display::Display::Egl(display) => display.extensions().contains(extension),
            #[allow(unreachable_patterns)]
            _ => false,
        }
    }

    /// Creates a context with `attributes`, of the configuration of the pbuffers.
    pub fn create_context(
        &self,
        attributes: ContextAttributesBuilder,
    ) -> Result<NotCurrentContext, HeadlessError> {
        Ok(unsafe {
            self.display
                .create_context(&self.config, &attributes.build(None))?
        })
    }

    /// Creates a pbuffer of `width` by `height` pixels.
    pub fn create_pbuffer(
        &self,
        width: u32,
        height: u32,
    ) -> Result<Surface<PbufferSurface>, HeadlessError> {
        let size = |size| NonZeroU32::new(size).unwrap_or(NonZeroU32::MIN);
        let attributes =
            SurfaceAttributesBuilder::<PbufferSurface>::new().build(size(width), size(height));
        Ok(unsafe {
            self.display
                .create_pbuffer_surface(&self.config, &attributes)?
        })
    }

    /// Creates a display that draws its frames to a pbuffer of `width` by `height` pixels, with
    /// a context of the first selected level that the driver creates, and the `debug` callback
    /// behavior.
    pub fn display(
        &self,
        width: u32,
        height: u32,
        debug: DebugCallbackBehavior,
    ) -> Result<Display<PbufferSurface>, HeadlessError> {
        let context = level_context_apis()
            .into_iter()
            .find_map(|api| self.create_context(context_attributes_builder(api)).ok())
            .ok_or(HeadlessError::NoContext)?;
        let surface = self.create_pbuffer(width, height)?;
        let context = context.make_current(&surface)?;
        Ok(Display::from_pbuffer(context, surface, debug)?)
    }
}

impl fmt::Debug for Headless {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "[safegl::backend::glutin::headless::Headless]")
    }
}
