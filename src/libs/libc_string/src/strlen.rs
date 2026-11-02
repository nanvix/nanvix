// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_char,
        c_uchar,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Calculates the length of a string.
///
/// This function computes the length of the string pointed to by `s`, excluding the terminating
/// null byte (`'\0'`).
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string whose length is to be calculated.
///
/// # Return Value
///
/// This function returns the number of characters in the string pointed to by `s`, excluding the
/// terminating null byte. If `s` is a null pointer, the behavior is undefined.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strlen(s: *const c_char) -> c_size_t {
    debug_assert!(!s.is_null(), "strlen(): null pointer");

    let bytes: *const c_uchar = s.cast::<c_uchar>();
    let mut i: usize = 0;

    // Scan one byte at a time so that no memory beyond the terminating null byte is ever read.
    while *bytes.add(i) != 0 {
        i += 1;
    }

    c_size_t::try_from(i).unwrap_or(c_size_t::MAX)
}

#[cfg(all(test, feature = "std"))]
mod test;
