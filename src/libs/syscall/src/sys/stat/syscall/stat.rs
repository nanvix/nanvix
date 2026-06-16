// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys;
use ::sys::error::Error;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
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
pub fn stat(pathname: &str, statbuf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("stat(): pathname = {:?}", pathname);

    sys::stat::fstatat(AT_FDCWD, pathname, statbuf, 0)
}
