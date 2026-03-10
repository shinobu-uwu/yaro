use std::env;
use std::path::PathBuf;

fn main() {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let linker_script = PathBuf::from(manifest_dir)
        .join("linker-scripts")
        .join(format!("linker-{arch}.ld"));

    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    println!("cargo:rerun-if-changed={}", linker_script.display());
}
