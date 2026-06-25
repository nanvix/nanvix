// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mkostemp;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Generates a unique temporary file name from `template`, creates and opens the file with mode
/// `0600`, and returns its file descriptor. The trailing `XXXXXX` of `template` is replaced in place
/// with the characters used in the successful name. The file is created with `O_EXCL`, so it is
/// guaranteed not to have existed beforehand.
///
/// # Parameters
///
/// - `template`: Pointer to a modifiable null-terminated string ending in `XXXXXX`.
///
/// # Returns
///
/// An open file descriptor on success, or `-1` on error with `errno` set (`EINVAL` if the template
/// does not end in `XXXXXX`, or the error reported by `open()`).
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `template` points to a writable null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/mkstemp.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mkstemp(template: *mut c_char) -> c_int {
    // SAFETY: forwarded under the same caller contract as mkstemp().
    unsafe { mkostemp::mkostemp(template, 0) }
}
