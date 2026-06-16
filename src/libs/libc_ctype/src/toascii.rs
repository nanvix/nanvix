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
/// Converts a character to a 7-bit ASCII value by clearing the high-order bits (POSIX extension).
///
/// Unlike `tolower`/`toupper`, this function intentionally masks *every* input instead of
/// preserving out-of-range values: `EOF` and values outside `0..=0xFF` are also reduced to their
/// low 7 bits. This matches the POSIX/NewLib definition of `toascii` (`c & 0x7F`).
///
/// # Parameters
///
/// - `c`: Character to convert, as an `int`.
///
/// # Return Value
///
/// The value of `c` with only the lower 7 bits preserved (`c & 0x7F`).
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn toascii(c: c_int) -> c_int {
    c & 0x7F
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::toascii;

    #[test]
    fn test_toascii_ascii_unchanged() {
        for c in 0x00..=0x7F {
            assert_eq!(toascii(c), c);
        }
    }

    #[test]
    fn test_toascii_high_bit_stripped() {
        assert_eq!(toascii(0x80), 0x00);
        assert_eq!(toascii(0xC1), 0x41); // 0xC1 & 0x7F == 'A'
        assert_eq!(toascii(0xFF), 0x7F);
    }

    #[test]
    fn test_toascii_negative() {
        // toascii() masks every input, including EOF and other negatives: in two's
        // complement the low 7 bits are preserved.
        assert_eq!(toascii(-1), 0x7F); // EOF
        assert_eq!(toascii(-2), 0x7E);
        assert_eq!(toascii(-128), 0x00);
    }

    #[test]
    fn test_toascii_above_unsigned_char() {
        // Values greater than 0xFF are also masked to their low 7 bits.
        assert_eq!(toascii(0x100), 0x00);
        assert_eq!(toascii(0x1C1), 0x41); // 0x1C1 & 0x7F == 'A'
        assert_eq!(toascii(0x1FF), 0x7F);
    }
}
