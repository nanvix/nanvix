// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
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
/// Reads data from a file descriptor. The `read()` function reads up to `count` bytes from the
/// file referred to by the file descriptor `fd` into the buffer pointed to by `buffer`. The
/// number of bytes read may be less than `count` if, for example, fewer bytes are available
/// in the file, the read was interrupted by a signal, or the file is a pipe or FIFO and has
/// insufficient data immediately available. A return value of `0` indicates end-of-file.
///
/// # Parameters
///
/// - `fd`: File descriptor from which data will be read. This must be a valid file descriptor
///   that has been opened for reading or is a standard input descriptor (stdin).
/// - `buffer`: Pointer to the buffer where the read data will be stored. The buffer must be
///   large enough to hold `count` bytes.
/// - `count`: Maximum number of bytes to read from the file descriptor. A count of `0` is
///   valid and will return `0` immediately without performing any read operation.
///
/// # Returns
///
/// The `read()` function returns the number of bytes actually read on success. This may be
/// less than `count` if fewer bytes are available or if the read was interrupted. On error,
/// it returns `-1` and sets `errno` to indicate the error. A return value of `0` indicates
/// end-of-file. Common error conditions include invalid file descriptor, invalid buffer
/// pointer, or interrupted system call.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buffer` points to a valid memory location with space for at least `count` bytes.
/// - `buffer` remains valid and writable for the duration of the function call.
/// - `buffer` is properly aligned for byte access.
/// - `fd` refers to a valid, open file descriptor with read permissions.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn read(fd: c_int, buffer: *mut c_void, count: c_size_t) -> c_ssize_t {
    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!("read(): invalid buffer (fd={fd:?}, buffer={buffer:?}, count={count:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is zero.
    if count == 0 {
        return 0;
    }

    // Construct buffer from raw parts.
    let buffer: &mut [u8] =
        unsafe { ::core::slice::from_raw_parts_mut(buffer as *mut u8, count as usize) };

    // Attempt to read from the file descriptor and check for errors.
    match crate::unistd::read(fd, buffer) {
        Ok(bytes_read) => bytes_read as c_ssize_t,
        Err(error) => {
            ::syslog::error!(
                "read(): failed (error={error:?}, fd={fd:?}, buffer={:?}, count={count:?})",
                buffer.as_ptr()
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
