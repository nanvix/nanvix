// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Clears the end-of-file and error indicators for the stream pointed to by `stream`.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/clearerr.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn clearerr(stream: *mut FILE) {
    if stream.is_null() {
        return;
    }

    (*stream).eof = 0;
    (*stream).error = 0;
}

///
/// # Description
///
/// Non-locking variant of [`clearerr`]. Nanvix streams are single-threaded, so this is exactly
/// equivalent to [`clearerr`].
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://man7.org/linux/man-pages/man3/unlocked_stdio.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn clearerr_unlocked(stream: *mut FILE) {
    clearerr(stream)
}
