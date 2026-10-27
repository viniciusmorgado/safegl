//! Turns the level features into cfgs and checks the combination of levels.
//!
//! A level is a ceiling: `cfg(safegl_gl43)` means that the desktop level is OpenGL 4.3 or above,
//! and `cfg(safegl_gles31)` that the OpenGL ES level is 3.1 or above, so the items of a version
//! are gated by the lowest version of each chain that has them. Without any level, the level is
//! `gl40`. Mistakes are reported by `compile_error!`s written to `levels.rs`, which the crate
//! includes.

use std::env;
use std::fs::File;
use std::io::Write;

const DESKTOP: [&str; 7] = ["gl40", "gl41", "gl42", "gl43", "gl44", "gl45", "gl46"];
const EMBEDDED: [&str; 3] = ["gles30", "gles31", "gles32"];

fn feature(name: &str) -> bool {
    env::var_os(format!(
        "CARGO_FEATURE_{}",
        name.to_uppercase().replace('-', "_")
    ))
    .is_some()
}

/// The index of the selected level of a chain, and an error if there are several.
fn selected(chain: &[&str], errors: &mut Vec<String>, kind: &str) -> Option<usize> {
    let levels: Vec<usize> = (0..chain.len()).filter(|&i| feature(chain[i])).collect();
    if levels.len() > 1 {
        errors.push(format!(
            "SafeGL levels `{}` and `{}` are both selected: select a single {} level",
            chain[levels[0]], chain[levels[1]], kind
        ));
    }
    levels.last().copied()
}

pub fn build(out: &mut File) {
    let mut errors = Vec::new();
    let mut desktop = selected(&DESKTOP, &mut errors, "desktop OpenGL");
    let embedded = selected(&EMBEDDED, &mut errors, "OpenGL ES");
    let webgl = feature("webgl");

    let browser = env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32")
        && env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("unknown");
    if browser {
        for level in desktop
            .map(|i| DESKTOP[i])
            .into_iter()
            .chain(embedded.map(|i| EMBEDDED[i]))
        {
            errors.push(format!(
                "the `{}` level can't be selected in the browser, which only has WebGL 2: select \
                 the `webgl` level instead",
                level
            ));
        }
        if !webgl {
            errors.push("browsers only have WebGL 2: select the `webgl` level".to_owned());
        }
    } else if desktop.is_none() && embedded.is_none() && !webgl {
        desktop = Some(0);
    }

    let mut names = vec!["safegl_desktop", "safegl_embedded", "safegl_webgl"];
    names.extend(
        DESKTOP
            .iter()
            .map(|level| leak(format!("safegl_{}", level))),
    );
    names.extend(
        EMBEDDED
            .iter()
            .map(|level| leak(format!("safegl_{}", level))),
    );
    println!("cargo::rustc-check-cfg=cfg({})", names.join(", "));

    if let Some(desktop) = desktop {
        println!("cargo::rustc-cfg=safegl_desktop");
        for level in &DESKTOP[..=desktop] {
            println!("cargo::rustc-cfg=safegl_{}", level);
        }
    }
    if let Some(embedded) = embedded {
        println!("cargo::rustc-cfg=safegl_embedded");
        for level in &EMBEDDED[..=embedded] {
            println!("cargo::rustc-cfg=safegl_{}", level);
        }
    }
    if webgl {
        println!("cargo::rustc-cfg=safegl_webgl");
    }

    for error in errors {
        writeln!(out, "compile_error!({:?});", error).unwrap();
    }
}

fn leak(name: String) -> &'static str {
    Box::leak(name.into_boxed_str())
}
