// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::memrchr;
use ::std::vec::Vec;
use ::sysapi::ffi::c_int;

#[test]
fn test_memrchr_found_at_end() {
    let buf: Vec<u8> = vec![1, 2, 3, 2, 5];
    let ret: *mut core::ffi::c_void = unsafe { memrchr(buf.as_ptr().cast(), 2 as c_int, 5) };
    assert!(!ret.is_null(), "memrchr should find the byte");
    let offset: usize = (ret as usize) - (buf.as_ptr() as usize);
    assert_eq!(offset, 3, "last 2 is at index 3");
}

#[test]
fn test_memrchr_found_at_start() {
    let buf: Vec<u8> = vec![7, 1, 2, 3, 4];
    let ret: *mut core::ffi::c_void = unsafe { memrchr(buf.as_ptr().cast(), 7 as c_int, 5) };
    assert!(!ret.is_null(), "memrchr should find the byte");
    let offset: usize = (ret as usize) - (buf.as_ptr() as usize);
    assert_eq!(offset, 0, "7 is at index 0");
}

#[test]
fn test_memrchr_not_found() {
    let buf: Vec<u8> = vec![1, 2, 3, 4, 5];
    let ret: *mut core::ffi::c_void = unsafe { memrchr(buf.as_ptr().cast(), 9 as c_int, 5) };
    assert!(ret.is_null(), "memrchr should return null when byte not found");
}
