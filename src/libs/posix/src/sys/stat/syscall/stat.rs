// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    sys,
    sys::stat,
};
use ::nvx::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `stat()` system call obtains information about a file.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
pub fn stat(pathname: &str, statbuf: &mut stat::stat) -> Result<(), Error> {
    ::nvx::trace!("stat(): pathname = {:?}", pathname);
    sys::stat::fstatat(fcntl::AT_FDCWD, pathname, statbuf, 0)
}
