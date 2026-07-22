// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    streams::FILE,
    SEEK_SET,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets the file position indicator for the stream pointed to by `stream` to the beginning of
/// the file. It is equivalent to `fseek(stream, 0, SEEK_SET)` except that the error indicator
/// for the stream is also cleared.
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
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/rewind.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn rewind(stream: *mut FILE) {
    if stream.is_null() {
        return;
    }

    crate::fseek::fseek(stream, 0, SEEK_SET);
    (*stream).error = 0;
}
