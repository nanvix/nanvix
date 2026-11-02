// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Locates the first occurrence of the wide character `c` in the wide string `s`.
///
/// # Parameters
///
/// - `s`: Pointer to a null-terminated wide string.
/// - `c`: Wide character to search for.
///
/// # Return Value
///
/// Returns a pointer to the first occurrence of `c` in `s`, or a null pointer if `c` is not found.
/// If `c` is the null wide character, a pointer to the terminating null is returned.
///
/// # Safety
///
/// Behavior is undefined if `s` is null or does not point to a valid null-terminated wide string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcschr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t {
    debug_assert!(!s.is_null());

    let mut i: c_size_t = 0;
    loop {
        let ch: wchar_t = unsafe { *s.add(i as usize) };
        if ch == c {
            return unsafe { s.add(i as usize) as *mut wchar_t };
        }
        if ch == 0 {
            return ::core::ptr::null_mut();
        }
        i += 1;
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
