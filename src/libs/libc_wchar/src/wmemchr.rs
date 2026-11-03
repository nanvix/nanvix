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

/// Finds the first occurrence of `c` in the first `n` wide characters of `s`.
///
/// # Safety
///
/// `s` must point to an array of at least `n` wide characters.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wmemchr(s: *const wchar_t, c: wchar_t, n: c_size_t) -> *mut wchar_t {
    debug_assert!(!s.is_null());

    let mut i: c_size_t = 0;
    while i < n {
        if unsafe { *s.add(i as usize) } == c {
            return unsafe { s.add(i as usize).cast_mut() };
        }
        i += 1;
    }
    core::ptr::null_mut()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
