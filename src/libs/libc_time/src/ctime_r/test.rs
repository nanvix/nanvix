// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ctime_r;
use ::sysapi::{
    ffi::c_char,
    sys_types::time_t,
};

fn buf_to_string(buf: &[c_char; 26]) -> ::std::string::String {
    let bytes: ::std::vec::Vec<u8> = buf
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    ::std::string::String::from_utf8(bytes).expect("valid UTF-8")
}

#[test]
fn test_ctime_r_epoch() {
    let t: time_t = 0;
    let mut buf: [c_char; 26] = [0; 26];
    let ret: *mut c_char = unsafe { ctime_r(&t, buf.as_mut_ptr()) };
    assert!(!ret.is_null());
    assert_eq!(buf_to_string(&buf), "Thu Jan  1 00:00:00 1970\n");
}

#[test]
fn test_ctime_r_known_date() {
    // 1993-06-30 21:49:08 UTC.
    let t: time_t = 741_476_948;
    let mut buf: [c_char; 26] = [0; 26];
    let ret: *mut c_char = unsafe { ctime_r(&t, buf.as_mut_ptr()) };
    assert!(!ret.is_null());
    assert_eq!(buf_to_string(&buf), "Wed Jun 30 21:49:08 1993\n");
}

#[test]
fn test_ctime_r_null_pointers() {
    let t: time_t = 0;
    let mut buf: [c_char; 26] = [0; 26];
    assert!(unsafe { ctime_r(core::ptr::null(), buf.as_mut_ptr()) }.is_null());
    assert!(unsafe { ctime_r(&t, core::ptr::null_mut()) }.is_null());
}
