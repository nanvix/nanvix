// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    unistd,
};
use ::sys::error::Error;
use ::sysapi::{
    fcntl::file_access_mode::O_WRONLY,
    ffi::c_int,
    sys_types::{
        mode_t,
        off_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `truncate()` system call causes the regular file named by `path` to be truncated to a size
/// of exactly `length` bytes.
///
/// If the file was previously larger than `length`, the extra data is discarded. If the file was
/// previously shorter than `length`, it is extended, and the extended part reads as null bytes
/// (`\0`).
///
/// # Parameters
///
/// - `path`: Path to the file to be truncated.
/// - `length`: New size of the file.
///
/// # Returns
///
/// Upon successful completion, `truncate()` returns empty. Otherwise, it returns an error.
///
pub fn truncate(path: &str, length: off_t) -> Result<(), Error> {
    ::syslog::trace!("truncate(): path={:?}, length={:?}", path, length);

    // Open the file for writing.
    let fd: c_int = fcntl::open(path, O_WRONLY, 0 as mode_t)?;

    // Truncate the file, then always close the descriptor so it is not leaked on error.
    // `Result::and` keeps the truncate error when truncation fails (the descriptor is still
    // closed), and otherwise surfaces any error from `close()`.
    let result: Result<(), Error> = unistd::ftruncate(fd, length);
    let close_result: Result<(), Error> = unistd::close(fd);

    result.and(close_result)
}
