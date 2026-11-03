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
/// Calculates the length of a string, examining at most `maxlen` bytes.
///
/// This function computes the length of the string pointed to by `s`, excluding the terminating
/// null byte (`'\0'`), but examines at most `maxlen` bytes.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string whose length is to be calculated.
/// - `maxlen`: Maximum number of bytes to examine.
///
/// # Return Value
///
/// Returns the number of characters in the string pointed to by `s`, excluding the terminating
/// null byte, or `maxlen` if no null byte is found within that many bytes.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strnlen(s: *const c_char, maxlen: c_size_t) -> c_size_t {
    debug_assert!(!s.is_null(), "strnlen(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strnlen(): pointer is not properly aligned"
    );

    let mut i: c_size_t = 0;
    while i < maxlen && *s.add(i as usize) != 0 {
        i += 1;
    }
    i
}

#[cfg(all(test, feature = "std"))]
mod test;
