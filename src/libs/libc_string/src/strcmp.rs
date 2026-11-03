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
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_uchar,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares two null-terminated strings.
///
/// This function compares the strings pointed to by `s1` and `s2` byte by byte, interpreting each
/// byte as an unsigned char (`c_uchar`). The comparison stops at the first differing byte or when
/// a null terminator is reached.
///
/// # Parameters
///
/// - `s1`: Pointer to the first null-terminated string.
/// - `s2`: Pointer to the second null-terminated string.
///
/// # Return Value
///
/// Returns an integer less than, equal to, or greater than zero if `s1` is found, respectively, to
/// be less than, to match, or be greater than `s2`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s1` and `s2` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
    debug_assert!(!s1.is_null(), "strcmp(): null pointer");
    debug_assert!(!s2.is_null(), "strcmp(): null pointer");
    debug_assert!(
        (s1 as usize).is_multiple_of(align_of::<c_char>()),
        "strcmp(): pointer is not properly aligned"
    );
    debug_assert!(
        (s2 as usize).is_multiple_of(align_of::<c_char>()),
        "strcmp(): pointer is not properly aligned"
    );

    let a: *const c_uchar = s1.cast::<c_uchar>();
    let b: *const c_uchar = s2.cast::<c_uchar>();
    let mut i: usize = 0;
    loop {
        let va: c_uchar = *a.add(i);
        let vb: c_uchar = *b.add(i);
        if va != vb {
            return (va as c_int) - (vb as c_int);
        }
        if va == 0 {
            return 0;
        }
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test;
