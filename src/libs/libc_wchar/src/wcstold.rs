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

/// Converts the initial portion of the wide string `nptr` to a `long double` value.
///
/// The C prototype returns `long double` to match POSIX. As with the narrow `strtold`, the
/// conversion is computed at `double` precision: the wide string is narrowed to its byte
/// representation and delegated to `strtod`. This is ABI-correct on the supported i686 target, where
/// the cdecl convention returns `double` and `long double` alike in the x87 `st0` register, so the
/// `f64` result is promoted to the 80-bit extended representation on return.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstold(nptr: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    extern "C" {
        fn strtod(s: *const c_char, e: *mut *mut c_char) -> f64;
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
    let val: f64 = unsafe { strtod(narrow.as_ptr(), &mut nend) };
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
    fn test_wcstold_simple() {
        // "3.50" parses to 3.5 and leaves the end pointer at the terminator.
        let s: [wchar_t; 5] = [0x33, 0x2E, 0x35, 0x30, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: f64 = unsafe { wcstold(s.as_ptr(), &mut end) };
        assert!((v - 3.5).abs() < 1e-9);
        assert_eq!(unsafe { *end }, 0);
    }

    #[test]
    fn test_wcstold_partial() {
        // "2.5x" parses 2.5 and stops at 'x'.
        let s: [wchar_t; 5] = [0x32, 0x2E, 0x35, 0x78, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: f64 = unsafe { wcstold(s.as_ptr(), &mut end) };
        assert!((v - 2.5).abs() < 1e-9);
        assert_eq!(unsafe { *end }, 0x78);
    }

    #[test]
    fn test_wcstold_null_endptr() {
        // A null end pointer is accepted.
        let s: [wchar_t; 3] = [0x31, 0x30, 0];
        let v: f64 = unsafe { wcstold(s.as_ptr(), core::ptr::null_mut()) };
        assert!((v - 10.0).abs() < 1e-9);
    }
}
