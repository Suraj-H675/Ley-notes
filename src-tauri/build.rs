use std::{env, fs, path::PathBuf};

fn main() {
    // `tauri-build` validates configured bundle resources while Cargo is still
    // compiling the Desktop crate. The real helper is built by Tauri's
    // beforeDev/beforeBundle hooks, which run outside this build-script process.
    // Keep an ignored zero-byte placeholder so plain `cargo check/test` remains
    // valid without recursively invoking Cargo from build.rs.
    let target = env::var("TARGET").expect("Cargo TARGET");
    let helper_name = if target.contains("windows") {
        "ley-helper.exe"
    } else {
        "ley-helper"
    };
    let generated = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Cargo manifest dir"))
        .join("generated");
    fs::create_dir_all(&generated).expect("create generated Tauri resources directory");
    let helper = generated.join(helper_name);
    if !helper.exists() {
        fs::write(&helper, []).expect("create Ley helper resource placeholder");
    }

    tauri_build::build()
}
