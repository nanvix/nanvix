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
/// Tests whether a character is a printable character (including space).
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is printable (0x20 through 0x7E), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isprint(c: c_int) -> c_int {
    if (0x20..=0x7E).contains(&c) {
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
    use super::isprint;

    #[test]
    fn test_isprint_printable() {
        for c in 0x20..=0x7E {
            assert_ne!(isprint(c), 0, "isprint should return non-zero for 0x{:02X}", c);
        }
    }

    #[test]
    fn test_isprint_control_chars() {
        for c in 0x00..0x20 {
            assert_eq!(isprint(c), 0, "isprint should return zero for 0x{:02X}", c);
        }
    }

    #[test]
    fn test_isprint_del() {
        assert_eq!(isprint(0x7F), 0);
    }

    #[test]
    fn test_isprint_eof() {
        assert_eq!(isprint(-1), 0);
    }
}
