// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sysapi::ffi::{
    c_char,
    c_int,
};

///
/// # Description
///
/// Reads formatted input from standard input.
///
/// # Parameters
///
/// - `fmt`: Null-terminated scanf format string.
/// - `args`: Destination pointers matching the format.
///
/// # Returns
///
/// The number of assignments, or `EOF` on input failure before the first conversion.
///
/// # Safety
///
/// `fmt` must be valid and the arguments must match its conversions. The caller must prevent
/// concurrent access to standard input.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn scanf(fmt: *const c_char, args: ...) -> c_int {
    unsafe { crate::vscanf::vscanf(fmt, args) }
}
