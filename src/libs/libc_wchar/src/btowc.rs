// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::{
    wint_t,
    WEOF,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a single-byte character to a wide character. In the POSIX locale, every byte value in
/// the range 0..=255 is a valid single-byte character.
///
/// # Parameters
///
/// - `c`: Character to convert, as an `int`.
///
/// # Return Value
///
/// The wide character representation of `c`, or `WEOF` if `c` is `EOF` or outside the byte range.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn btowc(c: c_int) -> wint_t {
    if (0..=255).contains(&c) {
        c as wint_t
    } else {
        WEOF
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_btowc_ascii() {
        assert_eq!(btowc(0x41), 0x41); // 'A'
        assert_eq!(btowc(0), 0); // null
        assert_eq!(btowc(127), 127); // DEL
    }

    #[test]
    fn test_btowc_posix_locale_accepts_all_bytes() {
        assert_eq!(btowc(128), 128);
        assert_eq!(btowc(255), 255);
    }

    #[test]
    fn test_btowc_eof() {
        assert_eq!(btowc(-1), WEOF);
    }

    #[test]
    fn test_btowc_out_of_range() {
        assert_eq!(btowc(-2), WEOF);
        assert_eq!(btowc(256), WEOF);
    }
}
