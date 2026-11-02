// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strncat;
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
fn test_strncat_n_less_than_src() {
    let mut dest: Vec<c_char> = vec![0 as c_char; 20];
    let hello: Vec<c_char> = make_c_string(b"hello");
    for (i, &c) in hello.iter().enumerate() {
        dest[i] = c;
    }
    let src: Vec<c_char> = make_c_string(b" world");
    unsafe { strncat(dest.as_mut_ptr(), src.as_ptr(), 3) };
    // Should append " wo" then null-terminate.
    let expected: Vec<c_char> = make_c_string(b"hello wo");
    for (i, &c) in expected.iter().enumerate() {
        assert_eq!(dest[i], c, "mismatch at index {i}");
    }
}

#[test]
fn test_strncat_n_greater_than_src() {
    let mut dest: Vec<c_char> = vec![0 as c_char; 20];
    let hello: Vec<c_char> = make_c_string(b"hello");
    for (i, &c) in hello.iter().enumerate() {
        dest[i] = c;
    }
    let src: Vec<c_char> = make_c_string(b" world");
    unsafe { strncat(dest.as_mut_ptr(), src.as_ptr(), 100) };
    let expected: Vec<c_char> = make_c_string(b"hello world");
    for (i, &c) in expected.iter().enumerate() {
        assert_eq!(dest[i], c, "mismatch at index {i}");
    }
}
