// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iswctype;
use crate::{
    wctype::{
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
    },
    wctype_t::wctype_t,
};

#[test]
fn test_iswctype_valid_classes() {
    assert_ne!(iswctype(0x41, WCTYPE_ALNUM), 0); // 'A'
    assert_ne!(iswctype(0x41, WCTYPE_ALPHA), 0); // 'A'
    assert_ne!(iswctype(0x09, WCTYPE_BLANK), 0); // tab
    assert_ne!(iswctype(0x7F, WCTYPE_CNTRL), 0); // DEL
    assert_ne!(iswctype(0x39, WCTYPE_DIGIT), 0); // '9'
    assert_ne!(iswctype(0x21, WCTYPE_GRAPH), 0); // '!'
    assert_ne!(iswctype(0x7A, WCTYPE_LOWER), 0); // 'z'
    assert_ne!(iswctype(0x20, WCTYPE_PRINT), 0); // space
    assert_ne!(iswctype(0x21, WCTYPE_PUNCT), 0); // '!'
    assert_ne!(iswctype(0x0A, WCTYPE_SPACE), 0); // newline
    assert_ne!(iswctype(0x41, WCTYPE_UPPER), 0); // 'A'
    assert_ne!(iswctype(0x46, WCTYPE_XDIGIT), 0); // 'F'
}

#[test]
fn test_iswctype_zero_class() {
    assert_eq!(iswctype(0x41, 0), 0);
}

#[test]
fn test_iswctype_invalid_class() {
    let invalid: wctype_t = 999;
    assert_eq!(iswctype(0x41, invalid), 0);
}
