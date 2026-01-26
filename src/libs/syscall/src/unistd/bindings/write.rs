// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ErrorCode,
};
use ::core::slice;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes data to a file descriptor. The `write()` function writes up to `count` bytes from the
/// buffer pointed to by `buffer` to the file referred to by the file descriptor `fd`. The number
/// of bytes written may be less than `count` if, for example, there is insufficient space on the
/// underlying physical medium, or the `RLIMIT_FSIZE` resource limit is encountered, or the call
/// was interrupted by a signal handler after having written less than `count` bytes.
///
/// # Parameters
///
/// - `fd`: File descriptor to which data will be written. This must be a valid file descriptor
///   that has been opened for writing or is a standard output descriptor (stdout/stderr).
/// - `buffer`: Pointer to the buffer containing the data to be written. The buffer must contain
///   at least `count` bytes of valid data.
/// - `count`: Number of bytes to write from the buffer. A count of `0` is invalid and will result
///   in an error.
///
/// # Returns
///
/// The `write()` function returns the number of bytes actually written on success. This may be
/// less than `count` if the write was interrupted or if there was insufficient space. On error,
/// it returns `-1` and sets `errno` to indicate the error. Common error conditions include
/// invalid file descriptor, invalid buffer pointer, or insufficient disk space.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buffer` points to a valid memory location containing at least `count` bytes of data.
/// - `buffer` remains valid and readable for the duration of the function call.
/// - `buffer` is properly aligned for byte access.
/// - `fd` refers to a valid, open file descriptor with write permissions.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: c_size_t) -> c_ssize_t {
    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!("write(): invalid buffer (fd={fd:?}, buffer={buffer:?}, count={count:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        ::syslog::error!(
            "write(): invalid write count (fd={fd:?}, buffer={buffer:?}, count={count:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Construct buffer from raw parts.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);

    // Attempt to write to file descriptor and check for errors.
    match crate::unistd::syscall::write(fd, buffer) {
        Ok(bytes_written) => bytes_written as c_ssize_t,
        Err(error) => {
            ::syslog::error!(
                "write(): {error:?} (fd={fd:?}, buffer={:?}, count={count:?})",
                buffer.as_ptr()
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
