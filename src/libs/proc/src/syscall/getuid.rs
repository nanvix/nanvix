// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::Error,
    pm::UserIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the user ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getuid()` returns the user ID of the calling process. Otherwise, it
/// returns an error.
///
pub fn getuid() -> Result<UserIdentifier, Error> {
    unimplemented!("getuid()");
}
