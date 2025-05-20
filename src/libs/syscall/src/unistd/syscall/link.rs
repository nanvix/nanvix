// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    fcntl,
    unistd,
};
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new hard link to an existing file.
///
/// # Parameters
///
/// - `oladpath`: path to the file to be linked.
/// - `newpath`: path to the new file.
///
/// # Returns
///
/// Upon successful completion, `link()` returns empty. Otherwise, it returns an error.
///
pub fn link(oldpath: &str, newpath: &str) -> Result<(), Error> {
    ::syslog::trace!("link(): oldpath = {:?}, newpath = {:?}", oldpath, newpath);
    unistd::linkat(fcntl::AT_FDCWD, oldpath, fcntl::AT_FDCWD, newpath, 0)
}
