// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strlcat;
use ::std::vec::Vec;
use ::sysapi::{
    ffi::c_char,
    sys_types::c_size_t,
};

fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes
        .iter()
        .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
        .collect();
    v.push(0 as c_char);
    v
}

#[test]
fn test_strlcat_normal() {
    let mut dest: Vec<c_char> = vec![0 as c_char; 20];
    let hello: Vec<c_char> = make_c_string(b"hello");
    for (i, &c) in hello.iter().enumerate() {
        dest[i] = c;
    }
    let src: Vec<c_char> = make_c_string(b" world");
    let ret: c_size_t = unsafe { strlcat(dest.as_mut_ptr(), src.as_ptr(), 20) };
    assert_eq!(ret as usize, 11, "strlcat returns strlen(dest) + strlen(src)");
    assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
    assert_eq!(dest[5], c_char::try_from(b' ').expect("ASCII fits in c_char"));
    assert_eq!(dest[11], 0 as c_char);
}

#[test]
fn test_strlcat_truncation() {
    let mut dest: Vec<c_char> = vec![0 as c_char; 8];
    let hello: Vec<c_char> = make_c_string(b"hello");
    for (i, &c) in hello.iter().enumerate() {
        dest[i] = c;
    }
    let src: Vec<c_char> = make_c_string(b" world");
    let ret: c_size_t = unsafe { strlcat(dest.as_mut_ptr(), src.as_ptr(), 8) };
    assert_eq!(ret as usize, 11, "strlcat returns intended total length");
    assert_eq!(dest[7], 0 as c_char, "dest should be null-terminated");
}
