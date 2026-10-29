/*!

Draws on Android, and keeps its OpenGL ES objects while the application is paused.

Android destroys the window of a paused application, and gives it a new one when it is resumed.
The display removes its surface before the window goes (`Display::remove_surface`), the context
staying current without surface, and gets the surface of the new window when the application is
resumed (`Display::set_surface`): textures, buffers and programs live on.

Built as a shared library for an Android application, with an OpenGL ES level, for example with
`cargo apk run --example android --no-default-features --features gles30,simple_window_builder`
(`cargo-apk`) or `x run --example android` (`xbuild`). On other systems the library is empty.

*/

#[cfg(target_os = "android")]
mod android {
    use std::num::NonZeroU32;

    use glutin::config::{Config, ConfigTemplateBuilder};
    use glutin::display::GetGlDisplay;
    use glutin::prelude::*;
    use glutin::surface::{Surface as GlutinSurface, SurfaceAttributesBuilder, WindowSurface};
    use glutin_winit::DisplayBuilder;
    use raw_window_handle::HasWindowHandle;
    use safegl::winit::application::ApplicationHandler;
    use safegl::winit::event::WindowEvent;
    use safegl::winit::event_loop::{ActiveEventLoop, EventLoop};
    use safegl::winit::platform::android::activity::AndroidApp;
    use safegl::winit::platform::android::EventLoopBuilderExtAndroid;
    use safegl::winit::window::{Window, WindowId};
    use safegl::{Display, Surface};

    #[derive(Default)]
    struct App {
        config: Option<Config>,
        window: Option<Window>,
        display: Option<Display<WindowSurface>>,
        frames: u32,
    }

    /// Creates the surface of `window`.
    fn window_surface(config: &Config, window: &Window) -> GlutinSurface<WindowSurface> {
        let size = window.inner_size();
        let attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.window_handle().unwrap().as_raw(),
            NonZeroU32::new(size.width.max(1)).unwrap(),
            NonZeroU32::new(size.height.max(1)).unwrap(),
        );
        unsafe {
            config
                .display()
                .create_window_surface(config, &attributes)
                .unwrap()
        }
    }

    impl ApplicationHandler for App {
        // the application starts or comes back with a new window
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window = match &self.config {
                Some(config) => {
                    glutin_winit::finalize_window(event_loop, Window::default_attributes(), config)
                        .unwrap()
                }
                None => {
                    let template = ConfigTemplateBuilder::new();
                    let (window, config) = DisplayBuilder::new()
                        .with_window_attributes(Some(Window::default_attributes()))
                        .build(event_loop, template, |mut configs| configs.next().unwrap())
                        .unwrap();
                    self.config = Some(config);
                    window.unwrap()
                }
            };
            let config = self.config.as_ref().unwrap();
            let surface = window_surface(config, &window);
            match &self.display {
                // the objects of the display are still there
                Some(display) => {
                    display.set_surface(surface).unwrap();
                }
                None => {
                    let api = safegl::backend::glutin::level_context_apis()[0];
                    let attributes = safegl::backend::glutin::context_attributes_builder(api)
                        .build(Some(window.window_handle().unwrap().as_raw()));
                    let context = unsafe {
                        config
                            .display()
                            .create_context(config, &attributes)
                            .unwrap()
                    };
                    let context = context.make_current(&surface).unwrap();
                    self.display = Some(Display::new(context, surface).unwrap());
                }
            }
            window.request_redraw();
            self.window = Some(window);
        }

        // the window is destroyed after this: the display removes its surface first
        fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
            if let Some(display) = &self.display {
                drop(display.remove_surface().unwrap());
            }
            self.window = None;
        }

        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
            let (Some(display), Some(window)) = (&self.display, &self.window) else {
                return;
            };
            match event {
                WindowEvent::Resized(size) => display.resize(size.into()),
                WindowEvent::RedrawRequested => {
                    // a color that changes with the frames
                    self.frames += 1;
                    let shade = (self.frames % 120) as f32 / 120.0;
                    let mut frame = display.draw();
                    frame.clear_color(shade, 0.3, 1.0 - shade, 1.0);
                    frame.finish().unwrap();
                    window.request_redraw();
                }
                _ => (),
            }
        }
    }

    #[no_mangle]
    fn android_main(app: AndroidApp) {
        let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
        event_loop.run_app(&mut App::default()).unwrap();
    }
}
