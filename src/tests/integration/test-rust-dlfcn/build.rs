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

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    // Build as a position-independent executable so --export-dynamic emits the
    // dynamic symbol tables used by DlHandle::GLOBAL. The x86_64 build uses the
    // repository's dedicated PIC target.
    println!("cargo::rustc-link-arg=-pie");

    // Nanvix loads executable PIEs at their fixed link address. ELF64 RELA
    // targets otherwise remain zero until a system dynamic linker applies
    // them, which Nanvix intentionally does not provide.
    if target_arch == "x86_64" {
        println!("cargo::rustc-link-arg=--apply-dynamic-relocs");
    }

    // Export all symbols to the dynamic symbol table so that dlsym() with
    // DlHandle::GLOBAL can resolve symbols defined in the main executable.
    println!("cargo::rustc-link-arg=--export-dynamic");

    // Suppress PT_INTERP — Nanvix has no system dynamic linker.
    println!("cargo::rustc-link-arg=--no-dynamic-linker");

    // Allow relocations against local symbols from Rust libraries compiled
    // without PIC. GNU ld allows these by default, but lld requires this flag.
    println!("cargo::rustc-link-arg=-z");
    println!("cargo::rustc-link-arg=notext");

    // Disable RELRO.  LLD's default RELRO layout creates a separate LOAD
    // segment for .dynamic right after .text.  That segment starts mid-page
    // (sharing the last page of .text) with RW permissions, while .text
    // maps the same page R+X.  The Nanvix kernel maps pages per-segment and
    // cannot remap a page with conflicting permissions — this causes a crash.
    // With -z norelro, LLD places .dynamic inside the regular data segment,
    // matching the layout GNU ld produces.
    println!("cargo::rustc-link-arg=-z");
    println!("cargo::rustc-link-arg=norelro");

    // Use legacy SysV hash style to avoid .gnu.hash / .hash section type
    // mismatch when lld merges sections specified by the linker script.
    println!("cargo::rustc-link-arg=--hash-style=sysv");
}
