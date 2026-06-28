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
    let target_directory: String = "src".to_string();

    // Save the current working directory.
    let saved_current_directory: String = match unistd::getcwd() {
        Ok(path) => path,
        Err(error) => panic!("{error:?}"),
    };

    // Change the current working directory to the target directory.
    if let Err(error) = unistd::chdir(&target_directory) {
        panic!("{error:?}");
    }

    // Ensure that the current working directory has changed.
    match unistd::getcwd() {
        Ok(current_directory) => {
            if current_directory == saved_current_directory {
                panic!("current working directory did not change");
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
