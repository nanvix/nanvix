// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    wchar_t::wchar_t,
    wcscmp::wcscmp,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Compares two wide-character strings according to the current locale collation order.
///
/// Nanvix currently supports the C/POSIX locale, where collation is equivalent to `wcscmp()`.
///
/// # Safety
///
/// `s1` and `s2` must point to valid null-terminated wide-character strings.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcscoll(s1: *const wchar_t, s2: *const wchar_t) -> c_int {
    unsafe { wcscmp(s1, s2) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
