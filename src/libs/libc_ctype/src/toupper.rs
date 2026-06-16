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
/// Converts a lowercase letter to its uppercase equivalent.
///
/// # Parameters
///
/// - `c`: Character to convert, as an `int`.
///
/// # Return Value
///
/// The uppercase equivalent if `c` is a lowercase letter ('a'-'z'), otherwise `c` unchanged.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn toupper(c: c_int) -> c_int {
    if (0x61..=0x7A).contains(&c) {
        c - 32
    } else {
        c
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::toupper;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_toupper_lowercase() {
        for c in b'a'..=b'z' {
            let expected = c - 32;
            assert_eq!(
                toupper(c_int::from(c)),
                c_int::from(expected),
                "toupper('{}') should return '{}'",
                c as char,
                expected as char
            );
        }
    }

    #[test]
    fn test_toupper_already_upper() {
        for c in b'A'..=b'Z' {
            assert_eq!(toupper(c_int::from(c)), c_int::from(c));
        }
    }

    #[test]
    fn test_toupper_digit() {
        for c in b'0'..=b'9' {
            assert_eq!(toupper(c_int::from(c)), c_int::from(c));
        }
    }

    #[test]
    fn test_toupper_eof() {
        assert_eq!(toupper(-1), -1);
    }
}
