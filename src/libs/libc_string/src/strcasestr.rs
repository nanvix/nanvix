// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::ffi::c_char;

//==================================================================================================
// Helpers
//==================================================================================================

/// Folds an ASCII upper-case byte to lower case.
#[inline]
fn to_lower(b: u8) -> u8 {
    if b.is_ascii_uppercase() {
        b + 32
    } else {
        b
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Locates the first case-insensitive occurrence of a substring in a string.
///
/// This function behaves like `strstr()` but ignores the case of ASCII letters when matching: it
/// returns a pointer to the first occurrence within the string pointed to by `haystack` of the
/// sequence of characters in the string pointed to by `needle`, comparing letters without regard
/// to case. It is a GNU extension.
///
/// # Parameters
///
/// - `haystack`: Pointer to the null-terminated string to be searched.
/// - `needle`: Pointer to the null-terminated substring to search for.
///
/// # Return Value
///
/// Returns a pointer to the beginning of the located substring, or a null pointer if the substring
/// is not found. If `needle` points to an empty string, `haystack` is returned.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `haystack` and `needle` without bounds
///   checking.
///
/// It is safe to call this function if and only if both `haystack` and `needle` point to valid,
/// null-terminated strings.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcasestr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    debug_assert!(!haystack.is_null(), "strcasestr(): null haystack pointer");
    debug_assert!(!needle.is_null(), "strcasestr(): null needle pointer");
    debug_assert!(
        (haystack as usize).is_multiple_of(align_of::<c_char>()),
        "strcasestr(): haystack pointer is not properly aligned"
    );
    debug_assert!(
        (needle as usize).is_multiple_of(align_of::<c_char>()),
        "strcasestr(): needle pointer is not properly aligned"
    );

    // Empty needle: return haystack.
    if *needle == 0 {
        return haystack.cast_mut();
    }

    let mut h: usize = 0;
    while *haystack.add(h) != 0 {
        let mut hi: usize = h;
        let mut ni: usize = 0;
        while *needle.add(ni) != 0
            && to_lower(*haystack.add(hi) as u8) == to_lower(*needle.add(ni) as u8)
        {
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
