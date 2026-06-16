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
/// Tests whether a character is alphabetic.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is alphabetic ('A'-'Z' or 'a'-'z'), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isalpha(c: c_int) -> c_int {
    if (0x41..=0x5A).contains(&c) || (0x61..=0x7A).contains(&c) {
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
    use super::isalpha;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_isalpha_uppercase() {
        for c in b'A'..=b'Z' {
            assert_ne!(
                isalpha(c_int::from(c)),
                0,
                "isalpha should return non-zero for '{}'",
                c as char
            );
        }
    }

    #[test]
    fn test_isalpha_lowercase() {
        for c in b'a'..=b'z' {
            assert_ne!(isalpha(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isalpha_digit() {
        for c in b'0'..=b'9' {
            assert_eq!(isalpha(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isalpha_special() {
        assert_eq!(isalpha(c_int::from(b' ')), 0);
        assert_eq!(isalpha(c_int::from(b'!')), 0);
        assert_eq!(isalpha(-1), 0); // EOF
    }
}
