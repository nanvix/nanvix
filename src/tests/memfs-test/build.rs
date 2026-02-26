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
    path::Path,
};

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    // Link with the Nanvix user linker script.
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/x86/user.ld");

    // Generate a FAT image with a test file.
    let manifest_dir: String = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let img_path = Path::new(&manifest_dir).join("test.fat");
    generate_fat_image(&img_path);

    println!("cargo:rerun-if-changed=build.rs");
}

//==================================================================================================
// FAT Image Generation
//==================================================================================================

/// Generates a FAT filesystem image containing test files for POSIX interception
/// validation.
///
/// Creates a 128KB FAT image with:
/// - `test.txt` containing "Hello from memfs!\n" (18 bytes).
/// - `data.bin` containing 4096 bytes of repeating pattern (0x00..0xFF * 16).
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

    // Write test files.
    {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("failed to open FAT image for writing");
        let storage = fatfs::StdIoWrapper::new(file);
        let fs =
            fatfs::FileSystem::new(storage, fatfs::FsOptions::new()).expect("failed to open FAT");
        let root = fs.root_dir();

        // Create test.txt.
        let mut txt = root
            .create_file("test.txt")
            .expect("failed to create test.txt");
        fatfs::Write::write_all(&mut txt, b"Hello from memfs!\n")
            .expect("failed to write test.txt");
        fatfs::Write::flush(&mut txt).expect("failed to flush test.txt");

        // Create data.bin with a known pattern.
        let mut bin = root
            .create_file("data.bin")
            .expect("failed to create data.bin");
        let pattern: Vec<u8> = (0..4096).map(|i| (i & 0xFF) as u8).collect();
        fatfs::Write::write_all(&mut bin, &pattern).expect("failed to write data.bin");
        fatfs::Write::flush(&mut bin).expect("failed to flush data.bin");
    }
}
