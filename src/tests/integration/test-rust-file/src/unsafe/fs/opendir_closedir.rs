// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystemPath,
    dir::{
        RawDirectory,
        closedir,
        opendir,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can open and close a directory using `opendir()` and `closedir()`.
pub fn test() {
    let dirname: FileSystemPath = match FileSystemPath::new(".") {
        Ok(path) => path,
        Err(error) => {
            panic!("Failed to create FileSystemPath: {error:?}");
        },
    };

    // Open a directory and assert result.
    let dir: RawDirectory = match opendir(&dirname) {
        Ok(dir) => dir,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Close directory and assert result.
    if let Err(error) = closedir(&dir) {
        panic!("{error:?}");
    }
}
