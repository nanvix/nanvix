// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::{
    wint_t,
    WEOF,
};
use ::libc_stdio::{
    fgetc::fgetc,
    stdin,
    FILE,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// End-of-file value returned by the byte-oriented stream functions.
const EOF: c_int = -1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the next wide character from the given file stream. In the C/POSIX locale a wide character
/// is encoded as a single byte, so one byte is read and returned as a wide character.
///
/// # Parameters
///
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Return Value
///
/// The next wide character as a [`wint_t`], or `WEOF` on end-of-file or a read error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fgetwc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fgetwc(stream: *mut FILE) -> wint_t {
    if stream.is_null() {
        return WEOF;
    }

    if (*stream).orientation == 0 {
        (*stream).orientation = 1;
    }

    // The C/POSIX locale is single-byte, so one byte read yields exactly one wide character.
    let c: c_int = unsafe { fgetc(stream) };
    if c == EOF {
        return WEOF;
    }

    // `fgetc` returns the byte as an `unsigned char` in 0..=255; that value is the wide character.
    c
}

///
/// # Description
///
/// Reads the next wide character from the given file stream. Equivalent to [`fgetwc`].
///
/// # Parameters
///
/// - `stream`: Pointer to the source [`FILE`] stream.
///
/// # Return Value
///
/// The next wide character as a [`wint_t`], or `WEOF` on end-of-file or a read error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getwc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getwc(stream: *mut FILE) -> wint_t {
    unsafe { fgetwc(stream) }
}

///
/// # Description
///
/// Reads the next wide character from the standard input stream. Equivalent to `fgetwc(stdin)`.
///
/// # Return Value
///
/// The next wide character as a [`wint_t`], or `WEOF` on end-of-file or a read error.
///
/// # Safety
///
/// This function is unsafe because it accesses the standard input stream through a raw pointer.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getwchar.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getwchar() -> wint_t {
    unsafe { fgetwc(stdin()) }
}
