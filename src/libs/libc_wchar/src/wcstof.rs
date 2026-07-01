// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    wchar_t::wchar_t,
    wcs_narrow::{
        to_narrow_alloc,
        NarrowString,
    },
};
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Converts the initial portion of the wide string `nptr` to a `float` value.
///
/// The wide string is narrowed to its byte representation and delegated to `strtof`, so it accepts
/// the same subject sequences as the narrow conversion, including the `INF`/`INFINITY` and `NAN`
/// spellings.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstof(nptr: *const wchar_t, endptr: *mut *mut wchar_t) -> f32 {
    extern "C" {
        fn strtof(s: *const c_char, e: *mut *mut c_char) -> f32;
    }
    let narrow: NarrowString = match unsafe { to_narrow_alloc(nptr) } {
        Some(narrow) => narrow,
        None => {
            if !endptr.is_null() {
                unsafe { *endptr = nptr.cast_mut() };
            }
            return 0.0;
        },
    };
    let mut nend: *mut c_char = core::ptr::null_mut();
    let val: f32 = unsafe { strtof(narrow.as_ptr(), &mut nend) };
    if !endptr.is_null() {
        let consumed: usize = (nend as usize) - (narrow.as_ptr() as usize);
        unsafe { *endptr = nptr.add(consumed).cast_mut() };
    }
    val
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcstof_simple() {
        // "3.50" parses to 3.5 and leaves the end pointer at the terminator.
        let s: [wchar_t; 5] = [0x33, 0x2E, 0x35, 0x30, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: f32 = unsafe { wcstof(s.as_ptr(), &mut end) };
        assert!((v - 3.5).abs() < 1e-6);
        assert_eq!(unsafe { *end }, 0);
    }

    #[test]
    fn test_wcstof_partial() {
        // "2.5x" parses 2.5 and stops at 'x'.
        let s: [wchar_t; 5] = [0x32, 0x2E, 0x35, 0x78, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: f32 = unsafe { wcstof(s.as_ptr(), &mut end) };
        assert!((v - 2.5).abs() < 1e-6);
        assert_eq!(unsafe { *end }, 0x78);
    }

    #[test]
    fn test_wcstof_null_endptr() {
        // A null end pointer is accepted.
        let s: [wchar_t; 3] = [0x31, 0x30, 0];
        let v: f32 = unsafe { wcstof(s.as_ptr(), core::ptr::null_mut()) };
        assert!((v - 10.0).abs() < 1e-6);
    }
}
