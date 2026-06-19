// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    cstring::c_str_eq,
    wctype_t::wctype_t,
};
use ::sysapi::ffi::c_char;

//==================================================================================================
// Constants
//==================================================================================================

/// Descriptor for the `alnum` wide-character class.
pub const WCTYPE_ALNUM: wctype_t = 1;

/// Descriptor for the `alpha` wide-character class.
pub const WCTYPE_ALPHA: wctype_t = 2;

/// Descriptor for the `blank` wide-character class.
pub const WCTYPE_BLANK: wctype_t = 3;

/// Descriptor for the `cntrl` wide-character class.
pub const WCTYPE_CNTRL: wctype_t = 4;

/// Descriptor for the `digit` wide-character class.
pub const WCTYPE_DIGIT: wctype_t = 5;

/// Descriptor for the `graph` wide-character class.
pub const WCTYPE_GRAPH: wctype_t = 6;

/// Descriptor for the `lower` wide-character class.
pub const WCTYPE_LOWER: wctype_t = 7;

/// Descriptor for the `print` wide-character class.
pub const WCTYPE_PRINT: wctype_t = 8;

/// Descriptor for the `punct` wide-character class.
pub const WCTYPE_PUNCT: wctype_t = 9;

/// Descriptor for the `space` wide-character class.
pub const WCTYPE_SPACE: wctype_t = 10;

/// Descriptor for the `upper` wide-character class.
pub const WCTYPE_UPPER: wctype_t = 11;

/// Descriptor for the `xdigit` wide-character class.
pub const WCTYPE_XDIGIT: wctype_t = 12;

//==================================================================================================
// Standalone Functions
//==================================================================================================

unsafe fn class_from_property(property: *const c_char) -> wctype_t {
    if property.is_null() {
        return 0;
    }

    if unsafe { c_str_eq(property, b"alnum") } {
        WCTYPE_ALNUM
    } else if unsafe { c_str_eq(property, b"alpha") } {
        WCTYPE_ALPHA
    } else if unsafe { c_str_eq(property, b"blank") } {
        WCTYPE_BLANK
    } else if unsafe { c_str_eq(property, b"cntrl") } {
        WCTYPE_CNTRL
    } else if unsafe { c_str_eq(property, b"digit") } {
        WCTYPE_DIGIT
    } else if unsafe { c_str_eq(property, b"graph") } {
        WCTYPE_GRAPH
    } else if unsafe { c_str_eq(property, b"lower") } {
        WCTYPE_LOWER
    } else if unsafe { c_str_eq(property, b"print") } {
        WCTYPE_PRINT
    } else if unsafe { c_str_eq(property, b"punct") } {
        WCTYPE_PUNCT
    } else if unsafe { c_str_eq(property, b"space") } {
        WCTYPE_SPACE
    } else if unsafe { c_str_eq(property, b"upper") } {
        WCTYPE_UPPER
    } else if unsafe { c_str_eq(property, b"xdigit") } {
        WCTYPE_XDIGIT
    } else {
        0
    }
}

///
/// # Description
///
/// Defines a wide-character classification descriptor.
///
/// # Parameters
///
/// - `property`: Pointer to a null-terminated character class name.
///
/// # Return Value
///
/// A non-zero character class descriptor for valid names, or zero otherwise.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `property`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wctype(property: *const c_char) -> wctype_t {
    unsafe { class_from_property(property) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        wctype,
        WCTYPE_ALNUM,
        WCTYPE_ALPHA,
        WCTYPE_BLANK,
        WCTYPE_CNTRL,
        WCTYPE_DIGIT,
        WCTYPE_GRAPH,
        WCTYPE_LOWER,
        WCTYPE_PRINT,
        WCTYPE_PUNCT,
        WCTYPE_SPACE,
        WCTYPE_UPPER,
        WCTYPE_XDIGIT,
    };

    #[test]
    fn test_wctype_valid_classes() {
        let classes = [
            (b"alnum\0".as_slice(), WCTYPE_ALNUM),
            (b"alpha\0".as_slice(), WCTYPE_ALPHA),
            (b"blank\0".as_slice(), WCTYPE_BLANK),
            (b"cntrl\0".as_slice(), WCTYPE_CNTRL),
            (b"digit\0".as_slice(), WCTYPE_DIGIT),
            (b"graph\0".as_slice(), WCTYPE_GRAPH),
            (b"lower\0".as_slice(), WCTYPE_LOWER),
            (b"print\0".as_slice(), WCTYPE_PRINT),
            (b"punct\0".as_slice(), WCTYPE_PUNCT),
            (b"space\0".as_slice(), WCTYPE_SPACE),
            (b"upper\0".as_slice(), WCTYPE_UPPER),
            (b"xdigit\0".as_slice(), WCTYPE_XDIGIT),
        ];

        for (name, expected) in classes {
            assert_eq!(unsafe { wctype(name.as_ptr().cast()) }, expected);
        }
    }

    #[test]
    fn test_wctype_invalid_class() {
        let invalid = b"vowel\0";
        assert_eq!(unsafe { wctype(invalid.as_ptr().cast()) }, 0);
    }

    #[test]
    fn test_wctype_null_class() {
        assert_eq!(unsafe { wctype(::core::ptr::null()) }, 0);
    }
}
