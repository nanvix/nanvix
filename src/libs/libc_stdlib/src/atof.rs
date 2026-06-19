// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::strtod::strtod;
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to `f64`.
///
/// This function is equivalent to `strtod(nptr, null)`.
///
/// # Parameters
///
/// - `nptr`: Pointer to the null-terminated string to be converted.
///
/// # Returns
///
/// The converted `f64` value. Returns `0.0` if no valid conversion could be performed.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that
/// `nptr` points to a valid null-terminated string.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/atof.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn atof(nptr: *const c_char) -> f64 {
    strtod(nptr, core::ptr::null_mut())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::atof;
    use ::sysapi::ffi::c_char;

    /// Helper to compare floats within an epsilon.
    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn basic_decimal() {
        let s = b"3.25\0";
        let result: f64 = unsafe { atof(s.as_ptr().cast::<c_char>()) };
        assert!(approx_eq(result, 3.25, 1e-10), "expected 3.25, got {result}");
    }

    #[test]
    fn negative_value() {
        let s = b"-2.5\0";
        let result: f64 = unsafe { atof(s.as_ptr().cast::<c_char>()) };
        assert!(approx_eq(result, -2.5, 1e-10), "expected -2.5, got {result}");
    }

    #[test]
    fn no_valid_conversion() {
        let s = b"abc\0";
        let result: f64 = unsafe { atof(s.as_ptr().cast::<c_char>()) };
        assert!(approx_eq(result, 0.0, 1e-10), "expected 0.0, got {result}");
    }
}
