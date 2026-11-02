// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strdup;
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
fn test_strdup_basic() {
    let src: Vec<c_char> = make_c_string(b"hello");
    let dup: *mut c_char = unsafe { strdup(src.as_ptr()) };
    assert!(!dup.is_null(), "strdup should not return null");
    assert_eq!(c_str_to_bytes(dup), b"hello");
    // Ensure it is a different allocation.
    assert_ne!(dup as *const c_char, src.as_ptr());
    unsafe { free(dup.cast()) };
}

#[test]
fn test_strdup_empty() {
    let src: Vec<c_char> = make_c_string(b"");
    let dup: *mut c_char = unsafe { strdup(src.as_ptr()) };
    assert!(!dup.is_null(), "strdup should not return null for empty string");
    assert_eq!(c_str_to_bytes(dup), b"");
    unsafe { free(dup.cast()) };
}
