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
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Searches a string for any of a set of bytes.
///
/// This function locates the first occurrence in the string `s` of any byte in the string `accept`.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `accept`: Pointer to a null-terminated string of bytes to search for.
///
/// # Return Value
///
/// Returns a pointer to the byte in `s` that matches one of the bytes in `accept`, or a null
/// pointer if no such byte is found.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s` and `accept` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strpbrk(s: *const c_char, accept: *const c_char) -> *mut c_char {
    debug_assert!(!s.is_null(), "strpbrk(): null pointer");
    debug_assert!(!accept.is_null(), "strpbrk(): null accept pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strpbrk(): pointer is not properly aligned"
    );
    debug_assert!(
        (accept as usize).is_multiple_of(align_of::<c_char>()),
        "strpbrk(): accept pointer is not properly aligned"
    );

    let mut i: usize = 0;
    while *s.add(i) != 0 {
        let ch: c_char = *s.add(i);
        let mut a: usize = 0;
        while *accept.add(a) != 0 {
            if ch == *accept.add(a) {
                return s.add(i).cast_mut();
            }
            a += 1;
        }
        i += 1;
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test;
