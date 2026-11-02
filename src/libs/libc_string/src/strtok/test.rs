// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtok;
use ::std::{
    sync::Mutex,
    vec::Vec,
};
use ::sysapi::ffi::c_char;

// Serializes tests that exercise `strtok()`, which mutates the shared `static mut LAST`.
static LOCK: Mutex<()> = Mutex::new(());

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
fn test_strtok_basic() {
    let _guard = LOCK.lock().expect("lock poisoned");
    let mut input: Vec<c_char> = make_c_string(b"hello world foo");
    let delim: Vec<c_char> = make_c_string(b" ");

    let t1: *mut c_char = unsafe { strtok(input.as_mut_ptr(), delim.as_ptr()) };
    assert!(!t1.is_null());
    assert_eq!(c_str_to_bytes(t1), b"hello");

    let t2: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
    assert!(!t2.is_null());
    assert_eq!(c_str_to_bytes(t2), b"world");

    let t3: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
    assert!(!t3.is_null());
    assert_eq!(c_str_to_bytes(t3), b"foo");

    let t4: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
    assert!(t4.is_null(), "should return null when no more tokens");
}

#[test]
fn test_strtok_multiple_delimiters() {
    let _guard = LOCK.lock().expect("lock poisoned");
    let mut input: Vec<c_char> = make_c_string(b"a,b;c");
    let delim: Vec<c_char> = make_c_string(b",;");

    let t1: *mut c_char = unsafe { strtok(input.as_mut_ptr(), delim.as_ptr()) };
    assert!(!t1.is_null());
    assert_eq!(c_str_to_bytes(t1), b"a");

    let t2: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
    assert!(!t2.is_null());
    assert_eq!(c_str_to_bytes(t2), b"b");

    let t3: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
    assert!(!t3.is_null());
    assert_eq!(c_str_to_bytes(t3), b"c");
}

#[test]
fn test_strtok_no_tokens() {
    let _guard = LOCK.lock().expect("lock poisoned");
    let mut input: Vec<c_char> = make_c_string(b",,,,");
    let delim: Vec<c_char> = make_c_string(b",");

    let t1: *mut c_char = unsafe { strtok(input.as_mut_ptr(), delim.as_ptr()) };
    assert!(t1.is_null(), "string of only delimiters should yield no tokens");
}
