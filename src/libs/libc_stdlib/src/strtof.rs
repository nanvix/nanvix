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
mod tests;
