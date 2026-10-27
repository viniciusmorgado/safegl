//! Needs a desktop OpenGL level or `gles32`.

#[cfg(any(safegl_desktop, safegl_gles32))]
#[macro_use]
extern crate safegl;

#[cfg(any(safegl_desktop, safegl_gles32))]
mod support;

#[cfg(any(safegl_desktop, safegl_gles32))]
#[path = "displacement_mapping/example.rs"]
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
