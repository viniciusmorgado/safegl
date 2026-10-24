# Animating our triangle

Now that we have a triangle, we are going to animate it. Remember that OpenGL is like a drawing software. If we want to make a change on the screen, we have to draw over the existing content to replace what is already there.

So far we have drawn when the operating system requests a frame, then waited for another event. For an animation, we need to request redraws continuously. Add this method to `impl ApplicationHandler for App`:

```rust
fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Some(state) = &self.state {
        state.window.request_redraw();
    }
}
```

Winit calls `about_to_wait` before waiting for more events. Requesting a redraw schedules another `RedrawRequested` event, which calls `State::draw` as before. Keep the resize and close handlers from the first chapter.

Now we can start animating our triangle!

# The naive approach

Our first approach will be to calculate the triangle's new vertex positions on the CPU each frame. Store the time the animation started by adding a field to `State`:

```rust
start: std::time::Instant,
```

In `State::new`, initialize it with `let start = std::time::Instant::now();` and include `start` in the returned `Self`. Elapsed time gives the same animation speed at different frame rates.

Replace `State::draw` with:

```rust
fn draw(&self) {
    let t = self.start.elapsed().as_secs_f32();
    let x_off = t.sin() * 0.5;

    let shape = vec![
        Vertex { position: [-0.5 + x_off, -0.5] },
        Vertex { position: [0.0 + x_off, 0.5] },
        Vertex { position: [0.5 + x_off, -0.25] },
    ];
    let vertex_buffer = safegl::VertexBuffer::new(&self.display, &shape).unwrap();

    let mut target = self.display.draw();
    target.clear_color(0.0, 0.0, 1.0, 1.0);
    target.draw(&vertex_buffer, &self.indices, &self.program,
                &safegl::uniforms::EmptyUniforms, &Default::default()).unwrap();
    target.finish().unwrap();
}
```

If you run this code, you should see your triangle going from the left to the right and back again smoothly!

This method is approximately what game programmers were doing in the 1990s. This works perfectly fine when you have small shapes (like a single triangle), but it is highly inefficient when you manipulate models with thousands of polygons. There are two reasons for this:

 - The CPU would spend a lot of time calculating the coordinates every time you draw (with one operation for each vertex for each model, at the end you reach hundreds of thousands of operations).

 - It takes some time to upload our shape from RAM to video memory. This time is wasted as the GPU has to wait until the transfer is finished before it can start drawing.

# Uniforms

Do you remember vertex shaders? Our vertex shader takes as input the attributes of each vertex, and outputs its position on the window. Instead of doing the addition in our program and upload the result, we are going to ask the GPU to do this operation.

Let's remove the local `shape` and `vertex_buffer` from our draw method. We can use the vertex buffer stored in `State`, which was uploaded once during initialization:

```rust
fn draw(&self) {
    let t = self.start.elapsed().as_secs_f32();
    let x_off = t.sin() * 0.5;

    let mut target = self.display.draw();
    target.clear_color(0.0, 0.0, 1.0, 1.0);
    target.draw(&self.vertex_buffer, &self.indices, &self.program,
                &safegl::uniforms::EmptyUniforms, &Default::default()).unwrap();
    target.finish().unwrap();
}
```

And now we are going to change our vertex shader a little bit:

```rust
let vertex_shader_src = r#"
    #version 410 core

    in vec2 position;

    uniform float x_off;

    void main() {
        vec2 pos = position;
        pos.x += x_off;
        gl_Position = vec4(pos, 0.0, 1.0);
    }
"#;
```

You may notice that this is exactly the operation that we've been doing above, except that this time it is done on the GPU side. We have added a variable `x_off` in our shader, which is declared as a **uniform**. A uniform is a global variable whose value is set when we draw by passing its value to the `draw` function. The easiest way to do so is to use the `uniform!` macro:

```rust
target.draw(&self.vertex_buffer, &self.indices, &self.program, &uniform! { x_off: x_off },
            &Default::default()).unwrap();
```

Using uniform variables solves our two problems above. The CPU calculates just one offset and uploads the value of `x_off` (a single float) instead of the whole shape.

<details>
<summary>Complete source code</summary>

```rust,ignore
{{#include ../examples/standalone/tutorial-03.rs}}
```

</details>
