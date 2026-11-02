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
/// Copies at most `n` bytes from a null-terminated string.
///
/// This function copies at most `n` bytes from the string pointed to by `src` to the buffer
/// pointed to by `dest`. If the length of `src` is less than `n`, the remainder of `dest` is
/// padded with null bytes. If the length of `src` is greater than or equal to `n`, `dest` is NOT
/// null-terminated.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination buffer.
/// - `src`: Pointer to the source null-terminated string.
/// - `n`: Maximum number of bytes to copy.
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
pub unsafe extern "C" fn strncpy(
    dest: *mut c_char,
    src: *const c_char,
    n: c_size_t,
) -> *mut c_char {
    debug_assert!(!dest.is_null(), "strncpy(): null destination pointer");
    debug_assert!(!src.is_null(), "strncpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strncpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strncpy(): source pointer is not properly aligned"
    );

    let n: usize = n as usize;
    let mut i: usize = 0;

    // Copy bytes from src until null terminator or n bytes.
    while i < n {
        let c: c_char = *src.add(i);
        *dest.add(i) = c;
        if c == 0 {
            i += 1;
            break;
        }
        i += 1;
    }

    // Pad remainder with null bytes.
    while i < n {
        *dest.add(i) = 0 as c_char;
        i += 1;
    }

    dest
}

#[cfg(all(test, feature = "std"))]
mod test;
