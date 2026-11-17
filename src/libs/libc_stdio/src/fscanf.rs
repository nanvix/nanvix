// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::FILE;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

///
/// # Description
///
/// Reads formatted input from a stream, leaving the first nonmatching byte unread.
///
/// # Parameters
///
/// - `stream`: Source stream.
/// - `fmt`: Null-terminated scanf format string.
/// - `args`: Destination pointers matching the format.
///
/// # Returns
///
/// The number of assignments, or `EOF` on input failure before the first conversion.
///
/// # Safety
///
/// `stream` must be open, `fmt` must be valid, and the arguments must match its conversions.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fscanf(stream: *mut FILE, fmt: *const c_char, args: ...) -> c_int {
    unsafe { crate::vfscanf::vfscanf(stream, fmt, args) }
}
