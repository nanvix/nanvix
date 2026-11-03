// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strndup;
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

fn c_str_to_bytes(p: *const c_char) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    unsafe {
        while *p.add(i) != 0 {
            v.push(u8::from_ne_bytes((*p.add(i)).to_ne_bytes()));
            i += 1;
        }
    }
    v
}

extern "C" {
    fn free(ptr: *mut core::ffi::c_void);
}

#[test]
fn test_strndup_n_less_than_length() {
    let src: Vec<c_char> = make_c_string(b"hello world");
    let dup: *mut c_char = unsafe { strndup(src.as_ptr(), 5) };
    assert!(!dup.is_null(), "strndup should not return null");
    assert_eq!(c_str_to_bytes(dup), b"hello");
    unsafe { free(dup.cast()) };
}

#[test]
fn test_strndup_n_greater_than_length() {
    let src: Vec<c_char> = make_c_string(b"hi");
    let dup: *mut c_char = unsafe { strndup(src.as_ptr(), 100) };
    assert!(!dup.is_null(), "strndup should not return null");
    assert_eq!(c_str_to_bytes(dup), b"hi");
    unsafe { free(dup.cast()) };
}
