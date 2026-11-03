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
/// Locates a substring.
///
/// This function finds the first occurrence of the substring `needle` in the string `haystack`.
/// The terminating null bytes are not compared. If `needle` is an empty string, `haystack` is
/// returned.
///
/// # Parameters
///
/// - `haystack`: Pointer to the null-terminated string to search in.
/// - `needle`: Pointer to the null-terminated substring to search for.
///
/// # Return Value
///
/// Returns a pointer to the beginning of the located substring, or a null pointer if the substring
/// is not found.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `haystack` and `needle` without bounds
///   checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    debug_assert!(!haystack.is_null(), "strstr(): null haystack pointer");
    debug_assert!(!needle.is_null(), "strstr(): null needle pointer");
    debug_assert!(
        (haystack as usize).is_multiple_of(align_of::<c_char>()),
        "strstr(): haystack pointer is not properly aligned"
    );
    debug_assert!(
        (needle as usize).is_multiple_of(align_of::<c_char>()),
        "strstr(): needle pointer is not properly aligned"
    );

    // Empty needle: return haystack.
    if *needle == 0 {
        return haystack.cast_mut();
    }

    let mut h: usize = 0;
    while *haystack.add(h) != 0 {
        let mut hi: usize = h;
        let mut ni: usize = 0;
        while *needle.add(ni) != 0 && *haystack.add(hi) == *needle.add(ni) {
            hi += 1;
            ni += 1;
        }
        if *needle.add(ni) == 0 {
            return haystack.add(h).cast_mut();
        }
        h += 1;
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test;
