# Creating a project

To start this tutorial, we will create a new project from scratch. Even though it is highly recommended to be familiar with Rust and Cargo before starting, some little reminders are always good. Let's start by running:

```sh
cargo new --bin my_project
cd my_project
```

The directory you have just created should contain a `Cargo.toml` file which contains our project's metadata, plus a `src/main.rs` file which contains the Rust source code. This is the file we will edit throughout the tutorial.

In order to use SafeGL, you need to add it as a dependency to your `Cargo.toml` file.

```toml
[dependencies]
safegl = "0.1.0"
```

By default SafeGL includes the windowing dependencies and requests an OpenGL 4.6 core context. You will need a graphics driver that supports this version.

On macOS, or hardware limited to OpenGL 4.1, use this dependency configuration instead:

```toml
[dependencies]
safegl = { version = "0.1.0", default-features = false, features = ["gl41", "glutin_backend", "simple_window_builder"] }
glutin = "0.32.3"
glutin-winit = "0.5.0"
winit = "0.30.13"
```

The additional windowing dependencies enable their platform backends when SafeGL's default features are disabled.

The shaders in this tutorial use GLSL 4.10 core, which works with both configurations. You should now have a `src/main.rs` file that looks similar to this:

```rust
fn main() {
    println!("Hello, World!");
}
```

It is now time to start work on the main function!

# Creating a window

The first step when creating a graphical application is to create a window. If you have ever worked with OpenGL before, you know how hard it is to do this correctly. Both window creation and context creation are platform-specific, and they are sometimes complex and tedious.

We will create the window with `SimpleWindowBuilder` and handle its events with winit's `ApplicationHandler`. Import these types at the top of `src/main.rs`:

```rust
use safegl::{glutin, winit, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

type Display = safegl::Display<glutin::surface::WindowSurface>;
```

SafeGL re-exports `winit` and `glutin`, so these imports use compatible versions of the windowing libraries. `Display` is a short name for a SafeGL display backed by a window surface.

Our program needs to keep its window and display alive between events. We store them in a `State` struct:

```rust
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
}
```

The builder creates the window, an OpenGL context and the SafeGL display. It needs an `ActiveEventLoop`, which winit supplies when it calls our application's `resumed` method:

```rust
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(state) = &self.state else { return };
        if state.window.id() != window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => (),
        }
    }
}
```

`App` starts without a window, so its state is `None`. When the application resumes, we create the state and request its first redraw. The guard prevents repeated resume notifications from creating extra windows. If the application is suspended, we drop its state; another resume will create it again.

The `window_event` method receives events for our window. When the user closes it, `event_loop.exit()` asks winit to stop. Start the event loop on the main thread:

```rust
fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.run_app(&mut App::default()).expect("run application");
}
```

If you run the program now with `cargo run`, you should see a window that remains open until you close it. We still need to draw its contents.

# Drawing on the window

SafeGL and the OpenGL API work similarly to drawing software like Paint or GIMP. We begin with an empty image, then draw an object on it, then another object, then another object, etc. until we are satisfied with the result. But contrary to these programs, you don't want your users to see the intermediate steps. Only the final result should be displayed.

To accomplish this, OpenGL uses what is called *double buffering*. Instead of drawing directly on the window, we are drawing to an image stored in memory. Once we are finished drawing, the buffers are swapped to present the completed image.
This is represented in SafeGL by the `Frame` object. When you want to start drawing something on a window, you must first call `display.draw()` to produce a new `Frame`:

```rust
let mut target = self.display.draw();
```

We can then use this `target` as a drawing surface. One of the operations that OpenGL and SafeGL provide is filling the surface with a given color. This is what we are going to do.

```rust
target.clear_color(0.0, 0.0, 1.0, 1.0);
```

Note that to use this function, we will need to import the `Surface` trait first:

```rust
use safegl::Surface;
```

The four values that we pass to `clear_color` represent the four components of our color: red, green, blue and alpha. For this window, values from `0.0` to `1.0` represent the color intensity. Here we are drawing an opaque blue color.

As explained above, the user doesn't immediately see the blue color on the screen. At this point if we were in a game, we would most likely draw our characters, their weapons, the ground, the sky, etc. But in this tutorial we will just stop here:

```rust
target.finish().unwrap();
```

This call to `finish()` means that we have finished drawing. It consumes the `Frame` and presents the back buffer in the window. Our window is now filled with blue.

Put the drawing operations in a `draw(&self)` method inside `impl State`. Call it from `window_event` whenever winit sends `RedrawRequested`. We also need to resize the OpenGL surface when the window changes size:

```rust
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
```

A minimized window can have zero width or height. Skip drawing and resizing the GL surface in that case. We request a redraw after a valid resize so the blue background fills the new window dimensions.

Here is our full program:

```rust,ignore
{{#include ../examples/standalone/tutorial-01.rs}}
```
