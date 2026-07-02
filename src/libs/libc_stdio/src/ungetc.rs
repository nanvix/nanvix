// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// End-of-file return value.
const EOF: c_int = -1;

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
mod test {
    use super::ungetc;
    use crate::streams::FILE;

    /// Builds a stream with the end-of-file indicator set and an empty push-back slot.
    fn make_file() -> FILE {
        FILE {
            fd: 0,
            eof: 1,
            error: 0,
            ungetc_buf: -1,
            orientation: 0,
        }
    }

    #[test]
    fn stores_character_and_clears_eof() {
        let mut f: FILE = make_file();
        // SAFETY: `f` is a valid FILE on the stack.
        let r: i32 = unsafe { ungetc(i32::from(b'A'), &raw mut f) };
        assert_eq!(r, i32::from(b'A'));
        assert_eq!(f.ungetc_buf, i32::from(b'A'));
        assert_eq!(f.eof, 0);
    }

    #[test]
    fn converts_value_to_unsigned_char() {
        let mut f: FILE = make_file();
        // 0x1FF is truncated to 0xFF when converted to unsigned char.
        // SAFETY: `f` is a valid FILE on the stack.
        let r: i32 = unsafe { ungetc(0x1FF, &raw mut f) };
        assert_eq!(r, 0xFF);
        assert_eq!(f.ungetc_buf, 0xFF);
    }

    #[test]
    fn rejects_eof() {
        let mut f: FILE = make_file();
        // SAFETY: `f` is a valid FILE on the stack.
        let r: i32 = unsafe { ungetc(-1, &raw mut f) };
        assert_eq!(r, -1);
        assert_eq!(f.ungetc_buf, -1);
        // The end-of-file indicator must remain untouched on failure.
        assert_eq!(f.eof, 1);
    }

    #[test]
    fn rejects_second_pushback() {
        let mut f: FILE = make_file();
        // SAFETY: `f` is a valid FILE on the stack.
        unsafe {
            assert_eq!(ungetc(i32::from(b'x'), &raw mut f), i32::from(b'x'));
            assert_eq!(ungetc(i32::from(b'y'), &raw mut f), -1);
        }
        assert_eq!(f.ungetc_buf, i32::from(b'x'));
    }

    #[test]
    fn rejects_null_stream() {
        // SAFETY: a null stream is explicitly handled and returns EOF.
        let r: i32 = unsafe { ungetc(i32::from(b'A'), ::core::ptr::null_mut()) };
        assert_eq!(r, -1);
    }
}
