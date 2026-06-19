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
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_ulonglong,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Converts the initial portion of the wide string `nptr` to an `unsigned long long` value.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstoull(
    nptr: *const wchar_t,
    endptr: *mut *mut wchar_t,
    base: c_int,
) -> c_ulonglong {
    extern "C" {
        fn strtoull(s: *const c_char, e: *mut *mut c_char, b: c_int) -> c_ulonglong;
    }
    let narrow: NarrowString = match unsafe { to_narrow_alloc(nptr) } {
        Some(narrow) => narrow,
        None => {
            if !endptr.is_null() {
                unsafe { *endptr = nptr.cast_mut() };
            }
            return 0;
        },
    };
    let mut nend: *mut c_char = core::ptr::null_mut();
    let val: c_ulonglong = unsafe { strtoull(narrow.as_ptr(), &mut nend, base) };
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
    fn test_wcstoull_large_value() {
        // 20_000_000_000 exceeds 32 bits, exercising unsigned long long width.
        let s: [wchar_t; 12] = [
            0x32, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0,
        ];
        let v: c_ulonglong = unsafe { wcstoull(s.as_ptr(), core::ptr::null_mut(), 10) };
        assert_eq!(v, 20_000_000_000);
    }

    #[test]
    fn test_wcstoull_hex() {
        // "abc" base 16 -> 2748.
        let s: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let v: c_ulonglong = unsafe { wcstoull(s.as_ptr(), core::ptr::null_mut(), 16) };
        assert_eq!(v, 2748);
    }
}
