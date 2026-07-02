// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fputwc::fputwc,
    wchar_t::{
        wchar_t,
        WEOF,
    },
};
use ::libc_stdio::FILE;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the null-terminated wide string `ws` to the given file stream. The terminating null wide
/// character is not written.
///
/// # Parameters
///
/// - `ws`: Pointer to a null-terminated wide string to write.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Return Value
///
/// A non-negative value on success, or `-1` on a write error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that `ws`
/// points to a valid, null-terminated wide string and that `stream` points to a valid, open
/// [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fputws.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fputws(ws: *const wchar_t, stream: *mut FILE) -> c_int {
    if ws.is_null() || stream.is_null() {
        return -1;
    }

    let mut i: usize = 0;
    loop {
        let wc: wchar_t = unsafe { *ws.add(i) };
        if wc == 0 {
            break;
        }
        if unsafe { fputwc(wc, stream) } == WEOF {
            return -1;
        }
        i += 1;
    }

    0
}
