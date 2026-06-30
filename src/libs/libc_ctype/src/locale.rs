// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    isalnum::isalnum,
    isalpha::isalpha,
    isblank::isblank,
    iscntrl::iscntrl,
    isdigit::isdigit,
    isgraph::isgraph,
    islower::islower,
    isprint::isprint,
    ispunct::ispunct,
    isspace::isspace,
    isupper::isupper,
    isxdigit::isxdigit,
    tolower::tolower,
    toupper::toupper,
};
use ::sysapi::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Nanvix supports only the C/POSIX locale, so every `*_l` function ignores its `locale_t` argument
// and delegates to its non-`_l` counterpart, mirroring the wide-character precedent in
// `libc_wctype::locale`.

/// Tests whether a character is alphanumeric in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isalnum_l(c: c_int, _locale: *mut c_void) -> c_int {
    isalnum(c)
}

/// Tests whether a character is alphabetic in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isalpha_l(c: c_int, _locale: *mut c_void) -> c_int {
    isalpha(c)
}

/// Tests whether a character is blank in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isblank_l(c: c_int, _locale: *mut c_void) -> c_int {
    isblank(c)
}

/// Tests whether a character is a control character in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn iscntrl_l(c: c_int, _locale: *mut c_void) -> c_int {
    iscntrl(c)
}

/// Tests whether a character is a decimal digit in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isdigit_l(c: c_int, _locale: *mut c_void) -> c_int {
    isdigit(c)
}

/// Tests whether a character has a graphical representation in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isgraph_l(c: c_int, _locale: *mut c_void) -> c_int {
    isgraph(c)
}

/// Tests whether a character is lowercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn islower_l(c: c_int, _locale: *mut c_void) -> c_int {
    islower(c)
}

/// Tests whether a character is printable in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isprint_l(c: c_int, _locale: *mut c_void) -> c_int {
    isprint(c)
}

/// Tests whether a character is punctuation in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ispunct_l(c: c_int, _locale: *mut c_void) -> c_int {
    ispunct(c)
}

/// Tests whether a character is white space in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isspace_l(c: c_int, _locale: *mut c_void) -> c_int {
    isspace(c)
}

/// Tests whether a character is uppercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isupper_l(c: c_int, _locale: *mut c_void) -> c_int {
    isupper(c)
}

/// Tests whether a character is a hexadecimal digit in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn isxdigit_l(c: c_int, _locale: *mut c_void) -> c_int {
    isxdigit(c)
}

/// Converts a character to lowercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tolower_l(c: c_int, _locale: *mut c_void) -> c_int {
    tolower(c)
}

/// Converts a character to uppercase in the C/POSIX locale.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn toupper_l(c: c_int, _locale: *mut c_void) -> c_int {
    toupper(c)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn test_classification_l_delegates() {
        let locale: *mut c_void = ::core::ptr::null_mut();
        assert_ne!(isdigit_l(i32::from(b'7'), locale), 0);
        assert_eq!(isdigit_l(i32::from(b'a'), locale), 0);
        assert_ne!(isalpha_l(i32::from(b'a'), locale), 0);
        assert_ne!(isupper_l(i32::from(b'A'), locale), 0);
        assert_ne!(islower_l(i32::from(b'a'), locale), 0);
        assert_ne!(isspace_l(i32::from(b' '), locale), 0);
        assert_ne!(isxdigit_l(i32::from(b'f'), locale), 0);
        assert_eq!(isxdigit_l(i32::from(b'g'), locale), 0);
    }

    #[test]
    fn test_conversion_l_delegates() {
        let locale: *mut c_void = ::core::ptr::null_mut();
        assert_eq!(tolower_l(i32::from(b'A'), locale), i32::from(b'a'));
        assert_eq!(toupper_l(i32::from(b'a'), locale), i32::from(b'A'));
    }
}
