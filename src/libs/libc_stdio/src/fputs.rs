// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::{
    ffi::{
        c_char,
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
/// Writes the string `s` to the given file stream. Unlike [`crate::puts::puts`], no trailing
/// newline is appended.
///
/// # Parameters
///
/// - `s`: Pointer to a null-terminated string to be written.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// A non-negative value on success, or `EOF` (`-1`) on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `s` points to a valid, null-terminated string.
/// - `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fputs.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fputs(s: *const c_char, stream: *mut FILE) -> c_int {
    extern "C" {
        fn write(fd: c_int, buf: *const c_void, count: c_size_t) -> c_ssize_t;
    }

    if s.is_null() || stream.is_null() {
        return -1;
    }

    let fd: c_int = (*stream).fd;

    // Compute string length.
    let mut len: usize = 0;
    // SAFETY: s is non-null; we walk until the null terminator.
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }

    if len == 0 {
        return 0;
    }

    // Loop until every byte is written: a single `write` may transfer fewer bytes than requested
    // without an error, and `fputs` must not stop short in that case.
    let mut offset: usize = 0;
    while offset < len {
        // SAFETY: s is valid for len bytes, fd comes from a valid FILE.
        let ret: c_ssize_t = unsafe {
            write(fd, s.cast::<u8>().add(offset).cast::<c_void>(), (len - offset) as c_size_t)
        };
        // A negative return is an error; a zero return makes no forward progress. In either case
        // POSIX requires the stream's error indicator to be set and EOF returned.
        if ret <= 0 {
            (*stream).error = 1;
            return -1;
        }
        offset += ret as usize;
    }

    0
}

///
/// # Description
///
/// Non-locking variant of [`fputs`]. Nanvix streams are single-threaded, so this is exactly
/// equivalent to [`fputs`].
///
/// # Parameters
///
/// - `s`: NUL-terminated string to write.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// A non-negative number on success, or `EOF` (`-1`) on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `s` points to a valid NUL-terminated string and that `stream` is a valid [`FILE`].
///
/// # References
///
/// - <https://man7.org/linux/man-pages/man3/unlocked_stdio.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fputs_unlocked(s: *const c_char, stream: *mut FILE) -> c_int {
    fputs(s, stream)
}
