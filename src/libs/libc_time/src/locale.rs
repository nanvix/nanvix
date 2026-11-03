// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    strftime::strftime,
    tm_struct::tm,
};
use ::sysapi::{
    ffi::{
        c_char,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// # Description
///
/// Formats a broken-down time according to `format` using the C/POSIX locale. Nanvix supports only
/// the C/POSIX locale, so this delegates to `strftime()` and ignores the `locale_t` argument.
///
/// # Parameters
///
/// - `s`: Destination buffer for the formatted string.
/// - `max`: Maximum number of bytes to write to `s`, including the null terminator.
/// - `format`: Pointer to the null-terminated format string.
/// - `timeptr`: Pointer to the broken-down time to format.
/// - `locale`: Locale to use (ignored; only the C/POSIX locale is supported).
///
/// # Returns
///
/// The number of bytes written to `s` (excluding the null terminator), or `0` if the result would
/// not fit.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `s`, `format`, and `timeptr`,
/// which must be valid for the requested operation.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strftime_l(
    s: *mut c_char,
    max: c_size_t,
    format: *const c_char,
    timeptr: *const tm,
    _locale: *mut c_void,
) -> c_size_t {
    unsafe { strftime(s, max, format, timeptr) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
