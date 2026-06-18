// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    errno::{
        __errno_location,
        EISDIR,
    },
    ffi::{
        c_char,
        c_int,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Removes a file or directory. Regular files (and other non-directory entries) are removed with
/// `unlink`; if the path refers to a directory, it is removed with `rmdir`.
///
/// # Parameters
///
/// - `pathname`: Pointer to a null-terminated string naming the file to remove.
///
/// # Returns
///
/// Zero on success, or -1 on failure with `errno` set.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `pathname` points to a valid, null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/remove.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn remove(pathname: *const c_char) -> c_int {
    extern "C" {
        fn unlink(path: *const c_char) -> c_int;
        fn rmdir(path: *const c_char) -> c_int;
    }

    if pathname.is_null() {
        return -1;
    }

    // SAFETY: pathname is a valid, null-terminated string.
    let rc: c_int = unsafe { unlink(pathname) };
    if rc == 0 {
        return 0;
    }

    // A directory cannot be unlinked; retry with rmdir.
    // SAFETY: __errno_location returns a valid pointer to the errno storage.
    if unsafe { *__errno_location() } == EISDIR {
        // SAFETY: pathname is a valid, null-terminated string.
        return unsafe { rmdir(pathname) };
    }

    rc
}
