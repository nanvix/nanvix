// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{pm::ProcessIdentifier, sys::error::Error};

//==================================================================================================
// Standalone Functions
//==================================================================================================///

/// # Description
///
/// `getpid()` returns the process ID (PID) of the calling process.
///
pub fn getpid() -> Result<ProcessIdentifier, Error> {
    ::nvx::log!("getpid()");
    ::nvx::sys::kcall::pm::getpid()
}
