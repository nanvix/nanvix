// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
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
            ::syslog::trace!("isatty(): file descriptor is not a terminal (fd={})", fd);
            Ok(false)
        },
        _ => {
            ::syslog::trace!("isatty(): invalid file descriptor (fd={})", fd);
            Err(Error::new(ErrorCode::BadFile, "invalid file descriptor"))
        },
    }
}
