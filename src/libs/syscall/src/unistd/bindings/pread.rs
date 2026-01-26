// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};

use ::core::slice;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
        off_t,
    },
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads data from a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to read into.
/// - `count`: Number of bytes to read.
/// - `offset`: Offset to read from.
///
/// # Returns
///
/// Upon successful completion, `pread()` returns the number of bytes read. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// The function is unsafe because:
/// - It may dereference pointers.
/// - It may access global variables.
///
/// It is safe to use this function if the following conditions are met:
/// - `buffer` points to a buffer of `count` bytes.
/// - This function is not called from multiple threads at the same time.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pread(
    fd: c_int,
    buffer: *mut c_void,
    count: c_size_t,
    offset: off_t,
) -> c_ssize_t {
    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!(
            "pread(): invalid buffer (fd={fd:?}, buffer={buffer:?}, count={count:?}, \
             offset={offset:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        return 0;
    }

    // Attempt to convert `buffer`.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(buffer as *mut u8, count as usize);

    // Attempt to read from the file descriptor and check for errors.
    match unistd::pread(fd, buffer, offset) {
        Ok(bytes_read) => bytes_read as c_ssize_t,
        Err(error) => {
            ::syslog::error!(
                "pread(): {error:?}, (fd={fd:?}, buffer={buffer:?}, count={count:?}, \
                 offset={offset:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
