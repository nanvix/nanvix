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
    process::Command,
};

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Compiles `libs/mul.c` into a shared library using the cross-compiler
/// specified by NANVIX_CC with the cross-compilation flags from NANVIX_CFLAGS.
///
/// - `output` — absolute path to the output `.so` file.
/// - `source` — absolute path to the `mul.c` source file.
fn build_shared_lib(output: &Path, source: &Path) {
    let cc = env::var("NANVIX_CC")
        .unwrap_or_else(|_| panic!("NANVIX_CC not set — required to cross-compile shared libs"));
    let cflags = env::var("NANVIX_CFLAGS").unwrap_or_else(|_| {
        panic!("NANVIX_CFLAGS not set — required to cross-compile shared libs")
    });
    let mut parts = cc.split_whitespace();
    let program = parts.next().unwrap_or_else(|| panic!("NANVIX_CC is empty"));
    let mut cmd = Command::new(program);
    cmd.args(parts);
    cmd.args(cflags.split_whitespace());
    cmd.args(["-shared", "-fPIC"]);
    cmd.arg(source);
    cmd.arg("-o");
    cmd.arg(output);
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));
    assert!(status.success(), "failed to build {}", output.display());
}

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
    // Build Shared Libraries (libmul.so and libmul-pie.so)
    //==============================================================================================

    // The dlfcn tests load these shared libraries at runtime via dlopen().
    // Uses the cross-compiler from NANVIX_CC with flags from NANVIX_CFLAGS.
    let mul_c: PathBuf = Path::new(&manifest_dir).join("libs/mul.c");
    println!("cargo:rerun-if-changed=libs/mul.c");

    // When rust-analyzer (or plain `cargo check`) runs without the full build
    // environment, these variables are absent.  Skip the shared-library build
    // and linker configuration so the IDE can still provide diagnostics.
    let libraries_dir: String = match env::var("LIBRARIES_DIR") {
        Ok(v) => v,
        Err(_) => return,
    };
    let lib_dir = Path::new(&libraries_dir);

    // Ensure the output directory exists (it may not in a fresh CI checkout).
    std::fs::create_dir_all(lib_dir)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", lib_dir.display()));

    // libmul.so — standard shared library (non-PIE).
    build_shared_lib(&lib_dir.join("libmul.so"), &mul_c);

    // libmul-pie.so — position-independent shared library.
    build_shared_lib(&lib_dir.join("libmul-pie.so"), &mul_c);

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
