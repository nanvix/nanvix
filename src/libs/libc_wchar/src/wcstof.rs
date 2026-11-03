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
mod test;
