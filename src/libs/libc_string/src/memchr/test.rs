// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::memchr;
use ::std::vec::Vec;
use ::sysapi::ffi::c_int;

#[test]
fn test_memchr_found() {
    let buf: Vec<u8> = vec![1, 2, 3, 4, 5];
    let ret: *mut core::ffi::c_void = unsafe { memchr(buf.as_ptr().cast(), 3 as c_int, 5) };
    assert!(!ret.is_null(), "memchr should find the byte");
    let offset: usize = (ret as usize) - (buf.as_ptr() as usize);
    assert_eq!(offset, 2, "byte 3 is at index 2");
}

#[test]
fn test_memchr_not_found() {
    let buf: Vec<u8> = vec![1, 2, 3, 4, 5];
    let ret: *mut core::ffi::c_void = unsafe { memchr(buf.as_ptr().cast(), 9 as c_int, 5) };
    assert!(ret.is_null(), "memchr should return null when byte not found");
}

#[test]
fn test_memchr_zero_length() {
    let buf: Vec<u8> = vec![1, 2, 3];
    let ret: *mut core::ffi::c_void = unsafe { memchr(buf.as_ptr().cast(), 1 as c_int, 0) };
    assert!(ret.is_null(), "memchr should return null with zero length");
}
