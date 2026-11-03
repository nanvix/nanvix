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
    ffi::{
        c_int,
        c_uchar,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares bytes in memory.
///
/// This function compares the first `len` bytes of the memory areas pointed to by `ptr1` and
/// `ptr2`. The comparison is performed byte by byte, interpreting each byte as an unsigned char
/// (`c_uchar`).
///
/// The function stops at the first differing byte or after `len` bytes have been compared.
///
/// # Parameters
///
/// - `ptr1`: Pointer to the first memory region to compare.
/// - `ptr2`: Pointer to the second memory region to compare.
/// - `len`: Number of bytes to compare.
///
/// # Return Value
///
/// This function returns an integer less than, equal to, or greater than zero if the first
/// differing byte in `ptr1` is respectively less than, equal to, or greater than the corresponding
/// byte in `ptr2`.  If all `len` bytes are equal, returns `0`.
///
/// The exact magnitude of the non-zero return value is the arithmetic difference between the
/// first pair of differing bytes interpreted as `c_uchar` and then cast to `c_int`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `ptr1` and `ptr2` without bounds checking.
///
/// It is safe to call this function if and only if all of the following conditions are met:
/// - `ptr1` points to a readable memory region of at least `len` bytes.
/// - `ptr2` points to a readable memory region of at least `len` bytes.
/// - `len` does not exceed `isize::MAX`.
/// - Both pointers are properly aligned for `c_uchar` access.
///
/// Violating any of these conditions results in undefined behavior.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memcmp(ptr1: *const c_void, ptr2: *const c_void, len: c_size_t) -> c_int {
    debug_assert!(!ptr1.is_null(), "memcmp(): null pointer");
    debug_assert!(!ptr2.is_null(), "memcmp(): null pointer");
    debug_assert!((len as usize) < isize::MAX as usize, "memcmp(): length too large");
    debug_assert!(
        (ptr1 as usize).is_multiple_of(align_of::<c_uchar>()),
        "memcmp(): pointer is not properly aligned"
    );
    debug_assert!(
        (ptr2 as usize).is_multiple_of(align_of::<c_uchar>()),
        "memcmp(): pointer is not properly aligned"
    );

    let a: *const c_uchar = ptr1.cast::<c_uchar>();
    let b: *const c_uchar = ptr2.cast::<c_uchar>();
    let len: usize = len as usize;
    let mut i: usize = 0;
    while i < len {
        let va: c_uchar = *a.add(i);
        let vb: c_uchar = *b.add(i);
        if va != vb {
            return (va as c_int) - (vb as c_int);
        }
        i += 1;
    }
    0
}

#[cfg(all(test, feature = "std"))]
mod test;
