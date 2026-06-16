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
/// Tests whether a character is an uppercase letter.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is an uppercase letter ('A'-'Z'), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isupper(c: c_int) -> c_int {
    if (0x41..=0x5A).contains(&c) {
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
    use super::isupper;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_isupper_uppercase() {
        for c in b'A'..=b'Z' {
            assert_ne!(
                isupper(c_int::from(c)),
                0,
                "isupper should return non-zero for '{}'",
                c as char
            );
        }
    }

    #[test]
    fn test_isupper_lowercase() {
        for c in b'a'..=b'z' {
            assert_eq!(isupper(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isupper_digits() {
        for c in b'0'..=b'9' {
            assert_eq!(isupper(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isupper_eof() {
        assert_eq!(isupper(-1), 0);
    }
}
