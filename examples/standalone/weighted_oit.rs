//! Weighted blended order-independent transparency, with per-output blending, texture clears and
//! framebuffer invalidation.
//!
//! Six tinted glass panes cross each other around a teapot. Ordinary alpha blending gives the
//! right picture only if transparent surfaces are drawn from back to front, which is impossible
//! here: every pane is both in front of and behind the others. Weighted blended
//! order-independent transparency (Morgan McGuire and Louis Bavoil, 2013) avoids sorting by
//! drawing the panes, in any order, into two textures:
//!
//! - `accumulation` (RGBA16F) adds up the premultiplied colors, weighted to favour what is
//!   close to the camera. Its blending is `ONE, ONE`.
//! - `revealage` (R16F) multiplies `1 - alpha` of every surface: how much of the opaque scene
//!   shows through. Its blending is `ZERO, ONE_MINUS_SRC_COLOR`.
//!
//! The two outputs of one draw call blend differently, with `DrawParameters::outputs`. They also
//! start from different values (0 and 1), which `clear()` on each texture sets without a
//! framebuffer. A last pass composites the average transparent color over the opaque scene, and
//! the targets whose content isn't needed any more are invalidated.
//!
//! - `O`: switch between order-independent transparency and plain alpha blending without
//!   sorting, which shows panes in the wrong order.
//! - `R`: show the revealage.
//! - `M`: mask the revealage output with a per-output color mask: nothing tells the
//!   composite how much the panes cover any more, and they disappear.
//! - `Space`: pause or resume the camera.
//!
//! The first mode can also be given on the command line, with any of `unsorted`, `revealage`
//! and `masked`: `cargo run --example weighted_oit -- unsorted`.
//!
//! Needs the `gl44` level, or a higher one.

#[cfg(safegl_gl44)]
#[macro_use]
extern crate safegl;

#[cfg(safegl_gl44)]
mod support;

#[cfg(safegl_gl44)]
#[path = "weighted_oit/example.rs"]
mod example;

#[cfg(safegl_gl44)]
fn main() {
    example::main();
}

#[cfg(not(safegl_gl44))]
fn main() {
    eprintln!(
        "This example needs the `gl44` level, or a higher one: run it with `--features gl44`."
    );
}
