// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strverscmp;
use ::sysapi::ffi::c_char;

fn cmp(s1: &[u8], s2: &[u8]) -> i32 {
    unsafe { strverscmp(s1.as_ptr().cast::<c_char>(), s2.as_ptr().cast::<c_char>()) }
}

#[test]
fn test_equal_strings() {
    assert_eq!(cmp(b"hello\0", b"hello\0"), 0);
}

#[test]
fn test_numeric_ordering() {
    assert!(cmp(b"file9\0", b"file10\0") < 0);
    assert!(cmp(b"file10\0", b"file9\0") > 0);
}

#[test]
fn test_plain_lexicographic() {
    assert!(cmp(b"abc\0", b"abd\0") < 0);
}

#[test]
fn test_leading_zeros() {
    assert!(cmp(b"a008\0", b"a08\0") < 0);
    assert!(cmp(b"a08\0", b"a008\0") > 0);
    assert!(cmp(b"a0\0", b"a00\0") > 0);
    assert!(cmp(b"a00\0", b"a0\0") < 0);
}
