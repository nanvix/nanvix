// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strsep;
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

#[test]
fn test_strsep_basic() {
    let mut input: Vec<c_char> = make_c_string(b"hello,world");
    let delim: Vec<c_char> = make_c_string(b",");
    let mut ptr: *mut c_char = input.as_mut_ptr();

    let t1: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
    assert!(!t1.is_null());
    assert_eq!(c_str_to_bytes(t1), b"hello");

    let t2: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
    assert!(!t2.is_null());
    assert_eq!(c_str_to_bytes(t2), b"world");

    let t3: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
    assert!(t3.is_null(), "should return null when no more tokens");
}

#[test]
fn test_strsep_multiple_tokens() {
    let mut input: Vec<c_char> = make_c_string(b"a;b;c");
    let delim: Vec<c_char> = make_c_string(b";");
    let mut ptr: *mut c_char = input.as_mut_ptr();

    let t1: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
    assert_eq!(c_str_to_bytes(t1), b"a");

    let t2: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
    assert_eq!(c_str_to_bytes(t2), b"b");

    let t3: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
    assert_eq!(c_str_to_bytes(t3), b"c");

    let t4: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
    assert!(t4.is_null());
}
