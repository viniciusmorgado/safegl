use std::env;
use std::fs::File;
use std::path::Path;

mod levels;
mod textures;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&dest);

    textures::build_texture_file(&mut File::create(&dest.join("textures.rs")).unwrap());
    levels::build(&mut File::create(dest.join("levels.rs")).unwrap());
    println!("cargo:rerun-if-changed=build/main.rs");
    println!("cargo:rerun-if-changed=build/levels.rs");

    // the EGL code, with the `egl` feature, on the platforms where glutin has EGL (the
    // `egl_backend` of glutin)
    println!("cargo::rustc-check-cfg=cfg(safegl_egl)");
    let families = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    let unix_or_windows = families.split(',').any(|family| family == "unix" || family == "windows");
    let apple = env::var("CARGO_CFG_TARGET_VENDOR").as_deref() == Ok("apple");
    let wasm = families.split(',').any(|family| family == "wasm");
    if env::var_os("CARGO_FEATURE_EGL").is_some() && unix_or_windows && !apple && !wasm {
        println!("cargo::rustc-cfg=safegl_egl");
    }

    // the tests draw on displays without window with `SAFEGL_TEST_DISPLAY=headless`
    println!("cargo::rustc-check-cfg=cfg(safegl_test_headless)");
    println!("cargo:rerun-if-env-changed=SAFEGL_TEST_DISPLAY");
    if env::var("SAFEGL_TEST_DISPLAY").as_deref() == Ok("headless") {
        println!("cargo::rustc-cfg=safegl_test_headless");
    }
}
