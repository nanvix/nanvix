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

/// Converts the initial portion of the wide string `nptr` to a `double` value.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string. `endptr`, if non-null, must be a
/// valid pointer.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstod(nptr: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
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
mod test;
