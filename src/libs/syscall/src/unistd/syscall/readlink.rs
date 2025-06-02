// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    fcntl,
    sys::types::ssize_t,
    unistd,
};
use ::sys::error::Error;

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
pub fn readlink(path: &str, buf: &mut [u8]) -> Result<ssize_t, Error> {
    ::syslog::trace!("readlinkat(): path={path:?}, buf.len={}", buf.len());
    unistd::readlinkat(fcntl::AT_FDCWD, path, buf)
}
