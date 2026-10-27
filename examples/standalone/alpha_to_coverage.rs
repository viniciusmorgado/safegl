//! Anti-aliasing alpha-tested foliage with alpha-to-coverage, sample shading and sample masks.
//!
//! A field of ferns is drawn into a 4x multisampled target, which is resolved with a blit and
//! shown with a magnified inset (bottom right) of the area outlined in the middle of the screen.
//! The ferns are cut out of their leaf texture by its alpha channel. How that cut-out is turned
//! into coverage is what the modes compare:
//!
//! - `1`: alpha test: fragments with an alpha below 0.5 are discarded. Multisampling only
//!   smooths the edges of triangles, not the edges of the leaves, which stay aliased.
//! - `2`: alpha-to-coverage with the alpha of the texture: an alpha of 0.25 covers about one
//!   sample out of four. Edges are smooth, but the blurry alpha of the small mipmaps makes
//!   distant ferns look thin and transparent.
//! - `3`: alpha-to-coverage with sharpened alpha: the alpha is rescaled so that the 0.5
//!   cut-off becomes an edge about one pixel wide, crisp at every distance.
//! - `4`: like `3`, with the fragment shader run for every sample (`sample_shading`), so the
//!   texture is also looked up per sample.
//! - `5`: like `3`, with a sample mask; press again to keep 1, 2, 3 or 4 of the 4 samples.
//! - `Space`: pause or resume the camera. `Up`/`Down`: move the camera closer or further.
//!
//! The first mode can also be given on the command line, for example
//! `cargo run --example alpha_to_coverage -- 3`, or `-- 5 2` for a mask keeping 2 samples.
//!
//! The fern model is "Fern 02" from Poly Haven (CC0), see `support/fern_02/README.md`.
//!
//! Needs a desktop OpenGL level or `gles32`.

#[cfg(any(safegl_desktop, safegl_gles32))]
#[macro_use]
extern crate safegl;

#[cfg(any(safegl_desktop, safegl_gles32))]
mod support;

#[cfg(any(safegl_desktop, safegl_gles32))]
#[path = "alpha_to_coverage/example.rs"]
mod example;

#[cfg(any(safegl_desktop, safegl_gles32))]
fn main() {
    example::main();
}

#[cfg(not(any(safegl_desktop, safegl_gles32)))]
fn main() {
    eprintln!(
        "This example needs a desktop OpenGL level or `gles32`: run it with `--features gl40`."
    );
}
