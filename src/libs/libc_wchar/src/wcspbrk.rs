// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Locates the first occurrence in the wide string `ws1` of any wide character from the wide string
/// `ws2`.
///
/// # Parameters
///
/// - `ws1`: Pointer to the null-terminated wide string to search.
/// - `ws2`: Pointer to a null-terminated wide string containing the set of wide characters to
///   match.
///
/// # Return Value
///
/// Returns a pointer to the first wide character in `ws1` that matches any wide character in
/// `ws2`, or a null pointer if no such wide character is found.
///
/// # Safety
///
/// Behavior is undefined if `ws1` or `ws2` is null or does not point to a valid null-terminated
/// wide string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcspbrk(ws1: *const wchar_t, ws2: *const wchar_t) -> *mut wchar_t {
    debug_assert!(!ws1.is_null(), "wcspbrk(): null ws1 pointer");
    debug_assert!(!ws2.is_null(), "wcspbrk(): null ws2 pointer");

    let mut s: *const wchar_t = ws1;
    while unsafe { *s } != 0 {
        let ch: wchar_t = unsafe { *s };
        let mut a: *const wchar_t = ws2;
        while unsafe { *a } != 0 {
            if ch == unsafe { *a } {
                return s.cast_mut();
            }
            a = unsafe { a.add(1) };
        }
        s = unsafe { s.add(1) };
    }

    ::core::ptr::null_mut()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
