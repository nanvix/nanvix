// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::{
    ffi::c_char,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Appends at most `n` bytes from a null-terminated string to another.
///
/// This function appends at most `n` bytes from the string pointed to by `src` to the end of the
/// string pointed to by `dest`, overwriting the null terminator at the end of `dest`, and then
/// adding a terminating null byte.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination null-terminated string.
/// - `src`: Pointer to the source null-terminated string.
/// - `n`: Maximum number of bytes to append from `src`.
///
/// # Return Value
///
/// Returns the original destination pointer `dest`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It writes to the memory region pointed to by `dest` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strncat(
    dest: *mut c_char,
    src: *const c_char,
    n: c_size_t,
) -> *mut c_char {
    debug_assert!(!dest.is_null(), "strncat(): null destination pointer");
    debug_assert!(!src.is_null(), "strncat(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strncat(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strncat(): source pointer is not properly aligned"
    );

    let n: usize = n as usize;

    // Find the end of dest.
    let mut d: usize = 0;
    while *dest.add(d) != 0 {
        d += 1;
    }

    // Append at most n bytes from src.
    let mut s: usize = 0;
    while s < n {
        let c: c_char = *src.add(s);
        if c == 0 {
            break;
        }
        *dest.add(d) = c;
        d += 1;
        s += 1;
    }

    // Always null-terminate.
    *dest.add(d) = 0 as c_char;

    dest
}

#[cfg(all(test, feature = "std"))]
mod test;
