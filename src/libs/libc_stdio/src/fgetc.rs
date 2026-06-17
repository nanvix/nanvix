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
/// Reads the next character from the given file stream and returns it as an `unsigned char` cast
/// to [`c_int`], or `EOF` (`-1`) on end-of-file or error. If a character was pushed back via
/// [`crate::ungetc::ungetc`], it is returned first.
///
/// # Parameters
///
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Returns
///
/// The next character as an `unsigned char` promoted to [`c_int`], or `EOF` (`-1`) on end-of-file
/// or error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fgetc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fgetc(stream: *mut FILE) -> c_int {
    extern "C" {
        fn read(fd: c_int, buf: *mut c_void, count: c_size_t) -> c_ssize_t;
    }

    if stream.is_null() {
        return -1;
    }

    // Return the pushed-back character if present.
    if (*stream).ungetc_buf != -1 {
        let c: c_int = (*stream).ungetc_buf;
        (*stream).ungetc_buf = -1;
        return c;
    }

    let fd: c_int = (*stream).fd;
    let mut byte: u8 = 0;
    // SAFETY: byte is a valid 1-byte buffer on the stack, fd comes from a valid FILE.
    let ret: c_ssize_t = unsafe { read(fd, (&raw mut byte).cast::<c_void>(), 1 as c_size_t) };

    if ret < 0 {
        (*stream).error = 1;
        -1
    } else if ret == 0 {
        (*stream).eof = 1;
        -1
    } else {
        byte as c_int
    }
}

///
/// # Description
///
/// Equivalent to [`fgetc`]. Reads the next character from the given file stream.
///
/// # Parameters
///
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Returns
///
/// The next character as an `unsigned char` promoted to [`c_int`], or `EOF` (`-1`) on end-of-file
/// or error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getc(stream: *mut FILE) -> c_int {
    fgetc(stream)
}
