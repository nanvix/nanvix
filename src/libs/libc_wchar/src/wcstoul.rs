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
mod test;
