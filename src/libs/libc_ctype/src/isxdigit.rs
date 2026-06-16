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
/// Tests whether a character is a hexadecimal digit.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is a hexadecimal digit ('0'-'9', 'A'-'F', or 'a'-'f'),
/// zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isxdigit(c: c_int) -> c_int {
    if (0x30..=0x39).contains(&c) || (0x41..=0x46).contains(&c) || (0x61..=0x66).contains(&c) {
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
    use super::isxdigit;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_isxdigit_digits() {
        for c in b'0'..=b'9' {
            assert_ne!(isxdigit(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isxdigit_upper_hex() {
        for c in b'A'..=b'F' {
            assert_ne!(isxdigit(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isxdigit_lower_hex() {
        for c in b'a'..=b'f' {
            assert_ne!(isxdigit(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isxdigit_non_hex() {
        assert_eq!(isxdigit(c_int::from(b'G')), 0);
        assert_eq!(isxdigit(c_int::from(b'g')), 0);
        assert_eq!(isxdigit(c_int::from(b'Z')), 0);
        assert_eq!(isxdigit(c_int::from(b'!')), 0);
    }

    #[test]
    fn test_isxdigit_eof() {
        assert_eq!(isxdigit(-1), 0);
    }
}
