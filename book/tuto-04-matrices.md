
# Matrices

We are moving our triangle from the left to the right of the screen with a simple addition. But what about other transformations like rotations, skews or rescalings?

All the geometrical operations that we need can be done with some maths:

 - Rescaling our triangle is done with `position *= factor;`
 - Rotating our triangle is done with `new_position = vec2(pos.x * cos(angle) - pos.y * sin(angle), pos.x * sin(angle) + pos.y * cos(angle));`
 - Skewing our triangle is done with `position.x += position.y * factor;`

But what if we want to do a rotation, then a translation, then a rescale? Or a skew and a rotation? Even though it is possible to accomplish like this, things become quite complicated very quickly.

Instead, programmers use **matrices**. A matrix is a two-dimensional table of numbers which *can represent a geometrical transformation*. In computer graphics, we use 4x4 matrices.

Let's get back to our moving triangle. We are going to change the vertex shader to use a matrix. Instead of adding the value of `x_off` to the coordinates, we are going to apply the matrix to them by multiplying it. This applies the transformation described by our matrix to the vertex's coordinates.

```rust
let vertex_shader_src = r#"
    #version 410 core

    in vec2 position;

    uniform mat4 matrix;

    void main() {
        gl_Position = matrix * vec4(position, 0.0, 1.0);
    }
"#;
```

Note that it is important to write `matrix * vertex` and not `vertex * matrix`. Matrix operations produce different results depending on the order.

In `State::draw`, calculate `let t = self.start.elapsed().as_secs_f32();` and `let x = t.sin() * 0.5;`. We also need to pass the matrix when calling the `draw` function:

```rust
let uniforms = uniform! {
    matrix: [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [ x , 0.0, 0.0, 1.0f32],
    ]
};

target.draw(&self.vertex_buffer, &self.indices, &self.program, &uniforms,
            &Default::default()).unwrap();
```

Note that in OpenGL, and therefore SafeGL, the matrices are column-major.  If we were to write the above matrix in standard mathematical notation, which is row-major, it would look like this:
```
1.0   0.0   0.0    x
0.0   1.0   0.0   0.0
0.0   0.0   1.0   0.0
0.0   0.0   0.0   1.0
```

You should see exactly the same thing as previously, but what we now have is much more flexible. For example, if instead we want to rotate the triangle we can try this matrix instead:

```rust
let uniforms = uniform! {
    matrix: [
        [ t.cos(), t.sin(), 0.0, 0.0],
        [-t.sin(), t.cos(), 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0f32],
    ]
};
```

<details>
<summary>Complete source code</summary>

```rust,ignore
{{#include ../examples/standalone/tutorial-04.rs}}
```

</details>
