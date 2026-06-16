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
/// Tests whether a character is a lowercase letter.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is a lowercase letter ('a'-'z'), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn islower(c: c_int) -> c_int {
    if (0x61..=0x7A).contains(&c) {
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
    use super::islower;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_islower_lowercase() {
        for c in b'a'..=b'z' {
            assert_ne!(
                islower(c_int::from(c)),
                0,
                "islower should return non-zero for '{}'",
                c as char
            );
        }
    }

    #[test]
    fn test_islower_uppercase() {
        for c in b'A'..=b'Z' {
            assert_eq!(islower(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_islower_digits() {
        for c in b'0'..=b'9' {
            assert_eq!(islower(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_islower_eof() {
        assert_eq!(islower(-1), 0);
    }
}
