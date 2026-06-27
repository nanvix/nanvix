// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::FileSystem;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get the current working directory.
pub fn test() {
    if let Err(error) = FileSystem::get_current_directory() {
        panic!("{error:?}");
    };
}
