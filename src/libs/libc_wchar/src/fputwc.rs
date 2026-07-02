// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::{
    wchar_t,
    wint_t,
    WEOF,
};
use ::libc_stdio::{
    fputc::fputc,
    stdout,
    FILE,
};
use ::sysapi::{
    errno::{
        __errno_location,
        EILSEQ,
    },
    ffi::c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// End-of-file value returned by the byte-oriented stream functions.
const EOF: c_int = -1;

//==================================================================================================
// Helpers
//==================================================================================================

/// Sets `errno` to `code`.
fn set_errno(code: c_int) {
    // SAFETY: `__errno_location()` returns a valid pointer to `errno`.
    unsafe {
        *__errno_location() = code;
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the wide character `wc` to the given file stream. In the C/POSIX locale a wide character
/// is encoded as a single byte, so only wide characters in the range `0..=255` are representable.
///
/// # Parameters
///
/// - `wc`: The wide character to write.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Return Value
///
/// The wide character written as a [`wint_t`], or `WEOF` on error. On an encoding error `errno` is
/// set to `EILSEQ`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fputwc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fputwc(wc: wchar_t, stream: *mut FILE) -> wint_t {
    if stream.is_null() {
        return WEOF;
    }

    if (*stream).orientation == 0 {
        (*stream).orientation = 1;
    }

    // The C/POSIX locale is single-byte: only wide characters in 0..=255 have a representation.
    let cp: u32 = u32::from_ne_bytes(wc.to_ne_bytes());
    if cp > 0xff {
        set_errno(EILSEQ);
        return WEOF;
    }

    let byte: c_int = c_int::from(u8::try_from(cp).unwrap_or(0));
    if unsafe { fputc(byte, stream) } == EOF {
        return WEOF;
    }

    wc
}

///
/// # Description
///
/// Writes the wide character `wc` to the given file stream. Equivalent to [`fputwc`].
///
/// # Parameters
///
/// - `wc`: The wide character to write.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Return Value
///
/// The wide character written as a [`wint_t`], or `WEOF` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/putwc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn putwc(wc: wchar_t, stream: *mut FILE) -> wint_t {
    unsafe { fputwc(wc, stream) }
}

///
/// # Description
///
/// Writes the wide character `wc` to the standard output stream. Equivalent to `fputwc(wc, stdout)`.
///
/// # Parameters
///
/// - `wc`: The wide character to write.
///
/// # Return Value
///
/// The wide character written as a [`wint_t`], or `WEOF` on error.
///
/// # Safety
///
/// This function is unsafe because it accesses the standard output stream through a raw pointer.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/putwchar.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn putwchar(wc: wchar_t) -> wint_t {
    unsafe { fputwc(wc, stdout()) }
}
