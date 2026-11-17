// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{
    EOF,
    FILE,
};
use ::core::ffi::VaList;
use ::sysapi::{
    errno::{
        __errno_location,
        EINVAL,
    },
    ffi::{
        c_char,
        c_int,
    },
};

///
/// # Description
///
/// Reads formatted input from a stream using the same conversions as [`crate::vsscanf::vsscanf`].
/// A read failure sets the stream's error indicator; end-of-file sets its EOF indicator.
///
/// # Parameters
///
/// - `stream`: Source stream.
/// - `fmt`: Null-terminated scanf format string.
/// - `ap`: Destination pointers matching the format.
///
/// # Returns
///
/// The number of assignments, or `EOF` on input failure before the first conversion.
///
/// # Errors
///
/// Null stream or format pointers set `errno` to `EINVAL` and return `EOF`.
///
/// # Safety
///
/// Non-null pointers must be valid, `stream` must be open, and `ap` must match the conversions.
/// The caller must prevent concurrent access to the stream.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vfscanf(stream: *mut FILE, fmt: *const c_char, ap: VaList<'_>) -> c_int {
    if stream.is_null() || fmt.is_null() {
        unsafe { *__errno_location() = EINVAL };
        return EOF;
    }
    unsafe { crate::vsscanf::scan_stream(stream, fmt, ap) }
}
