// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    set_errno,
    strtod::strtod,
};
use ::sysapi::{
    errno::ERANGE,
    ffi::c_char,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to `f32`.
///
/// This function delegates to `strtod` and narrows the result to `f32`. It accepts the same subject
/// sequences as `strtod`, including the `INF`/`INFINITY` and `NAN` spellings. If the value is finite
/// but too large to represent as `f32`, `errno` is set to `ERANGE` and an infinity is returned.
///
/// # Parameters
///
/// - `nptr`: Pointer to the null-terminated string to be converted.
/// - `endptr`: If not null, receives a pointer to the first character not converted.
///
/// # Returns
///
/// The converted `f32` value. Returns `0.0` if no valid conversion could be performed.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `nptr` points to a valid null-terminated string.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/strtof.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_possible_truncation)]
pub unsafe extern "C" fn strtof(nptr: *const c_char, endptr: *mut *mut c_char) -> f32 {
    let value: f64 = strtod(nptr, endptr);
    let narrowed: f32 = value as f32;
    // A finite double that narrows to infinity overflowed the `float` range; POSIX requires
    // `ERANGE`. When `strtod` itself overflowed, `value` is already infinite (and it has set
    // `ERANGE`), so this guard does not fire twice.
    if narrowed.is_infinite() && value.is_finite() {
        set_errno(ERANGE);
    }
    if value != 0.0 && (narrowed == 0.0 || narrowed.abs() < f32::MIN_POSITIVE) {
        set_errno(ERANGE);
    }
    narrowed
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::strtof;
    use crate::set_errno;
    use ::sysapi::{
        errno::ERANGE,
        ffi::{
            c_char,
            c_int,
        },
    };

    fn get_errno() -> c_int {
        unsafe { *::sysapi::errno::__errno_location() }
    }

    /// Helper to compare floats within an epsilon.
    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn basic_decimal() {
        let s = b"3.25\0";
        let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 3.25, 1e-5), "expected 3.25, got {result}");
    }

    #[test]
    fn negative_value() {
        let s = b"-1.5\0";
        let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, -1.5, 1e-5), "expected -1.5, got {result}");
    }

    #[test]
    fn exponent() {
        let s = b"2.5e3\0";
        let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 2500.0, 1.0), "expected 2500.0, got {result}");
    }

    #[test]
    fn endptr_set() {
        let s = b"1.0xyz\0";
        let mut end: *mut c_char = core::ptr::null_mut();
        let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), &mut end) };
        assert!(approx_eq(result, 1.0, 1e-5), "expected 1.0, got {result}");
        assert!(!end.is_null());
        assert_eq!(crate::c_char_to_u8(unsafe { *end }), b'x');
    }

    #[test]
    fn overflow_sets_erange() {
        set_errno(0);
        // `1e40` is representable as `f64` but overflows `f32`.
        let s = b"1e40\0";
        let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(result.is_infinite(), "expected inf, got {result}");
        assert_eq!(get_errno(), ERANGE);
    }

    #[test]
    fn underflow_sets_erange() {
        set_errno(0);
        let s = b"1e-50\0";
        let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert_eq!(result, 0.0);
        assert_eq!(get_errno(), ERANGE);
    }
}
