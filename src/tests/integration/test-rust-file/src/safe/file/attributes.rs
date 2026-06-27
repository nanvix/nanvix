// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileType,
    RegularFile,
    RegularFileOpenFlags,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get attributes of a file.
pub fn test() {
    let pathname: FileSystemPath = match FileSystemPath::new("README.md") {
        Ok(pathname) => pathname,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Open file and assert result.
    let file: RegularFile =
        match FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

    // Get file attributes and assert result.
    match file.attributes() {
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

    // File is automatically closed when it goes out of scope.
}
