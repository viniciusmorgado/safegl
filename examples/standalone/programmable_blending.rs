//! Blend modes that fixed-function blending can't do, with the texture barrier; and clearing and
//! copying textures without a framebuffer.
//!
//! Soft round brushes paint on a canvas texture. The canvas is the framebuffer they draw to and,
//! at the same time, a texture their fragment shader reads: each fragment reads the texel under
//! it, blends the brush color into it with a "Photoshop" blend mode, and writes the result back.
//! Reading a texture attached to the framebuffer being drawn to is normally undefined;
//! `texture::texture_barrier` between the brushes makes it well defined, because each draw
//! then reads only texels that no other draw since the last barrier has written.
//!
//! - `1`: overlay, `2`: soft light, `3`: color dodge.
//! - `Space`: copy the canvas into the snapshot shown in the bottom-left corner
//!   (`TextureAnyMipmap::copy_to`, `glCopyImageSubData`). A snapshot is also taken every
//!   4 seconds.
//! - `C`: clear the canvas to grey (`TextureAnyMipmap::clear`, `glClearTexImage`).
//!
//! The first blend mode can also be given on the command line:
//! `cargo run --example programmable_blending -- 2`.
//!
//! Needs the `gl45` level, or a higher one.

#[cfg(safegl_gl45)]
#[macro_use]
extern crate safegl;

#[cfg(safegl_gl45)]
mod support;

#[cfg(safegl_gl45)]
#[path = "programmable_blending/example.rs"]
mod example;

#[cfg(safegl_gl45)]
fn main() {
    example::main();
}

#[cfg(not(safegl_gl45))]
fn main() {
    eprintln!(
        "This example needs the `gl45` level, or a higher one: run it with `--features gl45`."
    );
}
