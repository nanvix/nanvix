// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_to_narrow_alloc_copies_long_prefix() {
    let mut wide: [wchar_t; 82] = [0; 82];
    for c in wide.iter_mut().take(80) {
        *c = 0x31;
    }
    wide[80] = 0x78;

    let narrow: NarrowString =
        unsafe { to_narrow_alloc(wide.as_ptr()) }.expect("narrow allocation should succeed");
    for i in 0..80 {
        assert_eq!(unsafe { *narrow.as_ptr().add(i) }, c_char::from_ne_bytes(*b"1"));
    }
    assert_eq!(unsafe { *narrow.as_ptr().add(80) }, c_char::from_ne_bytes(*b"x"));
    assert_eq!(unsafe { *narrow.as_ptr().add(81) }, 0);
}

#[test]
fn test_to_narrow_alloc_full_accepts_single_byte_characters() {
    let wide: [wchar_t; 3] = [0x41, 0xff, 0];

    let narrow: NarrowString = unsafe { to_narrow_alloc_full(wide.as_ptr()) }
        .expect("full narrow allocation should succeed");

    assert_eq!(unsafe { *narrow.as_ptr() }, c_char::from_ne_bytes(*b"A"));
    assert_eq!(unsafe { *narrow.as_ptr().add(1) }, c_char::from_ne_bytes([0xff]));
    assert_eq!(unsafe { *narrow.as_ptr().add(2) }, 0);
}

#[test]
fn test_to_narrow_alloc_full_rejects_unrepresentable_characters() {
    let wide: [wchar_t; 3] = [0x41, 0x100, 0];

    assert!(unsafe { to_narrow_alloc_full(wide.as_ptr()) }.is_none());
}
