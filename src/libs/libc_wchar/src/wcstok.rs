// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `true` if the wide character `c` appears in the null-terminated delimiter set `delim`.
///
/// # Safety
///
/// `delim` must point to a valid, null-terminated wide string.
unsafe fn is_delim(c: wchar_t, delim: *const wchar_t) -> bool {
    let mut d: *const wchar_t = delim;
    while unsafe { *d } != 0 {
        if unsafe { *d } == c {
            return true;
        }
        d = unsafe { d.add(1) };
    }
    false
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Extracts the next token from the wide string `wcs`, delimited by any character in `delim`. The
/// parsing state is kept in `*ptr`, making this function reentrant.
///
/// # Safety
///
/// `delim` must point to a valid, null-terminated wide string, `ptr` must be a valid pointer, and
/// `wcs` (when non-null) must point to a valid, null-terminated wide string.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstok(
    wcs: *mut wchar_t,
    delim: *const wchar_t,
    ptr: *mut *mut wchar_t,
) -> *mut wchar_t {
    let mut s: *mut wchar_t = if wcs.is_null() { unsafe { *ptr } } else { wcs };
    if s.is_null() {
        return core::ptr::null_mut();
    }

    // Skip leading delimiters.
    while unsafe { *s } != 0 && unsafe { is_delim(*s, delim) } {
        s = unsafe { s.add(1) };
    }
    if unsafe { *s } == 0 {
        unsafe { *ptr = s };
        return core::ptr::null_mut();
    }

    let token: *mut wchar_t = s;

    // Find end of token.
    while unsafe { *s } != 0 && !unsafe { is_delim(*s, delim) } {
        s = unsafe { s.add(1) };
    }
    if unsafe { *s } != 0 {
        unsafe { *s = 0 };
        s = unsafe { s.add(1) };
    }
    unsafe { *ptr = s };
    token
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
