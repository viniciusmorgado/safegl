// This example demonstrates SPIR-V shader loading.
// It's based on the screenshot example but uses an empty vertex buffer.
//
// Needs the `gl46` level, or a higher one.

#[cfg(safegl_gl46)]
#[macro_use]
extern crate safegl;

#[cfg(safegl_gl46)]
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
