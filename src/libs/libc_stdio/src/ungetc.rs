// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    streams::FILE,
    EOF,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Pushes the character `c` (converted to `unsigned char`) back onto the input stream pointed to
/// by `stream`, where it will be available for subsequent read operations. Only one character of
/// push-back is guaranteed.
///
/// # Parameters
///
/// - `c`: The character to push back, passed as a [`c_int`].
/// - `stream`: Pointer to the target [`FILE`] stream.
///
/// # Returns
///
/// The character pushed back as an `unsigned char` cast to [`c_int`], or `EOF` (`-1`) on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/ungetc.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ungetc(c: c_int, stream: *mut FILE) -> c_int {
    if stream.is_null() || c == EOF {
        return EOF;
    }

    // Only one character of push-back is supported.
    if (*stream).ungetc_buf != -1 {
        return EOF;
    }

    // POSIX: the pushed-back character is the value of `c` converted to `unsigned char`.
    let byte: c_int = c_int::from(c as u8);
    (*stream).ungetc_buf = byte;
    // A successful ungetc clears the end-of-file indicator.
    (*stream).eof = 0;

    byte
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
