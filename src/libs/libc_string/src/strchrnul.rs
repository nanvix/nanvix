// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
/// Locates the first occurrence of a character in a string, never returning a null pointer.
///
/// This function behaves like `strchr()`: it returns a pointer to the first occurrence of the
/// character `c` (converted to a `c_char`) in the string pointed to by `s`. Unlike `strchr()`,
/// however, if `c` is not found it returns a pointer to the terminating null byte of `s` rather
/// than a null pointer. It is a GNU extension.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `c`: Character to locate (converted to `c_char`).
///
/// # Return Value
///
/// Returns a pointer to the located character, or a pointer to the terminating null byte of `s` if
/// the character does not occur in the string.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
/// It is safe to call this function if and only if `s` points to a valid, null-terminated string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strchrnul(s: *const c_char, c: c_int) -> *mut c_char {
    debug_assert!(!s.is_null(), "strchrnul(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strchrnul(): pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let p: *const c_uchar = s.cast::<c_uchar>();
    let mut i: usize = 0;
    loop {
        let ch: c_uchar = *p.add(i);
        if ch == target || ch == 0 {
            return s.add(i).cast_mut();
        }
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test;
