// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use ::std::path::Path;

//==================================================================================================
// Public API
//==================================================================================================

/// Creates a zeroed file of `size` bytes at `path` and formats it as a FAT filesystem.
pub fn mkfatfs(path: &Path, size: usize) {
    let buf: Vec<u8> = vec![0u8; size];
    std::fs::write(path, &buf).expect("failed to create FAT image file");

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("failed to open FAT image for formatting");
    let mut storage = fatfs::StdIoWrapper::new(file);
    fatfs::format_volume(&mut storage, fatfs::FormatVolumeOptions::new())
        .expect("failed to format FAT volume");
}
