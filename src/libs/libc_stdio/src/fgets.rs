// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads at most `size - 1` characters from the given file stream into the buffer pointed to by
/// `s`. Reading stops after a newline character (which is stored) or at end-of-file. A null
/// terminator is written after the last character in the buffer.
///
/// # Parameters
///
/// - `s`: Pointer to the buffer where the read string will be stored.
/// - `size`: Maximum number of characters to store (including the null terminator).
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Returns
///
/// `s` on success, or a null pointer if end-of-file is reached before any characters are read
/// or if an error occurs.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `s` points to a buffer of at least `size` bytes.
/// - `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fgets.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fgets(s: *mut c_char, size: c_int, stream: *mut FILE) -> *mut c_char {
    if s.is_null() || stream.is_null() || size <= 0 {
        return core::ptr::null_mut();
    }

    let max: usize = (size - 1) as usize;
    let mut i: usize = 0;

    while i < max {
        let c: c_int = crate::fgetc::fgetc(stream);
        if c == -1 {
            break;
        }
        *s.add(i) = c as c_char;
        i += 1;
        if c == b'\n' as c_int {
            break;
        }
    }

    if i == 0 {
        return core::ptr::null_mut();
    }

    *s.add(i) = 0;
    s
}
