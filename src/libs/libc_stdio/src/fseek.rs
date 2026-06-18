// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    ffi::{
        c_int,
        c_long,
    },
    sys_types::off_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the file position indicator for the stream pointed to by `stream`. The new position,
/// measured in bytes, is obtained by adding `offset` bytes to the position specified by `whence`.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
/// - `offset`: Number of bytes to offset from `whence`.
/// - `whence`: Position from which `offset` is applied (`SEEK_SET`, `SEEK_CUR`, or `SEEK_END`).
///
/// # Returns
///
/// Zero on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fseek.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fseek(stream: *mut FILE, offset: c_long, whence: c_int) -> c_int {
    extern "C" {
        fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    }

    if stream.is_null() {
        return -1;
    }

    let fd: c_int = (*stream).fd;
    // SAFETY: fd comes from a valid FILE, offset and whence are caller-provided.
    let ret: off_t = unsafe { lseek(fd, offset as off_t, whence) };
    if ret < 0 {
        (*stream).error = 1;
        return -1;
    }

    // A successful seek clears the end-of-file indicator and the push-back buffer.
    (*stream).eof = 0;
    (*stream).ungetc_buf = -1;

    0
}
