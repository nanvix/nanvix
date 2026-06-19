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
mod test {
    use super::*;

    #[test]
    fn test_wcscoll_equal() {
        let s1: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let s2: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        assert_eq!(unsafe { wcscoll(s1.as_ptr(), s2.as_ptr()) }, 0);
    }

    #[test]
    fn test_wcscoll_uses_posix_codepoint_order() {
        let lower: [wchar_t; 2] = [0x61, 0];
        let upper: [wchar_t; 2] = [0x41, 0];
        assert!(unsafe { wcscoll(lower.as_ptr(), upper.as_ptr()) } > 0);
    }
}
