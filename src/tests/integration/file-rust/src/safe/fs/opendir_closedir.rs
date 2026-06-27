// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    dir::Directory,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can open and close a directory.
pub fn test() {
    let dirname: FileSystemPath = match FileSystemPath::new(".") {
        Ok(path) => path,
        Err(error) => {
            panic!("Failed to create FileSystemPath: {error:?}");
        },
    };

    // Open a directory and assert result.
    let _dir: Directory = match FileSystem::open_directory(&dirname) {
        Ok(dir) => dir,
        Err(error) => {
            panic!("{error:?}");
        },
    };

    // Directory is automatically closed when it goes out of scope.
}
