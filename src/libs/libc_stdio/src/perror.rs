// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
// Constants
//==================================================================================================

/// File descriptor for standard error.
const STDERR_FD: c_int = 2;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Prints an error message to standard error. If `s` is not null and does not point to a null
/// byte, the string `s` is printed followed by a colon and a space. A generic error message is
/// then printed, followed by a newline.
///
/// # Note
///
/// A per-`errno` message string is not emitted because `errno`-to-string mapping is not available
/// in this `no_std` environment; a fixed generic message is used instead.
///
/// # Parameters
///
/// - `s`: Optional pointer to a null-terminated prefix string. May be null.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that,
/// if `s` is non-null, it points to a valid, null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/perror.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn perror(s: *const c_char) {
    extern "C" {
        fn write(fd: c_int, buf: *const c_void, count: c_size_t) -> c_ssize_t;
    }

    // Write prefix string if provided.
    if !s.is_null() {
        let first_byte: c_char = *s;
        if first_byte != 0 {
            // Compute string length.
            let mut len: usize = 0;
            while *s.add(len) != 0 {
                len += 1;
            }
            let _ = write(STDERR_FD, s.cast::<c_void>(), len as c_size_t);
            let sep: &[u8] = b": ";
            let _ = write(STDERR_FD, sep.as_ptr().cast::<c_void>(), sep.len() as c_size_t);
        }
    }

    // Write a generic error message (errno-to-string mapping is not available in no_std).
    let msg: &[u8] = b"error\n";
    let _ = write(STDERR_FD, msg.as_ptr().cast::<c_void>(), msg.len() as c_size_t);
}
