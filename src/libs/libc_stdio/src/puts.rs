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

/// File descriptor for standard output.
const STDOUT_FD: c_int = 1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the string `s` followed by a newline character to standard output.
///
/// # Parameters
///
/// - `s`: Pointer to a null-terminated string to be written.
///
/// # Returns
///
/// A non-negative value on success, or `EOF` (`-1`) on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that
/// `s` points to a valid, null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/puts.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn puts(s: *const c_char) -> c_int {
    extern "C" {
        fn write(fd: c_int, buf: *const c_void, count: c_size_t) -> c_ssize_t;
    }

    if s.is_null() {
        return -1;
    }

    // Compute string length.
    let mut len: usize = 0;
    // SAFETY: s is non-null; we walk until the null terminator.
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }

    // Write string.
    if len > 0 {
        // SAFETY: s is valid for len bytes.
        let ret: c_ssize_t = unsafe { write(STDOUT_FD, s.cast::<c_void>(), len as c_size_t) };
        if ret < 0 {
            return -1;
        }
    }

    // Write trailing newline.
    let nl: u8 = b'\n';
    // SAFETY: nl is a valid 1-byte value on the stack.
    let ret: c_ssize_t =
        unsafe { write(STDOUT_FD, (&raw const nl).cast::<c_void>(), 1 as c_size_t) };
    if ret < 0 {
        return -1;
    }

    0
}
