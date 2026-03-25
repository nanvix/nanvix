// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    env,
    fs::File,
    io::Read,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    //==============================================================================================
    // Configuration
    //==============================================================================================

    // Get CARGO_MANIFEST_DIR Environment Variable.
    let manifest_dir: String = match env::var("CARGO_MANIFEST_DIR") {
        Ok(mdir) => mdir,
        Err(_) => panic!("failed to get CARGO_MANIFEST_DIR environment variable"),
    };

    let config_path: PathBuf = Path::new(&manifest_dir).join("config.json");
    if config_path.exists() {
        println!("cargo:rerun-if-changed=./config.json");
        let mut file: File = File::open(&config_path).expect("Failed to open file");
        let mut raw_content: String = String::new();
        let _ = file.read_to_string(&mut raw_content);
        let content = raw_content.replace("\n", "").replace(" ", "");
        println!("cargo:rustc-env=CONFIG={content}");
    }

    //==============================================================================================
    // Linker Configuration
    //==============================================================================================

    println!("cargo::rustc-link-arg=-Tbuild/user/linker/x86/user.ld");

    // Build as a position-independent executable so the linker emits .dynsym,
    // .dynstr, .dynamic, and .hash sections.  Without -pie the binary is fully
    // static and --export-dynamic is silently ignored.
    println!("cargo::rustc-link-arg=-pie");

    // Export all symbols to the dynamic symbol table so that dlsym() with
    // DlHandle::GLOBAL can resolve symbols defined in the main executable.
    println!("cargo::rustc-link-arg=--export-dynamic");

    // Suppress PT_INTERP — Nanvix has no system dynamic linker.
    println!("cargo::rustc-link-arg=--no-dynamic-linker");
}
