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
/// Locates the first occurrence of a character in a string.
///
/// This function returns a pointer to the first occurrence of the character `c` (converted to a
/// `c_char`) in the string pointed to by `s`. The terminating null byte is considered part of the
/// string, so if `c` is `'\0'`, the function returns a pointer to the terminator.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `c`: Character to locate (converted to `c_char`).
///
/// # Return Value
///
/// Returns a pointer to the located character, or a null pointer if the character does not occur
/// in the string.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strchr(s: *const c_char, c: c_int) -> *mut c_char {
    debug_assert!(!s.is_null(), "strchr(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strchr(): pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let p: *const c_uchar = s.cast::<c_uchar>();
    let mut i: usize = 0;
    loop {
        let ch: c_uchar = *p.add(i);
        if ch == target {
            return s.add(i).cast_mut();
        }
        if ch == 0 {
            return core::ptr::null_mut();
        }
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test;
