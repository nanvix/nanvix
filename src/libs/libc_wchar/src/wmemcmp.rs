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
/// Compares `n` wide characters from the arrays `s1` and `s2`.
///
/// # Parameters
///
/// - `s1`: Pointer to the first wide character array.
/// - `s2`: Pointer to the second wide character array.
/// - `n`: Number of wide characters to compare.
///
/// # Return Value
///
/// Returns a negative value if `s1` is less than `s2`, zero if they are equal, or a positive value
/// if `s1` is greater than `s2`.
///
/// # Safety
///
/// Behavior is undefined if `s1` or `s2` is null, or if either buffer has fewer than `n` wide
/// character positions.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wmemcmp(s1: *const wchar_t, s2: *const wchar_t, n: c_size_t) -> c_int {
    debug_assert!(!s1.is_null());
    debug_assert!(!s2.is_null());

    let mut i: c_size_t = 0;
    while i < n {
        let c1: wchar_t = unsafe { *s1.add(i as usize) };
        let c2: wchar_t = unsafe { *s2.add(i as usize) };
        let diff: i64 = i64::from(c1) - i64::from(c2);
        if diff != 0 {
            return diff.signum() as c_int;
        }
        i += 1;
    }
    0
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wmemcmp_equal() {
        let s1: [wchar_t; 3] = [0x41, 0x42, 0x43];
        let s2: [wchar_t; 3] = [0x41, 0x42, 0x43];
        assert_eq!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 3) }, 0);
    }

    #[test]
    fn test_wmemcmp_less() {
        let s1: [wchar_t; 3] = [0x41, 0x42, 0x43];
        let s2: [wchar_t; 3] = [0x41, 0x42, 0x44];
        assert!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 3) } < 0);
    }

    #[test]
    fn test_wmemcmp_greater() {
        let s1: [wchar_t; 3] = [0x41, 0x42, 0x44];
        let s2: [wchar_t; 3] = [0x41, 0x42, 0x43];
        assert!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 3) } > 0);
    }

    #[test]
    fn test_wmemcmp_zero_count() {
        let s1: [wchar_t; 2] = [0x41, 0x42];
        let s2: [wchar_t; 2] = [0x43, 0x44];
        assert_eq!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 0) }, 0);
    }
}
