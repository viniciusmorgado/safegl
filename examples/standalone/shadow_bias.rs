//! Shadow map bias with polygon offset, and how `polygon_offset_clamp` tames it.
//!
//! A teapot stands on the ground under a low sun that circles around it. Its shadow comes from a
//! shadow map: the depth of the scene seen from the sun, compared with the depth of each pixel.
//! Because the shadow map has a limited resolution, a surface compared with its own depth
//! shadows itself in stripes ("shadow acne"). Drawing the shadow map with a depth offset
//! (`DrawParameters::polygon_offset`) pushes surfaces away from the sun to fix that:
//!
//! - `1`: no offset: acne everywhere the sun grazes a surface.
//! - `2`: constant offset (`units`): acne remains on surfaces that the sun grazes, whose depth
//!   changes a lot within one texel of the shadow map.
//! - `3`: slope-scaled offset (`factor`, exaggerated here): the offset grows with the slope,
//!   which removes the acne, but where the sun grazes a surface it grows so large that the
//!   surface moves behind what it should shadow: the handle loses its shadow and shadows come
//!   loose from the objects that cast them ("peter-panning").
//! - `4`: slope-scaled offset with `polygon_offset_clamp`: no acne, and the shadow stays
//!   attached. `+`/`-` change the clamp.
//! - `L`: start or stop the sun. `Space`: start or stop the camera, which faces the sun so
//!   that the shadow falls towards it.
//!
//! The first mode can also be given on the command line: `cargo run --example shadow_bias -- 3`.
//!
//! Needs the `gl46` level, or a higher one.

#[cfg(safegl_gl46)]
#[macro_use]
extern crate safegl;

#[cfg(safegl_gl46)]
mod support;

#[cfg(safegl_gl46)]
#[path = "shadow_bias/example.rs"]
mod example;

#[cfg(safegl_gl46)]
fn main() {
    example::main();
}

#[cfg(not(safegl_gl46))]
fn main() {
    eprintln!(
        "This example needs the `gl46` level, or a higher one: run it with `--features gl46`."
    );
}
