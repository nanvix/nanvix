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
// Constants
//==================================================================================================

/// Name of the fixture file placed in the host-mounted directory.
///
/// The guest test mounts the host filesystem at `/mnt`, opens this file, forks, and verifies that
/// the child inherits the open descriptor and shares its offset. Keep this in sync with
/// `FIXTURE_PATH` in `src/tests/test-fork-hostfs/src/tests/fork_hostfs.rs`.
const FIXTURE_NAME: &str = "forkfd.dat";

/// Number of bytes written to the fixture file. Byte `i` holds the value `i as u8`, giving the
/// guest a deterministic pattern to assert against.
const FIXTURE_LEN: usize = 256;

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");
    println!("cargo:rerun-if-changed=build.rs");

    // Create the host directory that the test runner passes to `-mount`, populated with the
    // deterministic fixture file the guest reads across `fork()`. Always regenerate from scratch so
    // that a stale directory from a previous run cannot mask a missing or corrupt fixture.
    let manifest_dir: String = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .expect("no parent: tests/")
        .parent()
        .expect("no parent: src/")
        .parent()
        .expect("no parent: root");
    let mount_dir = workspace_root.join("bin").join("test-fork-hostfs-data");

    // Remove any stale data from a previous run, then recreate.
    let _ = std::fs::remove_dir_all(&mount_dir);
    std::fs::create_dir_all(&mount_dir).expect("failed to create test-fork-hostfs-data directory");

    // Write the fixture file holding a deterministic ramp (`byte[i] == i as u8`).
    let mut payload: Vec<u8> = vec![0u8; FIXTURE_LEN];
    for (i, byte) in payload.iter_mut().enumerate() {
        *byte = i as u8;
    }
    std::fs::write(mount_dir.join(FIXTURE_NAME), &payload).expect("failed to write fixture file");
}
