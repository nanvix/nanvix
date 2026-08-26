// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::sys_types::uid_t;

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
pub fn geteuid() -> Result<uid_t, Error> {
    ::syslog::trace!("geteuid()");
    Ok(::sys::pm::UserIdentifier::ROOT.as_usize() as uid_t)
}
