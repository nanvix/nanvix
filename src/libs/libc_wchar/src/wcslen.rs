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
/// Computes the length of a wide string.
///
/// # Parameters
///
/// - `s`: Pointer to a null-terminated wide string.
///
/// # Return Value
///
/// The number of wide characters preceding the null terminator.
///
/// # Safety
///
/// Behavior is undefined if `s` is null or does not point to a valid null-terminated wide string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcslen(s: *const wchar_t) -> c_size_t {
    debug_assert!(!s.is_null());

    let mut i: c_size_t = 0;
    while unsafe { *s.add(i as usize) } != 0 {
        i += 1;
    }
    i
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
