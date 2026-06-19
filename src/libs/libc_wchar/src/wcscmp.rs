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
use ::sysapi::{
    ffi::c_int,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares two wide strings lexicographically.
///
/// # Parameters
///
/// - `s1`: Pointer to the first null-terminated wide string.
/// - `s2`: Pointer to the second null-terminated wide string.
///
/// # Return Value
///
/// Returns a negative value if `s1` is less than `s2`, zero if they are equal, or a positive value
/// if `s1` is greater than `s2`.
///
/// # Safety
///
/// Behavior is undefined if `s1` or `s2` is null or does not point to a valid null-terminated wide
/// string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcscmp(s1: *const wchar_t, s2: *const wchar_t) -> c_int {
    debug_assert!(!s1.is_null());
    debug_assert!(!s2.is_null());

    let mut i: c_size_t = 0;
    loop {
        let c1: wchar_t = unsafe { *s1.add(i as usize) };
        let c2: wchar_t = unsafe { *s2.add(i as usize) };
        let diff: i64 = i64::from(c1) - i64::from(c2);
        if diff != 0 || c1 == 0 {
            return diff.signum() as c_int;
        }
        i += 1;
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcscmp_equal() {
        let s1: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
        let s2: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
        assert_eq!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) }, 0);
    }

    #[test]
    fn test_wcscmp_less() {
        let s1: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
        let s2: [wchar_t; 4] = [0x41, 0x42, 0x44, 0];
        assert!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) } < 0);
    }

    #[test]
    fn test_wcscmp_greater() {
        let s1: [wchar_t; 4] = [0x41, 0x42, 0x44, 0];
        let s2: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
        assert!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) } > 0);
    }

    #[test]
    fn test_wcscmp_prefix() {
        let s1: [wchar_t; 3] = [0x41, 0x42, 0];
        let s2: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
        assert!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) } < 0);
    }
}
