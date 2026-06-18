// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the character `c` (converted to `unsigned char`) to the given file stream.
///
/// # Parameters
///
/// - `c`: The character to write, passed as a [`c_int`].
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// The character written as an `unsigned char` cast to [`c_int`], or `EOF` (`-1`) on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fputc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fputc(c: c_int, stream: *mut FILE) -> c_int {
    extern "C" {
        fn write(fd: c_int, buf: *const c_void, count: c_size_t) -> c_ssize_t;
    }

    if stream.is_null() {
        return -1;
    }

    let fd: c_int = (*stream).fd;
    let byte: u8 = c as u8;
    // SAFETY: byte is a valid 1-byte value on the stack, fd comes from a valid FILE.
    let ret: c_ssize_t = unsafe { write(fd, (&raw const byte).cast::<c_void>(), 1 as c_size_t) };
    if ret < 0 {
        // POSIX: on a write error the error indicator for the stream shall be set.
        (*stream).error = 1;
        -1
    } else {
        byte as c_int
    }
}
