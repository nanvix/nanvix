// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys;
use ::sys::error::Error;
use ::sysapi::{
    fcntl::atflags::{
        AT_FDCWD,
        AT_SYMLINK_NOFOLLOW,
    },
    sys_stat,
};

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
pub fn lstat(pathname: &str, buf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("lstat(): pathname = {:?}, statbuf = {:?}", pathname, buf);

    sys::stat::fstatat(AT_FDCWD, pathname, buf, AT_SYMLINK_NOFOLLOW)
}
