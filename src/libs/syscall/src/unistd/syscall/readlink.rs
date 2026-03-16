// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::unistd;
use ::sys::error::Error;
use ::sysapi::{
    fcntl::atflags::AT_FDCWD,
    sys_types::c_ssize_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the value of a symbolic link.
///
/// # Parameters
///
/// - `path`: The path to the symbolic link.
/// - `buf`: Storage location for the value of the symbolic link.
///
/// # Returns
///
/// Upon successful completion, `readlink()` returns the number of bytes read. Otherwise, it returns
/// an error.
///
pub fn readlink(path: &str, buf: &mut [u8]) -> Result<c_ssize_t, Error> {
    ::syslog::trace!("readlinkat(): path={path:?}, buf.len={}", buf.len());

    // Delegate to readlinkat(AT_FDCWD, ...) in all modes for consistent error handling.
    unistd::readlinkat(AT_FDCWD, path, buf)
}
