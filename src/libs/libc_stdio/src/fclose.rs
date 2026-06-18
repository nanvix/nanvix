// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::{
    stderr,
    stdin,
    stdout,
    FILE,
};
use ::sysapi::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Flushes and closes the stream associated with `stream`. The underlying file descriptor is
/// closed and the [`FILE`] memory is freed. Standard streams (stdin, stdout, stderr) are not
/// closed or freed.
///
/// # Parameters
///
/// - `stream`: Pointer to the [`FILE`] stream to close.
///
/// # Returns
///
/// Zero on success, or `EOF` (`-1`) on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure previously returned by
/// [`crate::fopen::fopen`].
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fclose.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fclose(stream: *mut FILE) -> c_int {
    extern "C" {
        fn close(fd: c_int) -> c_int;
        fn free(ptr: *mut c_void);
    }

    if stream.is_null() {
        return -1;
    }

    // Do not close or free the standard streams. Identify them by pointer identity rather
    // than by descriptor value, because a regular stream may legitimately own descriptor 0,
    // 1, or 2 if a standard descriptor was previously closed.
    // SAFETY: the accessors return valid pointers to the static standard streams.
    let is_standard: bool = unsafe {
        core::ptr::eq(stream, stdin())
            || core::ptr::eq(stream, stdout())
            || core::ptr::eq(stream, stderr())
    };
    if is_standard {
        return 0;
    }

    let fd: c_int = (*stream).fd;
    let mut result: c_int = 0;

    // SAFETY: fd is a valid open file descriptor.
    if unsafe { close(fd) } != 0 {
        result = -1;
    }

    // SAFETY: stream was allocated by malloc in fopen.
    unsafe { free(stream.cast::<c_void>()) };

    result
}
