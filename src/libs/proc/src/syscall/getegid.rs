// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::Error,
    pm::GroupIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the effective group ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getegid()` returns the effective group ID of the calling process.
/// Otherwise, it returns an error.
///
pub fn getegid() -> Result<GroupIdentifier, Error> {
    unimplemented!("getegid()");
}
