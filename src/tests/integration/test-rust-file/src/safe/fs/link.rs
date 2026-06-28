// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    self,
    FileSystem,
    FileSystemPath,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can create a hard link to a file using `link()`.
pub fn test() {
    let filename: &str = "README.md";
    let linkname: &str = "README.link";

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

    // Create a hard link.
    if let Err(error) = safe::fs::link(&filepath, &linkpath) {
        panic!("{error:?}");
    }

    // Get attributes of the original file.
    let original_attr = match FileSystem::get_file_attributes(&filepath) {
        Ok(attr) => attr,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Get attributes of the hard link.
    let link_attr = match FileSystem::get_file_attributes(&linkpath) {
        Ok(attr) => attr,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Check that the hard link has the same file type, permissions, and size.
    if original_attr.file_type() != link_attr.file_type() {
        panic!("file types do not match");
    }
    if original_attr.permissions() != link_attr.permissions() {
        panic!("permissions do not match");
    }
    if original_attr.size() != link_attr.size() {
        panic!("sizes do not match");
    }

    // Remove the hard link.
    if let Err(error) = FileSystem::remove_file(&linkpath) {
        panic!("{error:?}");
    }
}
