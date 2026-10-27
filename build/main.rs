use std::env;
use std::fs::File;
use std::path::Path;

mod levels;
mod textures;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&dest);

    textures::build_texture_file(&mut File::create(&dest.join("textures.rs")).unwrap());
    levels::build(&mut File::create(&dest.join("levels.rs")).unwrap());
    println!("cargo:rerun-if-changed=build/main.rs");
    println!("cargo:rerun-if-changed=build/levels.rs");
}
