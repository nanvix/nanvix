// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strcat;
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
fn test_strcat_basic() {
    let mut dest: Vec<c_char> = vec![0 as c_char; 20];
    let hello: Vec<c_char> = make_c_string(b"hello");
    let world: Vec<c_char> = make_c_string(b" world");
    // Copy "hello" into dest first.
    for (i, &c) in hello.iter().enumerate() {
        dest[i] = c;
    }
    unsafe { strcat(dest.as_mut_ptr(), world.as_ptr()) };
    let expected: Vec<c_char> = make_c_string(b"hello world");
    for (i, &c) in expected.iter().enumerate() {
        assert_eq!(dest[i], c, "mismatch at index {i}");
    }
}

#[test]
fn test_strcat_empty_src() {
    let mut dest: Vec<c_char> = vec![0 as c_char; 10];
    let hello: Vec<c_char> = make_c_string(b"hello");
    for (i, &c) in hello.iter().enumerate() {
        dest[i] = c;
    }
    let empty: Vec<c_char> = make_c_string(b"");
    unsafe { strcat(dest.as_mut_ptr(), empty.as_ptr()) };
    assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
    assert_eq!(dest[5], 0 as c_char);
}

#[test]
fn test_strcat_empty_dest() {
    let mut dest: Vec<c_char> = vec![0 as c_char; 10];
    let src: Vec<c_char> = make_c_string(b"abc");
    unsafe { strcat(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
    assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
    assert_eq!(dest[2], c_char::try_from(b'c').expect("ASCII fits in c_char"));
    assert_eq!(dest[3], 0 as c_char);
}
