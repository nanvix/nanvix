// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    env,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    // Create a benchmark data directory with test files.
    let manifest_dir: String = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .expect("no parent: benchmarks/")
        .parent()
        .expect("no parent: src/")
        .parent()
        .expect("no parent: root");
    let mount_dir: PathBuf = workspace_root.join("bin").join("mount-bench-data");
    std::fs::create_dir_all(&mount_dir).expect("failed to create mount-bench-data directory");

    // Create a 4 KiB test file for sequential I/O benchmarking.
    let data: Vec<u8> = vec![0xABu8; 4096];
    std::fs::write(mount_dir.join("bench-4k.bin"), &data).expect("failed to write bench-4k.bin");

    println!("cargo:rerun-if-changed=build.rs");
}
