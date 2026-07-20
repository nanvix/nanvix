// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fgetwc::fgetwc,
    wchar_t::{
        wchar_t,
        wint_t,
        WEOF,
    },
};
use ::libc_stdio::FILE;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Wide newline character (`L'\n'`), which terminates a line read.
const NEWLINE: wint_t = 0x0A;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads wide characters from `stream` into the array `ws` until `n - 1` wide characters have been
/// read, a newline is read (which is retained), or end-of-file is reached. A null wide character is
/// written after the last wide character stored.
///
/// # Parameters
///
/// - `ws`: Destination buffer for at least `n` wide characters.
/// - `n`: Maximum number of wide characters to store, including the terminating null.
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Return Value
///
/// Returns `ws` on success. Returns a null pointer if end-of-file is reached before any wide
/// character is read, or if a read error occurs.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that `ws`
/// points to storage for at least `n` wide characters and that `stream` points to a valid, open
/// [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fgetws.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fgetws(ws: *mut wchar_t, n: c_int, stream: *mut FILE) -> *mut wchar_t {
    if ws.is_null() || stream.is_null() || n <= 0 {
        return ::core::ptr::null_mut();
    }

    // Room for at most `n - 1` wide characters plus the terminating null.
    let limit: usize = usize::try_from(n - 1).unwrap_or(0);
    if limit == 0 {
        // The buffer only has room for the terminator.
        unsafe { *ws = 0 };
        return ws;
    }

    let mut i: usize = 0;
    while i < limit {
        let wc: wint_t = unsafe { fgetwc(stream) };
        if wc == WEOF {
            break;
        }
        unsafe { *ws.add(i) = wc };
        i += 1;
        if wc == NEWLINE {
            break;
        }
    }

    // End-of-file or a read error before any wide character was read.
    if i == 0 {
        return ::core::ptr::null_mut();
    }

    unsafe { *ws.add(i) = 0 };
    ws
}
