// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
