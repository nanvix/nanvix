// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests the end-of-file indicator for the stream pointed to by `stream`.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// Non-zero if the end-of-file indicator is set for `stream`, zero otherwise.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/feof.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn feof(stream: *mut FILE) -> c_int {
    if stream.is_null() {
        return 0;
    }

    (*stream).eof
}
