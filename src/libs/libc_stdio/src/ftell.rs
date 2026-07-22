// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    streams::FILE,
    SEEK_CUR,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_long,
    },
    sys_types::off_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the current value of the file position indicator for the stream pointed to by
/// `stream`.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// The current file position on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/ftell.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ftell(stream: *mut FILE) -> c_long {
    extern "C" {
        fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    }

    if stream.is_null() {
        return -1;
    }

    let fd: c_int = (*stream).fd;
    // SAFETY: fd comes from a valid FILE.
    let pos: off_t = unsafe { lseek(fd, 0, SEEK_CUR) };
    if pos < 0 {
        // Reflect the seek failure in the stream's error indicator so ferror() reports it.
        (*stream).error = 1;
        return -1;
    }
    pos as c_long
}
