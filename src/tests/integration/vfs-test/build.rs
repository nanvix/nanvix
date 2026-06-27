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
    //==============================================================================================
    // Link Archive
    //==============================================================================================

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    //==============================================================================================
    // Generate FAT Image
    //==============================================================================================

    let out_dir: String = env::var("OUT_DIR").expect("OUT_DIR not set");
    let img_path = Path::new(&out_dir).join("test.img");

    generate_fat_image(&img_path);

    // Copy the image to the build-artifacts directory (bin/) so that
    // the test runner can reference it via a stable, well-known path.
    let manifest_dir: String = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .expect("no parent: integration/")
        .parent()
        .expect("no parent: tests/")
        .parent()
        .expect("no parent: src/")
        .parent()
        .expect("no parent: root");
    let bin_dir = workspace_root.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("failed to create bin directory");
    std::fs::copy(&img_path, bin_dir.join("vfs-test.img"))
        .expect("failed to copy test.img to bin directory");

    println!("cargo:rustc-env=VFS_TEST_IMG={}", img_path.display());
    println!("cargo:rerun-if-changed=build.rs");
}

//==================================================================================================
// FAT Image Generation
//==================================================================================================

/// Generates a FAT filesystem image containing test files.
///
/// Creates a 128KB FAT image with:
/// - `hello.txt` containing "Hello from FAT32 on Nanvix!\n"
/// - `subdir/` directory with `nested.txt` inside
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

    // Write test files into the FAT image.
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

        // Create hello.txt in root.
        let mut hello = root
            .create_file("hello.txt")
            .expect("failed to create hello.txt");
        fatfs::Write::write_all(&mut hello, b"Hello from FAT32 on Nanvix!\n")
            .expect("failed to write hello.txt");
        fatfs::Write::flush(&mut hello).expect("failed to flush hello.txt");

        // Create a subdirectory with a nested file.
        root.create_dir("subdir").expect("failed to create subdir");
        let subdir = root.open_dir("subdir").expect("failed to open subdir");
        let mut nested = subdir
            .create_file("nested.txt")
            .expect("failed to create nested.txt");
        fatfs::Write::write_all(&mut nested, b"nested content\n")
            .expect("failed to write nested.txt");
        fatfs::Write::flush(&mut nested).expect("failed to flush nested.txt");
    }
}
