// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    RegularFile,
    RegularFileAdvice,
    RegularFileOffset,
    RegularFileOpenFlags,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can provide advice about the use of a regular file.
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

    // Give advice about file access amd assert result.
    if let Err(error) = file.advise(
        RegularFileOffset::from(0),
        RegularFileOffset::from(0),
        RegularFileAdvice::sequential_access(),
    ) {
        panic!("{error:?}");
    }

    // File is automatically closed when it goes out of scope.
}
