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
/// Returns the effective group ID of the calling process.
///
/// # Returns
///
/// Upon successful completion, `getegid()` returns the effective group ID of the calling process.
/// Otherwise, it returns an error.
///
pub fn getegid() -> Result<gid_t, Error> {
    ::syslog::trace!("getegid()");
    Ok(usize::from(::sys::pm::GroupIdentifier::ROOT) as gid_t)
}
