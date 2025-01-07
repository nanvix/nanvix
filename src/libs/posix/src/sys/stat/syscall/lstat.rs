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
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Upon failure, a negative error code is returned
/// instead.
///
pub fn lstat(pathname: &str, statbuf: &mut stat::stat) -> i32 {
    ::nvx::log!("lstat(): pathname = {:?}, statbuf = {:?}", pathname, statbuf);
    sys::stat::fstatat(fcntl::AT_FDCWD, pathname, statbuf, fcntl::AT_SYMLINK_NOFOLLOW)
}
