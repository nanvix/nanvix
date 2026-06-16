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
/// Tests whether a character is a 7-bit ASCII character (POSIX extension).
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is in the range 0x00 through 0x7F, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isascii(c: c_int) -> c_int {
    if (0x00..=0x7F).contains(&c) {
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
    use super::isascii;

    #[test]
    fn test_isascii_range() {
        for c in 0x00..=0x7F {
            assert_ne!(isascii(c), 0, "isascii should return non-zero for 0x{:02X}", c);
        }
    }

    #[test]
    fn test_isascii_above_range() {
        assert_eq!(isascii(0x80), 0);
        assert_eq!(isascii(0xFF), 0);
        assert_eq!(isascii(0x100), 0);
    }

    #[test]
    fn test_isascii_negative() {
        assert_eq!(isascii(-1), 0); // EOF
        assert_eq!(isascii(-128), 0);
    }
}
