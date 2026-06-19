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
mod test {
    use super::*;

    #[test]
    fn test_wmemchr_finds_first_match() {
        let buf: [wchar_t; 4] = [0x41, 0x42, 0x41, 0x43];
        let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x41, 4) };
        assert_eq!(ret.cast_const(), buf.as_ptr());
    }

    #[test]
    fn test_wmemchr_finds_match_after_first() {
        let buf: [wchar_t; 4] = [0x41, 0x42, 0x43, 0x44];
        let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x43, 4) };
        assert_eq!(ret.cast_const(), unsafe { buf.as_ptr().add(2) });
    }

    #[test]
    fn test_wmemchr_returns_null_when_absent() {
        let buf: [wchar_t; 3] = [0x41, 0x42, 0x43];
        let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x44, 3) };
        assert!(ret.is_null());
    }

    #[test]
    fn test_wmemchr_zero_count() {
        let buf: [wchar_t; 1] = [0x41];
        let ret: *mut wchar_t = unsafe { wmemchr(buf.as_ptr(), 0x41, 0) };
        assert!(ret.is_null());
    }
}
