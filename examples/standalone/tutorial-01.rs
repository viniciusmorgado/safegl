#![deny(deprecated)]

use safegl::{glutin, winit, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

type Display = safegl::Display<glutin::surface::WindowSurface>;

struct State {
    display: Display,
    window: Window,
}

impl State {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let (window, display) = safegl::backend::glutin::SimpleWindowBuilder::new()
            .with_title("SafeGL tutorial #1")
            .build(event_loop);

        Self { display, window }
    }

    fn draw(&self) {
        let mut target = self.display.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);
        target.finish().unwrap();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let state = State::new(event_loop);
            state.window.request_redraw();
            self.state = Some(state);
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.state = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &self.state else { return };
        if state.window.id() != window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                state.display.resize(size.into());
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let size = state.window.inner_size();
                if size.width > 0 && size.height > 0 {
                    state.draw();
                }
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop
        .run_app(&mut App::default())
        .expect("run application");
}
