// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strncpy;
use ::std::vec::Vec;
use ::sysapi::ffi::c_char;

fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes
        .iter()
        .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
        .collect();
    v.push(0 as c_char);
    v
}

#[test]
fn test_strncpy_exact_fit() {
    let src: Vec<c_char> = make_c_string(b"abc");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 4];
    unsafe { strncpy(dest.as_mut_ptr(), src.as_ptr(), 4) };
    assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
    assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
    assert_eq!(dest[2], c_char::try_from(b'c').expect("ASCII fits in c_char"));
    assert_eq!(dest[3], 0 as c_char);
}

#[test]
fn test_strncpy_short_source() {
    let src: Vec<c_char> = make_c_string(b"ab");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 6];
    unsafe { strncpy(dest.as_mut_ptr(), src.as_ptr(), 6) };
    assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
    assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
    // Remaining bytes should be null-padded.
    for (j, &c) in dest[2..6].iter().enumerate() {
        let idx: usize = j + 2;
        assert_eq!(c, 0 as c_char, "byte at index {idx} should be null");
    }
}

#[test]
fn test_strncpy_long_source() {
    let src: Vec<c_char> = make_c_string(b"abcdef");
    let mut dest: Vec<c_char> = vec![0x7F as c_char; 3];
    unsafe { strncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
    assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
    assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
    assert_eq!(dest[2], c_char::try_from(b'c').expect("ASCII fits in c_char"));
    // Not null-terminated when source is longer than n.
}
