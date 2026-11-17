// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::core::ffi::VaList;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

///
/// # Description
///
/// Reads formatted input from standard input using a variable argument list.
///
/// # Parameters
///
/// - `fmt`: Null-terminated scanf format string.
/// - `ap`: Destination pointers matching the format.
///
/// # Returns
///
/// The number of assignments, or `EOF` on input failure before the first conversion.
///
/// # Safety
///
/// `fmt` must be valid and `ap` must match its conversions. The caller must prevent concurrent
/// access to standard input.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vscanf(fmt: *const c_char, ap: VaList<'_>) -> c_int {
    unsafe { crate::vfscanf::vfscanf(crate::stdin(), fmt, ap) }
}
