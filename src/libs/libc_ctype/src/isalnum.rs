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
/// Tests whether a character is alphanumeric.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is alphanumeric ('A'-'Z', 'a'-'z', or '0'-'9'), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isalnum(c: c_int) -> c_int {
    if (0x41..=0x5A).contains(&c) || (0x61..=0x7A).contains(&c) || (0x30..=0x39).contains(&c) {
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
    use super::isalnum;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_isalnum_uppercase() {
        for c in b'A'..=b'Z' {
            assert_ne!(
                isalnum(c_int::from(c)),
                0,
                "isalnum should return non-zero for '{}'",
                c as char
            );
        }
    }

    #[test]
    fn test_isalnum_lowercase() {
        for c in b'a'..=b'z' {
            assert_ne!(isalnum(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isalnum_digits() {
        for c in b'0'..=b'9' {
            assert_ne!(isalnum(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isalnum_special() {
        assert_eq!(isalnum(c_int::from(b' ')), 0);
        assert_eq!(isalnum(c_int::from(b'!')), 0);
        assert_eq!(isalnum(c_int::from(b'@')), 0);
        assert_eq!(isalnum(-1), 0); // EOF
    }
}
