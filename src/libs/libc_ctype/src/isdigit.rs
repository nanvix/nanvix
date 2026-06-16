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
/// Tests whether a character is a decimal digit.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is a digit ('0'-'9'), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isdigit(c: c_int) -> c_int {
    if (0x30..=0x39).contains(&c) {
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
    use super::isdigit;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_isdigit_digits() {
        for c in b'0'..=b'9' {
            assert_ne!(
                isdigit(c_int::from(c)),
                0,
                "isdigit should return non-zero for '{}'",
                c as char
            );
        }
    }

    #[test]
    fn test_isdigit_letters() {
        for c in b'A'..=b'Z' {
            assert_eq!(isdigit(c_int::from(c)), 0);
        }
        for c in b'a'..=b'z' {
            assert_eq!(isdigit(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_isdigit_special() {
        assert_eq!(isdigit(c_int::from(b' ')), 0);
        assert_eq!(isdigit(c_int::from(b'!')), 0);
        assert_eq!(isdigit(c_int::from(b'/')), 0);
        assert_eq!(isdigit(c_int::from(b':')), 0);
        assert_eq!(isdigit(-1), 0); // EOF
    }
}
