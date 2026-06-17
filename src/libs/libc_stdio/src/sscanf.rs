// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Reads formatted input from the string `s` according to the format string `fmt`.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated input string.
/// - `fmt`: Pointer to a null-terminated scanf format string.
/// - `args`: Pointers to the storage locations matching the conversions in `fmt`.
///
/// # Returns
///
/// The number of input items successfully matched and assigned, or `EOF` if input ends before the
/// first conversion.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. `s` and `fmt` must be valid,
/// null-terminated strings and the variadic arguments must be pointers matching the conversions.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sscanf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sscanf(s: *const c_char, fmt: *const c_char, args: ...) -> c_int {
    // SAFETY: forwarding the variadic argument list to vsscanf.
    unsafe { crate::vsscanf::vsscanf(s, fmt, args) }
}
