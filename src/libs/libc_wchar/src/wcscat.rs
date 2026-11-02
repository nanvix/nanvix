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
/// Appends the wide string `src` to the end of the wide string `dest`, including the null
/// terminator.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination null-terminated wide string.
/// - `src`: Pointer to the source null-terminated wide string.
///
/// # Return Value
///
/// Returns `dest`.
///
/// # Safety
///
/// Behavior is undefined if `dest` or `src` is null, or if the destination buffer is not large
/// enough to hold the concatenated result.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcscat(dest: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t {
    debug_assert!(!dest.is_null());
    debug_assert!(!src.is_null());

    // Find the end of dest.
    let mut d: c_size_t = 0;
    while unsafe { *dest.add(d as usize) } != 0 {
        d += 1;
    }

    // Copy src to the end of dest.
    let mut s: c_size_t = 0;
    loop {
        let c: wchar_t = unsafe { *src.add(s as usize) };
        unsafe { *dest.add(d as usize) = c };
        if c == 0 {
            break;
        }
        d += 1;
        s += 1;
    }
    dest
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
