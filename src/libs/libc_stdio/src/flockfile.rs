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
/// Acquires ownership of the given file stream. Nanvix streams are single-threaded, so this is a
/// no-op that exists for source compatibility with the `*_unlocked` I/O family.
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
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/flockfile.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn flockfile(stream: *mut FILE) {
    let _ = stream;
}

///
/// # Description
///
/// Attempts to acquire ownership of the given file stream without blocking. Nanvix streams are
/// single-threaded, so the lock is always available and this function always succeeds.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// Zero, indicating that the stream was successfully locked.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/flockfile.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ftrylockfile(stream: *mut FILE) -> c_int {
    let _ = stream;
    0
}

///
/// # Description
///
/// Relinquishes ownership of the given file stream. Nanvix streams are single-threaded, so this is
/// a no-op that exists for source compatibility with the `*_unlocked` I/O family.
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
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/flockfile.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn funlockfile(stream: *mut FILE) {
    let _ = stream;
}
