//! Needs a desktop OpenGL level.

#[cfg(safegl_desktop)]
#[macro_use]
extern crate safegl;

#[cfg(safegl_desktop)]
mod support;

#[cfg(safegl_desktop)]
#[path = "subroutines/example.rs"]
mod example;

#[cfg(safegl_desktop)]
fn main() {
    example::main();
}

#[cfg(not(safegl_desktop))]
fn main() {
    eprintln!(
        "This example needs a desktop OpenGL level: run it with `--features gl40`."
    );
}
