// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests whether a character is a white-space character.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is a white-space character (' ', '\\t', '\\n', '\\v', '\\f', '\\r'),
/// zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isspace(c: c_int) -> c_int {
    if c == 0x20 || (0x09..=0x0D).contains(&c) {
        1
    } else {
        0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::isspace;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_isspace_space() {
        assert_ne!(isspace(c_int::from(b' ')), 0);
    }

    #[test]
    fn test_isspace_tab() {
        assert_ne!(isspace(0x09), 0); // '\t'
    }

    #[test]
    fn test_isspace_newline() {
        assert_ne!(isspace(0x0A), 0); // '\n'
    }

    #[test]
    fn test_isspace_vertical_tab() {
        assert_ne!(isspace(0x0B), 0); // '\v'
    }

    #[test]
    fn test_isspace_form_feed() {
        assert_ne!(isspace(0x0C), 0); // '\f'
    }

    #[test]
    fn test_isspace_carriage_return() {
        assert_ne!(isspace(0x0D), 0); // '\r'
    }

    #[test]
    fn test_isspace_non_whitespace() {
        assert_eq!(isspace(c_int::from(b'A')), 0);
        assert_eq!(isspace(c_int::from(b'0')), 0);
        assert_eq!(isspace(c_int::from(b'!')), 0);
    }

    #[test]
    fn test_isspace_eof() {
        assert_eq!(isspace(-1), 0);
    }
}
