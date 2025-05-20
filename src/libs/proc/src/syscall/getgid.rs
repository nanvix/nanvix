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
/// Returns the group ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getgid()` returns the group ID of the calling process. Otherwise, it
/// returns an error.
///
pub fn getgid() -> Result<GroupIdentifier, Error> {
    unimplemented!("getgid()");
}
