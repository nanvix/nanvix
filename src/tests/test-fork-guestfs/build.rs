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

/// Name of the fixture file placed at the root of the generated filesystem image.
///
/// The guest test opens this file, forks, and verifies that the child inherits the open
/// descriptor. Keep this in sync with `FIXTURE_PATH` in
/// `src/tests/test-fork-guestfs/src/tests/fork_fds.rs`.
const FIXTURE_NAME: &str = "forkfd.dat";

/// Number of bytes written to the fixture file. Byte `i` holds the value `i as u8`, giving the
/// guest a deterministic pattern to assert against.
const FIXTURE_LEN: usize = 256;

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    //==============================================================================================
    // Link Archive
    //==============================================================================================

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    //==============================================================================================
    // Generate FAT Image
    //==============================================================================================

    let out_dir: String = env::var("OUT_DIR").expect("OUT_DIR not set");
    let img_path = Path::new(&out_dir).join("test-fork-guestfs.img");

    generate_fat_image(&img_path);

    // Copy the image to the build-artifacts directory (bin/) so that the test runner can
    // reference it via a stable, well-known path.
    let manifest_dir: String = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .expect("no parent: tests/")
        .parent()
        .expect("no parent: src/")
        .parent()
        .expect("no parent: root");
    let bin_dir = workspace_root.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("failed to create bin directory");
    std::fs::copy(&img_path, bin_dir.join("test-fork-guestfs.img"))
        .expect("failed to copy test-fork-guestfs.img to bin directory");

    println!("cargo:rustc-env=FORK_FD_IMG={}", img_path.display());
    println!("cargo:rerun-if-changed=build.rs");
}

//==================================================================================================
// FAT Image Generation
//==================================================================================================

/// Generates a FAT filesystem image containing the fixture file used by the test.
///
/// Creates a 128 KiB FAT image with a single root-level file, [`FIXTURE_NAME`], holding
/// [`FIXTURE_LEN`] bytes of a deterministic ramp pattern (`byte[i] == i as u8`).
fn generate_fat_image(path: &Path) {
    let size: usize = 128 * 1024;

    // Create zeroed file.
    let buf: Vec<u8> = vec![0u8; size];
    std::fs::write(path, &buf).expect("failed to create FAT image file");

    // Format as FAT.
    {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("failed to open FAT image for formatting");
        let mut storage = fatfs::StdIoWrapper::new(file);
        fatfs::format_volume(&mut storage, fatfs::FormatVolumeOptions::new())
            .expect("failed to format FAT volume");
    }

    // Write the fixture file into the FAT image.
    {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("failed to open FAT image for writing");
        let storage = fatfs::StdIoWrapper::new(file);
        let fs = fatfs::FileSystem::new(storage, fatfs::FsOptions::new())
            .expect("failed to open FAT filesystem");
        let root = fs.root_dir();

        let mut payload: Vec<u8> = vec![0u8; FIXTURE_LEN];
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte = i as u8;
        }

        let mut fixture = root
            .create_file(FIXTURE_NAME)
            .expect("failed to create fixture file");
        fatfs::Write::write_all(&mut fixture, &payload).expect("failed to write fixture file");
        fatfs::Write::flush(&mut fixture).expect("failed to flush fixture file");
    }
}
