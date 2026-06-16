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
/// Tests whether a character is a printable character other than space.
///
/// # Parameters
///
/// - `c`: Character to test, as an `int`.
///
/// # Return Value
///
/// Non-zero if the character is a graphic character (0x21 through 0x7E), zero otherwise.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isgraph(c: c_int) -> c_int {
    if (0x21..=0x7E).contains(&c) {
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
    use super::isgraph;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_isgraph_graphic() {
        for c in 0x21..=0x7E {
            assert_ne!(isgraph(c), 0, "isgraph should return non-zero for 0x{:02X}", c);
        }
    }

    #[test]
    fn test_isgraph_space() {
        assert_eq!(isgraph(c_int::from(b' ')), 0);
    }

    #[test]
    fn test_isgraph_control() {
        for c in 0x00..0x20 {
            assert_eq!(isgraph(c), 0, "isgraph should return zero for 0x{:02X}", c);
        }
    }

    #[test]
    fn test_isgraph_del() {
        assert_eq!(isgraph(0x7F), 0);
    }

    #[test]
    fn test_isgraph_eof() {
        assert_eq!(isgraph(-1), 0);
    }
}
