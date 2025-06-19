// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Exits the calling process.
///
/// # Parameters
///
/// - `status`: Exit status.
///
/// # Return Values
///
/// Upon successful completion, this function does not return. Upon failure, an error is returned
/// instead.
///
pub fn _exit(status: i32) -> Result<!, Error> {
    ::sys::kcall::pm::exit(status)?;
}
