// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::types::mode_t;
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the user and group onwership of a file.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, it returns an error.
///
pub fn chmod(path: &str, mode: mode_t) -> Result<(), Error> {
    ::nvx::trace!("chmod(): path = {:?}, mode = {:?}", path, mode);
    crate::sys::stat::fchmodat(crate::fcntl::AT_FDCWD, path, mode, 0)
}
