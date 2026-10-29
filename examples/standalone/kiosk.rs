/*!

Draws fullscreen without window system, the way kiosks and embedded boards do: the frames are
drawn on a GBM surface and shown on a screen with DRM/KMS page flips, without X11 nor Wayland.

Run it from a text console (Ctrl+Alt+F3 on most distributions), with access to the DRM device:
the graphical session holds the screen otherwise. `DRM_DEVICE` chooses the device, `/dev/dri/card0`
by default, and `FRAMES` the number of frames, 300 by default.

The display is an ordinary `Display<WindowSurface>`: glutin creates its EGL display from the GBM
device and its surface from the GBM surface, as it would from a window. `libgbm` is loaded when
the example runs.

*/

#[cfg(target_os = "linux")]
mod kiosk {
    use std::ffi::{c_int, c_void};
    use std::fs::{File, OpenOptions};
    use std::num::NonZeroU32;
    use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
    use std::ptr::NonNull;

    use drm::buffer::{Buffer, DrmFourcc, Handle};
    use drm::control::{connector, crtc, framebuffer, Device as ControlDevice, PageFlipFlags};
    use glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder};
    use glutin::display::{Display as GlutinDisplay, DisplayApiPreference};
    use glutin::prelude::*;
    use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
    use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};
    use safegl::{Display, Surface};

    /// `GBM_FORMAT_XRGB8888`, `GBM_BO_USE_SCANOUT` and `GBM_BO_USE_RENDERING`.
    const XRGB8888: u32 = u32::from_le_bytes(*b"XR24");
    const SCANOUT_AND_RENDERING: u32 = 1 | 4;

    /// The functions of `libgbm` that the example calls.
    struct Gbm {
        _library: libloading::Library,
        create_device: unsafe extern "C" fn(c_int) -> *mut c_void,
        destroy_device: unsafe extern "C" fn(*mut c_void),
        create_surface: unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32) -> *mut c_void,
        destroy_surface: unsafe extern "C" fn(*mut c_void),
        lock_front_buffer: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        release_buffer: unsafe extern "C" fn(*mut c_void, *mut c_void),
        // the `gbm_bo_handle` union, whose `u32` member is the GEM handle
        buffer_handle: unsafe extern "C" fn(*mut c_void) -> u64,
        buffer_stride: unsafe extern "C" fn(*mut c_void) -> u32,
    }

    impl Gbm {
        fn load() -> Gbm {
            unsafe {
                let library = libloading::Library::new("libgbm.so.1").expect("libgbm is missing");
                macro_rules! symbol {
                    ($name:literal) => {
                        *library
                            .get($name.as_bytes())
                            .expect(concat!("libgbm has no ", $name))
                    };
                }
                Gbm {
                    create_device: symbol!("gbm_create_device"),
                    destroy_device: symbol!("gbm_device_destroy"),
                    create_surface: symbol!("gbm_surface_create"),
                    destroy_surface: symbol!("gbm_surface_destroy"),
                    lock_front_buffer: symbol!("gbm_surface_lock_front_buffer"),
                    release_buffer: symbol!("gbm_surface_release_buffer"),
                    buffer_handle: symbol!("gbm_bo_get_handle"),
                    buffer_stride: symbol!("gbm_bo_get_stride"),
                    _library: library,
                }
            }
        }
    }

    /// The DRM device, a graphics card.
    struct Card(File);

    impl AsFd for Card {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.0.as_fd()
        }
    }

    impl drm::Device for Card {}
    impl ControlDevice for Card {}

    /// A buffer of the GBM surface, which the screen scans out.
    struct ScanoutBuffer {
        size: (u32, u32),
        pitch: u32,
        handle: Handle,
    }

    impl Buffer for ScanoutBuffer {
        fn size(&self) -> (u32, u32) {
            self.size
        }

        fn format(&self) -> DrmFourcc {
            DrmFourcc::Xrgb8888
        }

        fn pitch(&self) -> u32 {
            self.pitch
        }

        fn handle(&self) -> Handle {
            self.handle
        }
    }

    /// The first connected screen: its connector, its preferred mode and a CRTC that drives it.
    fn screen(card: &Card) -> (connector::Handle, drm::control::Mode, crtc::Handle) {
        let resources = card.resource_handles().expect("the DRM resources can't be read");
        let connector = resources
            .connectors()
            .iter()
            .filter_map(|&handle| card.get_connector(handle, true).ok())
            .find(|connector| connector.state() == connector::State::Connected)
            .expect("no screen is connected");
        let mode = *connector.modes().first().expect("the screen has no mode");
        let crtc = connector
            .encoders()
            .iter()
            .filter_map(|&encoder| card.get_encoder(encoder).ok())
            .find_map(|encoder| {
                encoder
                    .crtc()
                    .or_else(|| resources.filter_crtcs(encoder.possible_crtcs()).first().copied())
            })
            .expect("no CRTC drives the screen");
        (connector.handle(), mode, crtc)
    }

    pub fn main() {
        let path = std::env::var("DRM_DEVICE").unwrap_or_else(|_| "/dev/dri/card0".to_owned());
        let frames: u32 = std::env::var("FRAMES").map_or(300, |frames| frames.parse().unwrap());
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap_or_else(|error| panic!("{path} can't be opened: {error}"));
        let card = Card(file);
        let (connector, mode, crtc) = screen(&card);
        let (width, height) = (mode.size().0 as u32, mode.size().1 as u32);

        // the GBM surface that the frames are drawn on, and that the screen scans out
        let gbm = Gbm::load();
        let device = unsafe { (gbm.create_device)(card.0.as_raw_fd()) };
        let device = NonNull::new(device).expect("GBM can't use the DRM device");
        let surface = unsafe {
            (gbm.create_surface)(device.as_ptr(), width, height, XRGB8888, SCANOUT_AND_RENDERING)
        };
        let surface = NonNull::new(surface).expect("the GBM surface can't be created");

        // the EGL display of the GBM device, and the surface of the GBM surface
        let display_handle = RawDisplayHandle::Gbm(GbmDisplayHandle::new(device.cast()));
        let window_handle = RawWindowHandle::Gbm(GbmWindowHandle::new(surface.cast()));
        let gl_display = unsafe { GlutinDisplay::new(display_handle, DisplayApiPreference::Egl) }
            .expect("EGL can't use the GBM device");
        let template = ConfigTemplateBuilder::new()
            .with_surface_type(ConfigSurfaceTypes::WINDOW)
            .compatible_with_native_window(window_handle)
            .build();
        let config = unsafe { gl_display.find_configs(template) }
            .unwrap()
            .next()
            .expect("no EGL configuration draws on the GBM surface");
        let api = safegl::backend::glutin::level_context_apis()[0];
        let attributes =
            safegl::backend::glutin::context_attributes_builder(api).build(Some(window_handle));
        let context = unsafe { gl_display.create_context(&config, &attributes) }.unwrap();
        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle,
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
        let gl_surface =
            unsafe { gl_display.create_window_surface(&config, &surface_attributes) }.unwrap();
        let context = context.make_current(&gl_surface).unwrap();
        let display = Display::new(context, gl_surface).unwrap();
        println!(
            "{}x{} on {path}, with {}",
            width,
            height,
            display.get_opengl_renderer_string()
        );

        // each frame is swapped into a buffer of the GBM surface, which the screen scans out
        // from the next vertical blank: the previous buffer is released after the flip
        let mut shown: Option<(*mut c_void, framebuffer::Handle)> = None;
        for frame_number in 0..frames {
            let shade = (frame_number % 120) as f32 / 120.0;
            let mut frame = display.draw();
            frame.clear_color(shade, 0.2, 1.0 - shade, 1.0);
            frame.finish().unwrap();

            let buffer = unsafe { (gbm.lock_front_buffer)(surface.as_ptr()) };
            assert!(!buffer.is_null(), "the GBM surface has no front buffer");
            let handle = unsafe { (gbm.buffer_handle)(buffer) } as u32;
            let scanout = ScanoutBuffer {
                size: (width, height),
                pitch: unsafe { (gbm.buffer_stride)(buffer) },
                handle: NonZeroU32::new(handle).expect("the buffer has no handle").into(),
            };
            let framebuffer = card.add_framebuffer(&scanout, 24, 32).unwrap();
            if shown.is_none() {
                card.set_crtc(crtc, Some(framebuffer), (0, 0), &[connector], Some(mode))
                    .expect("the screen can't be set: is another program holding it?");
            } else {
                card.page_flip(crtc, framebuffer, PageFlipFlags::EVENT, None)
                    .unwrap();
                // waiting for the flip
                for _ in card.receive_events().unwrap() {}
            }
            if let Some((old, old_framebuffer)) = shown.replace((buffer, framebuffer)) {
                card.destroy_framebuffer(old_framebuffer).unwrap();
                unsafe { (gbm.release_buffer)(surface.as_ptr(), old) };
            }
        }

        // the display and its EGL surface go before the GBM surface and device
        drop(display);
        drop(gl_display);
        unsafe {
            (gbm.destroy_surface)(surface.as_ptr());
            (gbm.destroy_device)(device.as_ptr());
        }
    }
}

#[cfg(target_os = "linux")]
fn main() {
    kiosk::main();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("this example draws with DRM/KMS and GBM, on Linux");
}
