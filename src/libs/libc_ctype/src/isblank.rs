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
/// Tests whether a character is a blank character (space or horizontal tab).
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is a blank character (' ' or '\\t'), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isblank(c: c_int) -> c_int {
    if c == 0x20 || c == 0x09 {
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
    use super::isblank;

    #[test]
    fn test_isblank_space() {
        assert_ne!(isblank(0x20), 0);
    }

    #[test]
    fn test_isblank_tab() {
        assert_ne!(isblank(0x09), 0);
    }

    #[test]
    fn test_isblank_newline() {
        assert_eq!(isblank(0x0A), 0);
    }

    #[test]
    fn test_isblank_other_whitespace() {
        assert_eq!(isblank(0x0B), 0); // '\v'
        assert_eq!(isblank(0x0C), 0); // '\f'
        assert_eq!(isblank(0x0D), 0); // '\r'
    }

    #[test]
    fn test_isblank_eof() {
        assert_eq!(isblank(-1), 0);
    }
}
