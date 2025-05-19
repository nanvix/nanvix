// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::{
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks if the file descriptor is a terminal.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, a boolean indicating whether the file descriptor is a terminal is
/// returned. Otherwise, an error is returned.
///
pub fn isatty(fd: RawFileDescriptor) -> Result<bool, Error> {
    ::syslog::trace!("isatty(): fd={}", fd);

    match fd {
        STDIN_FILENO | STDOUT_FILENO | STDERR_FILENO => Ok(true),
        fd if fd > 0 => {
            ::syslog::error!("isatty(): file descriptor is not a terminal (fd={})", fd);
            Ok(false)
        },
        _ => {
            ::syslog::error!("isatty(): invalid file descriptor (fd={})", fd);
            Err(Error::new(ErrorCode::BadFile, "invalid file descriptor"))
        },
    }
}
