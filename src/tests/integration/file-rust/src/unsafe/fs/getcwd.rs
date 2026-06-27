// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can get the current working directory using `getcwd()`.
pub fn test() {
    if let Err(error) = unistd::getcwd() {
        panic!("{error:?}");
    };
}
