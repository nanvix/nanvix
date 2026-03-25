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

/// Compiles `libs/mul.c` into a shared library using the Nanvix cross-compiler.
///
/// - `cc` — absolute path to the cross-compiler (e.g. `i686-nanvix-gcc`).
/// - `extra_cflags` — additional compiler flags (e.g. `-fPIE` for the PIE variant).
/// - `extra_ldflags` — additional linker flags.
/// - `output` — absolute path to the output `.so` file.
/// - `source` — absolute path to the `mul.c` source file.
fn build_shared_lib(
    cc: &str,
    extra_cflags: &[&str],
    extra_ldflags: &[&str],
    output: &Path,
    source: &Path,
) {
    let mut cmd = Command::new(cc);
    cmd.args(extra_cflags);
    cmd.args(["-shared", "-fPIC"]);
    cmd.args(extra_ldflags);
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
    // Previously they were built by the deleted C test Makefiles (dlfcn-c and dlfcn-pie-c).
    let mul_c: PathBuf = Path::new(&manifest_dir).join("libs/mul.c");
    println!("cargo:rerun-if-changed=libs/mul.c");

    let nanvix_cc: String = env::var("NANVIX_CC")
        .unwrap_or_else(|_| panic!("NANVIX_CC not set — required to cross-compile libmul.so"));
    // Strip sccache prefix if present — we need the bare compiler path.
    let nanvix_cc: &str = nanvix_cc
        .split_whitespace()
        .find(|s| s.contains("gcc"))
        .unwrap_or_else(|| nanvix_cc.split_whitespace().last().unwrap_or(&nanvix_cc));

    let libraries_dir: String = env::var("LIBRARIES_DIR")
        .unwrap_or_else(|_| panic!("LIBRARIES_DIR not set — required to place libmul.so"));
    let lib_dir = Path::new(&libraries_dir);

    // libmul.so — standard shared library (non-PIE).
    build_shared_lib(nanvix_cc, &[], &[], &lib_dir.join("libmul.so"), &mul_c);

    // libmul-pie.so — position-independent shared library.
    build_shared_lib(nanvix_cc, &[], &[], &lib_dir.join("libmul-pie.so"), &mul_c);

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
