// Based on https://github.com/Erkaman/image-load-store-demo
// License included at the end of the file.
//
// This example showcases image load/store functionality in compute shaders, which
// allows you to read and write to arbitrary textures in a shader.
//
// Three compute shaders are launched. The first one draws the mandelbrot fractal. The second
// one applies a box filter on the output of the first. Finally, the third one copies data
// the original unsigned integer texture to a signed normalized integer texture.
// Finally, this last texture is blitted to the display framebuffer.
//
// Needs the `gl43` or `gles31` level, or a higher one.

#[cfg(any(safegl_gl43, safegl_gles31))]
#[macro_use]
extern crate safegl;

#[cfg(any(safegl_gl43, safegl_gles31))]
mod support;

#[cfg(any(safegl_gl43, safegl_gles31))]
#[path = "compute_image/example.rs"]
mod example;

#[cfg(any(safegl_gl43, safegl_gles31))]
fn main() {
    example::main();
}

#[cfg(not(any(safegl_gl43, safegl_gles31)))]
fn main() {
    eprintln!(
        "This example needs the `gl43` or `gles31` level, or a higher one: run it with `--features gl43`."
    );
}
