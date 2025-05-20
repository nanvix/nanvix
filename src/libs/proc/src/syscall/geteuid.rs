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
/// Returns the effective user ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `geteuid()` returns the effective user ID of the calling process.
/// Otherwise, it returns an error.
///
pub fn geteuid() -> Result<UserIdentifier, Error> {
    unimplemented!("geteuid()");
}
