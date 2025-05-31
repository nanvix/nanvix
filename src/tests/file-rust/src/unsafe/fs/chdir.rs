// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::{
    String,
    ToString,
};
use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can change the current working directory using `chdir()`.
pub fn test() {
    let root_directory: String = "/".to_string();

    // Save the current working directory.
    let saved_current_directory: String = match unistd::getcwd() {
        Ok(path) => path,
        Err(error) => panic!("{error:?}"),
    };

    // Ensure that current working directory is not the root directory.
    if saved_current_directory == root_directory {
        panic!("current working directory is already the root directory");
    }

    // Change the current working directory to the root directory.
    if let Err(error) = unistd::chdir(&root_directory) {
        panic!("{error:?}");
    }

    // Ensure that the current working directory is now the root directory.
    match unistd::getcwd() {
        Ok(current_directory) => {
            if current_directory != root_directory {
                panic!("current working directory is not the root directory");
            }
        },
        Err(error) => panic!("{error:?}"),
    };

    // Restore the saved current working directory.
    if let Err(error) = unistd::chdir(&saved_current_directory) {
        panic!("{error:?}");
    }

    // Ensure that the current working directory is restored.
    match unistd::getcwd() {
        Ok(current_directory) => {
            if current_directory != saved_current_directory {
                panic!("current working directory is not restored");
            }
        },
        Err(error) => panic!("{error:?}"),
    };
}
