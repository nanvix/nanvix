// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    self,
    FileSystem,
    FileSystemPath,
    FileType,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can create and read a symbolic link using `symlink()` and `readlink()`.
pub fn test() {
    let filename: &str = "README.md";
    let linkname: &str = "README.symlink";

    let filepath: FileSystemPath = match FileSystemPath::new(filename) {
        Ok(path) => path,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let linkpath: FileSystemPath = match FileSystemPath::new(linkname) {
        Ok(path) => path,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Clean up any stale symlink from a previous run.
    let _ = FileSystem::remove_file(&linkpath);

    // Create a symbolic link.
    if let Err(error) = safe::fs::symlink(&filepath, &linkpath) {
        panic!("{error:?}");
    }

    // Verify the symbolic link itself via lstat().
    match safe::fs::lstat(&linkpath) {
        Ok(attr) => {
            if attr.file_type() != FileType::SymbolicLink {
                panic!("expected symbolic link, got {:?}", attr.file_type());
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Verify the stored path via readlink().
    match safe::fs::readlink(&linkpath) {
        Ok(target) => {
            if target.as_str() != filename {
                panic!(
                    "readlink() returned unexpected target (expected: {filename}, got: {})",
                    target.as_str()
                );
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Verify the symbolic link resolves to the original file via stat().
    let original_attr = match FileSystem::get_file_attributes(&filepath) {
        Ok(attr) => attr,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    let resolved_attr = match FileSystem::get_file_attributes(&linkpath) {
        Ok(attr) => attr,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // stat() follows symlinks, so both should describe the same file.
    if original_attr.file_type() != resolved_attr.file_type() {
        panic!("resolved file type does not match original");
    }
    if original_attr.size() != resolved_attr.size() {
        panic!("resolved file size does not match original");
    }

    // Remove the symbolic link.
    if let Err(error) = FileSystem::remove_file(&linkpath) {
        panic!("{error:?}");
    }
}
