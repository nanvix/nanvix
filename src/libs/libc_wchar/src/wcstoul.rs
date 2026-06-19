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
    c_ulong,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Converts the initial portion of the wide string `nptr` to an `unsigned long` value.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstoul(
    nptr: *const wchar_t,
    endptr: *mut *mut wchar_t,
    base: c_int,
) -> c_ulong {
    extern "C" {
        fn strtoul(s: *const c_char, e: *mut *mut c_char, b: c_int) -> c_ulong;
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
    let val: c_ulong = unsafe { strtoul(narrow.as_ptr(), &mut nend, base) };
    if !endptr.is_null() {
        let consumed: usize = (nend as usize) - (narrow.as_ptr() as usize);
        unsafe { *endptr = nptr.add(consumed).cast_mut() };
    }
    val
}

//==================================================================================================
// Unit Tests
//==================================================================================================

// `c_ulong` is 32-bit on the x86 guest but 64-bit on the x86_64 host, while the host `strtoul`
// returns a 32-bit `unsigned long`. The asserted values fit in 32 bits and are read consistently on
// both targets; the end-pointer checks are width-independent.
#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wcstoul_decimal() {
        // "123" base 10 -> 123.
        let s: [wchar_t; 4] = [0x31, 0x32, 0x33, 0];
        let v: c_ulong = unsafe { wcstoul(s.as_ptr(), core::ptr::null_mut(), 10) };
        assert_eq!(v, 123);
    }

    #[test]
    fn test_wcstoul_hex() {
        // "ff" base 16 -> 255; the end pointer lands on the terminator.
        let s: [wchar_t; 3] = [0x66, 0x66, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: c_ulong = unsafe { wcstoul(s.as_ptr(), &mut end, 16) };
        assert_eq!(v, 255);
        assert_eq!(unsafe { *end }, 0);
    }

    #[test]
    fn test_wcstoul_endptr_stops_at_non_digit() {
        // "99.5" parses 99 and leaves the end pointer at '.' (index 2).
        let s: [wchar_t; 5] = [0x39, 0x39, 0x2E, 0x35, 0];
        let mut end: *mut wchar_t = core::ptr::null_mut();
        let v: c_ulong = unsafe { wcstoul(s.as_ptr(), &mut end, 10) };
        assert_eq!(v, 99);
        assert_eq!(end.cast_const(), unsafe { s.as_ptr().add(2) });
    }
}
