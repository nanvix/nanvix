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
    ungetc::ungetc,
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
/// Pushes the wide character `wc` back onto the input stream `stream`, where it will be returned by
/// the next read. Only one wide character of push-back is guaranteed. In the C/POSIX locale a wide
/// character is encoded as a single byte, so only wide characters in the range `0..=255` can be
/// pushed back.
///
/// # Parameters
///
/// - `wc`: The wide character to push back.
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Return Value
///
/// The wide character pushed back as a [`wint_t`], or `WEOF` on error. On an encoding error (a wide
/// character that is not representable in the single-byte C/POSIX locale) `errno` is set to
/// `EILSEQ`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/ungetwc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ungetwc(wc: wint_t, stream: *mut FILE) -> wint_t {
    if stream.is_null() || wc == WEOF {
        return WEOF;
    }

    // The C/POSIX locale is single-byte: only wide characters in 0..=255 can be pushed back.
    let cp: u32 = u32::from_ne_bytes(wc.to_ne_bytes());
    if cp > 0xff {
        set_errno(EILSEQ);
        return WEOF;
    }

    let byte: c_int = c_int::from(u8::try_from(cp).unwrap_or(0));
    if unsafe { ungetc(byte, stream) } == EOF {
        return WEOF;
    }

    if (*stream).orientation == 0 {
        (*stream).orientation = 1;
    }

    wc
}
