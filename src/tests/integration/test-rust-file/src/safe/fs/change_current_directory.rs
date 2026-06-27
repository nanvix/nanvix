// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can change the current working directory.
pub fn test() {
    let target_directory: FileSystemPath = match FileSystemPath::new("src") {
        Ok(path) => path,
        Err(error) => panic!("{error:?}"),
    };

    // Save the current working directory.
    let saved_current_directory: FileSystemPath = match FileSystem::get_current_directory() {
        Ok(path) => path,
        Err(error) => panic!("{error:?}"),
    };

    // Change the current working directory to the target directory.
    if let Err(error) = FileSystem::change_current_directory(&target_directory) {
        panic!("{error:?}");
    }

    // Ensure that the current working directory has changed.
    match FileSystem::get_current_directory() {
        Ok(current_directory) => {
            if current_directory == saved_current_directory {
                panic!("current working directory did not change");
            }
        },
        Err(error) => panic!("{error:?}"),
    };

    // Restore the saved current working directory.
    if let Err(error) = FileSystem::change_current_directory(&saved_current_directory) {
        panic!("{error:?}");
    }

    // Ensure that the current working directory is restored.
    match FileSystem::get_current_directory() {
        Ok(current_directory) => {
            if current_directory != saved_current_directory {
                panic!("current working directory is not restored");
            }
        },
        Err(error) => panic!("{error:?}"),
    };
}
