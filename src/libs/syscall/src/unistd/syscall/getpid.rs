// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::Error,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================///

///
/// # Description
///
/// `getpid()` returns the process ID (PID) of the calling process.
///
pub fn getpid() -> Result<ProcessIdentifier, Error> {
    ::sys::kcall::pm::getpid()
}
