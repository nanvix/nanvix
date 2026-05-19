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
    path::Path,
};

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");
    println!("cargo:rerun-if-changed=build.rs");

    // Create a test directory with sample files that the test runner will pass to `-mount`.
    // Always regenerate from scratch because a previous test run may have modified
    // the contents directly via hostfsd.
    let manifest_dir: String = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .expect("no parent: tests/")
        .parent()
        .expect("no parent: src/")
        .parent()
        .expect("no parent: root");
    let mount_dir = workspace_root.join("bin").join("mount-test-data");

    // Remove any stale data from a previous run, then recreate.
    let _ = std::fs::remove_dir_all(&mount_dir);
    std::fs::create_dir_all(&mount_dir).expect("failed to create mount-test-data directory");

    // Write a known test file that the guest will read and verify.
    std::fs::write(mount_dir.join("input.txt"), b"mount-test-input\n")
        .expect("failed to write input.txt");

    // Write a subdirectory with a nested file.
    let sub = mount_dir.join("subdir");
    std::fs::create_dir_all(&sub).expect("failed to create subdir");
    std::fs::write(sub.join("nested.txt"), b"nested-content\n")
        .expect("failed to write nested.txt");
}
