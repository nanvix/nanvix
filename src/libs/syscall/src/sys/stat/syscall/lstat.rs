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
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `stat()` system call obtains information about a file.
///
/// if pathname is a symbolic link, then it returns information about the link itself.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
pub fn lstat(pathname: &str, buf: &mut stat::stat) -> Result<(), Error> {
    ::syslog::trace!("lstat(): pathname = {:?}, statbuf = {:?}", pathname, buf);
    sys::stat::fstatat(fcntl::AT_FDCWD, pathname, buf, fcntl::AT_SYMLINK_NOFOLLOW)
}
