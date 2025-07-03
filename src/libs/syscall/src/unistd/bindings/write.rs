// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ErrorCode,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
    unistd::{
        STDERR_FILENO,
        STDOUT_FILENO,
    },
};
use core::slice;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: c_size_t) -> c_ssize_t {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::syslog::trace!("write(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    }

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!("write(): invalid buffer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        ::syslog::error!("write(): invalid write count");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Construct buffer from raw parts.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);

    // Attempt to write to file descriptor and check for errors.
    match crate::unistd::syscall::write(fd, buffer) {
        Ok(bytes_written) => bytes_written as c_ssize_t,
        Err(error) => {
            ::syslog::error!("write(): failed (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
