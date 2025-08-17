// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileType,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get the attributes of a file.
pub fn test() {
    let filename: FileSystemPath = match FileSystemPath::new("README.md") {
        Ok(filename) => filename,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Get file attributes and assert result.
    match FileSystem::get_file_attributes(&filename) {
        Ok(attr) => {
            // Check if the file is a regular file.
            if attr.file_type() != FileType::RegularFile {
                panic!("file is not a regular file");
            }
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }
}
