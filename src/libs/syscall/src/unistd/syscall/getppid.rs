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
/// `getppid()` returns the process ID (PID) of the parent of the calling process.
///
pub fn getppid() -> Result<ProcessIdentifier, Error> {
    ::sys::kcall::pm::__kcall_getppid()
}
