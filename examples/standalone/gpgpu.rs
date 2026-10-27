//! Needs the `gl43` or `gles31` level, or a higher one.

#[cfg(any(safegl_gl43, safegl_gles31))]
#[macro_use]
extern crate safegl;

#[cfg(any(safegl_gl43, safegl_gles31))]
mod support;

#[cfg(any(safegl_gl43, safegl_gles31))]
#[path = "gpgpu/example.rs"]
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
