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
/// Converts an uppercase letter to its lowercase equivalent.
///
/// # Parameters
///
/// - `c`: Character to convert, as an `int`.
///
/// # Return Value
///
/// The lowercase equivalent if `c` is an uppercase letter ('A'-'Z'), otherwise `c` unchanged.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tolower(c: c_int) -> c_int {
    if (0x41..=0x5A).contains(&c) {
        c + 32
    } else {
        c
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::tolower;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_tolower_uppercase() {
        for c in b'A'..=b'Z' {
            let expected = c + 32;
            assert_eq!(
                tolower(c_int::from(c)),
                c_int::from(expected),
                "tolower('{}') should return '{}'",
                c as char,
                expected as char
            );
        }
    }

    #[test]
    fn test_tolower_already_lower() {
        for c in b'a'..=b'z' {
            assert_eq!(tolower(c_int::from(c)), c_int::from(c));
        }
    }

    #[test]
    fn test_tolower_digit() {
        for c in b'0'..=b'9' {
            assert_eq!(tolower(c_int::from(c)), c_int::from(c));
        }
    }

    #[test]
    fn test_tolower_eof() {
        assert_eq!(tolower(-1), -1);
    }
}
