/*!

This example demonstrates how to manually create a SafeGL context with any backend you want, most
notably without glutin.

There are three concepts in play:

 - The `Backend` trait, which defines how SafeGL interfaces with the OpenGL context
   provider (glutin, SDL, glfw, etc.).

 - The `Context` struct, which is the main object of SafeGL. The context also provides
   OpenGL-related functions like `get_free_video_memory` or `get_supported_glsl_version`.

 - The `Facade` trait, which is the trait required to be implemented on objects that you pass
   to functions like `VertexBuffer::new`. This trait is implemented on `Rc<Context>`, which
   means that you can direct pass the context.

*/

use safegl::Surface;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::SurfaceAttributesBuilder;
use glutin::surface::WindowSurface;
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use winit::event_loop::EventLoop;

use std::cell::RefCell;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::os::raw::c_void;
use std::rc::Rc;
use takeable_option::Takeable;

fn main() {
    let event_loop = EventLoop::builder().build().expect("event loop building");
    let window_attributes = winit::window::Window::default_attributes();
    let config_template_builder = ConfigTemplateBuilder::new();
    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

    // First we create a window
    let (window, gl_config) = display_builder
        .build(&event_loop, config_template_builder, |mut configs| {
            // Just use the first configuration since we don't have any special preferences here
            configs.next().unwrap()
        })
        .unwrap();
    let window = window.unwrap();

    // Then the configuration which decides which OpenGL version we'll end up using. SafeGL needs at
    // least OpenGL 4.1, so we request it explicitly: glutin's default is 3.3 core, which is too old.
    // When this fails we'll try and create an OpenGL ES 3.0 context, this is mainly used on mobile
    // devices or various ARM SBC's.
    // If you depend on features available in modern OpenGL Versions you need to request a specific, modern, version. Otherwise things will very likely fail.
    let window_handle = window.window_handle().unwrap();
    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 1))))
        .build(Some(window_handle.into()));
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
        .build(Some(window_handle.into()));

    let not_current_gl_context = Some(unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_config
                    .display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
    });

    // Determine our framebuffer size based on the window size, or default to 800x600 if it's invisible
    let (width, height): (u32, u32) = window.inner_size().into();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window_handle.into(),
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    );
    // Now we can create our surface, use it to make our context current and finally create our display

    let surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .unwrap()
    };
    let context = not_current_gl_context
        .unwrap()
        .make_current(&surface)
        .unwrap();
    let gl_window = Rc::new(RefCell::new(Takeable::new((context, surface))));
    // the event loop resizes the surface along with the window
    let resizable_gl_window = gl_window.clone();

    // in order to create our context, we will need to provide an object which implements
    // the `Backend` trait
    struct Backend {
        gl_window: Rc<
            RefCell<
                Takeable<(
                    glutin::context::PossiblyCurrentContext,
                    glutin::surface::Surface<WindowSurface>,
                )>,
            >,
        >,
    }

    unsafe impl safegl::backend::Backend for Backend {
        fn swap_buffers(&self) -> Result<(), safegl::SwapBuffersError> {
            let gl_window = self.gl_window.borrow();
            match gl_window.1.swap_buffers(&gl_window.0) {
                Ok(()) => Ok(()),
                Err(glutin::error::Error { .. }) => Err(safegl::SwapBuffersError::ContextLost),
            }
        }

        // this function is called only after the OpenGL context has been made current
        unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
            let symbol = CString::new(symbol).unwrap();
            self.gl_window
                .borrow()
                .0
                .display()
                .get_proc_address(&symbol) as *const _
        }

        // this function is used to adjust the viewport when the user wants to draw or blit on
        // the whole window
        fn get_framebuffer_dimensions(&self) -> (u32, u32) {
            // we default to a dummy value is the window no longer exists
            let window = &self.gl_window.borrow().1;
            (window.width().unwrap(), window.height().unwrap())
        }

        fn resize(&self, new_size: (u32, u32)) {
            let pair = &self.gl_window.borrow();
            let context = &pair.0;
            let window = &pair.1;
            window.resize(
                context,
                NonZeroU32::new(new_size.0).unwrap(),
                NonZeroU32::new(new_size.1).unwrap(),
            );
        }

        fn is_current(&self) -> bool {
            // if you are using a library that doesn't provide an equivalent to `is_current`, you
            // can just put `unimplemented!` and pass `false` when you create
            // the `Context` (see below)
            self.gl_window.borrow().0.is_current()
        }

        unsafe fn make_current(&self) {
            let mut gl_window_takeable = self.gl_window.borrow_mut();
            let gl_window = Takeable::take(&mut gl_window_takeable);
            gl_window.0.make_current(&gl_window.1).unwrap();
            Takeable::insert(&mut gl_window_takeable, gl_window);
        }

        fn set_swap_interval(&self, interval: safegl::backend::SwapInterval) {
            let interval = match interval {
                safegl::backend::SwapInterval::DontWait => glutin::surface::SwapInterval::DontWait,
                safegl::backend::SwapInterval::Wait(frames) => {
                    glutin::surface::SwapInterval::Wait(frames)
                }
            };
            let pair = &self.gl_window.borrow();
            let context = &pair.0;
            let window = &pair.1;
            window.set_swap_interval(context, interval).unwrap();
        }
    }

    // now building the context
    let context = unsafe {
        // The first parameter is our backend.
        //
        // The second parameter tells SafeGL whether or not it should regularly call `is_current`
        // on the backend to make sure that the OpenGL context is still the current one.
        //
        // It is recommended to pass `true`, but you can pass `false` if you are sure that no
        // other OpenGL context will be made current in this thread.
        let backend = Backend {
            gl_window: gl_window,
        };
        safegl::backend::Context::new(backend, true, Default::default())
    }
    .unwrap();

    // drawing a frame to prove that it works
    // note that constructing a `Frame` object manually is a bit hacky and may be changed
    // in the future
    let draw = |context: &Rc<safegl::backend::Context>| {
        let mut target = safegl::Frame::new(context.clone(), context.get_framebuffer_dimensions());
        target.clear_color(0.0, 1.0, 0.0, 1.0);
        target.finish().unwrap();
    };
    draw(&context);

    // the window is still available
    #[allow(deprecated)]
    event_loop
        .run(|event, window_target| match event {
            safegl::winit::event::Event::WindowEvent { event, .. } => match event {
                safegl::winit::event::WindowEvent::CloseRequested => window_target.exit(),
                // the surface keeps its size until it is resized, which would make the drawing
                // stretch or move with the window
                safegl::winit::event::WindowEvent::Resized(size) => {
                    if let (Some(width), Some(height)) =
                        (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        let pair = resizable_gl_window.borrow();
                        pair.1.resize(&pair.0, width, height);
                    }
                    window.request_redraw();
                }
                safegl::winit::event::WindowEvent::RedrawRequested => draw(&context),
                _ => (),
            },
            _ => (),
        })
        .unwrap();
}
