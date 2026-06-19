// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wint_t;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// End-of-file indicator for byte I/O.
const EOF: c_int = -1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a wide character to a single-byte character. In the POSIX locale, wide-character values
/// in the range 0..=255 are representable as single-byte characters.
///
/// # Parameters
///
/// - `c`: Wide character to convert.
///
/// # Return Value
///
/// The single-byte representation of `c`, or `EOF` if `c` cannot be represented as a single byte.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn wctob(c: wint_t) -> c_int {
    if (0..=255).contains(&c) {
        c as c_int
    } else {
        EOF
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_wctob_ascii() {
        assert_eq!(wctob(0x41), 0x41); // 'A'
        assert_eq!(wctob(0), 0); // null
        assert_eq!(wctob(127), 127); // DEL
    }

    #[test]
    fn test_wctob_posix_locale_accepts_all_bytes() {
        assert_eq!(wctob(128), 128);
        assert_eq!(wctob(255), 255);
    }

    #[test]
    fn test_wctob_out_of_byte_range() {
        assert_eq!(wctob(256), EOF);
        assert_eq!(wctob(0x1F600), EOF); // emoji codepoint
    }

    #[test]
    fn test_wctob_weof() {
        assert_eq!(wctob(-1), EOF);
    }
}
