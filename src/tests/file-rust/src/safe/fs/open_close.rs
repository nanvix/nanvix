// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    RegularFile,
    RegularFileOpenFlags,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can open and close a file.
pub fn test() {
    let pathname: FileSystemPath = match FileSystemPath::new("README.md") {
        Ok(pathname) => pathname,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Open file and assert result.
    let _file: RegularFile =
        match FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None) {
            Ok(file) => file,
            Err(error) => {
                panic!("{error:?}");
            },
        };

    // File is automatically closed when it goes out of scope.
}
