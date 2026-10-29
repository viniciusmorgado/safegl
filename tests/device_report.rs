//! A report of the driver of the tests, for the runs on devices
//! (`tests/cross/embedded_device_tests.lua`): its version, its renderer, its extensions, its
//! limits and the precision of its shaders. Ignored by default, it runs with
//! `cargo test --test device_report -- --ignored --nocapture`.

#![cfg(not(target_arch = "wasm32"))]

#[macro_use]
extern crate safegl;

use safegl::CapabilitiesSource;

mod support;

#[test]
#[ignore]
fn device_report() {
    let display = support::build_display();
    println!("version: {}", display.get_opengl_version_string());
    println!("vendor: {}", display.get_opengl_vendor_string());
    println!("renderer: {}", display.get_opengl_renderer_string());
    println!("extensions: {:#?}", display.get_extensions());
    println!("capabilities: {:#?}", display.get_capabilities());
    #[cfg(any(safegl_gl41, safegl_gles30))]
    {
        use safegl::program::{PrecisionQualifier as Q, ShaderType};
        for shader in [ShaderType::Vertex, ShaderType::Fragment] {
            for qualifier in [
                Q::HighFloat,
                Q::MediumFloat,
                Q::LowFloat,
                Q::HighInt,
                Q::MediumInt,
                Q::LowInt,
            ] {
                let format = display.shader_precision_format(shader, qualifier);
                println!("precision of {shader:?} {qualifier:?}: {format:?}");
            }
        }
    }
}
