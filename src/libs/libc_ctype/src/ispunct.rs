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
/// Tests whether a character is a punctuation character.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is printable but not a space, letter, or digit, zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ispunct(c: c_int) -> c_int {
    // Printable (0x20..=0x7E) but not space, not alpha, not digit.
    let is_printable = (0x20..=0x7E).contains(&c);
    let is_space = c == 0x20;
    let is_alpha = (0x41..=0x5A).contains(&c) || (0x61..=0x7A).contains(&c);
    let is_digit = (0x30..=0x39).contains(&c);
    if is_printable && !is_space && !is_alpha && !is_digit {
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
    use super::ispunct;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_ispunct_punctuation() {
        let puncts = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
        for &c in puncts {
            assert_ne!(
                ispunct(c_int::from(c)),
                0,
                "ispunct should return non-zero for '{}'",
                c as char
            );
        }
    }

    #[test]
    fn test_ispunct_letters() {
        for c in b'A'..=b'Z' {
            assert_eq!(ispunct(c_int::from(c)), 0);
        }
        for c in b'a'..=b'z' {
            assert_eq!(ispunct(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_ispunct_digits() {
        for c in b'0'..=b'9' {
            assert_eq!(ispunct(c_int::from(c)), 0);
        }
    }

    #[test]
    fn test_ispunct_space() {
        assert_eq!(ispunct(c_int::from(b' ')), 0);
    }

    #[test]
    fn test_ispunct_eof() {
        assert_eq!(ispunct(-1), 0);
    }
}
