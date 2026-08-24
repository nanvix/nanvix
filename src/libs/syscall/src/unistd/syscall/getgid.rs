// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::sys_types::gid_t;

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
pub fn getgid() -> Result<gid_t, Error> {
    ::syslog::trace!("getgid()");
    Ok(::sys::pm::GroupIdentifier::ROOT.as_usize() as gid_t)
}
