// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::bcmp;
use ::std::vec::Vec;
use ::sysapi::ffi::c_int;

#[test]
fn test_bcmp_equal() {
    let a: Vec<u8> = vec![1, 2, 3, 4, 5];
    let b: Vec<u8> = vec![1, 2, 3, 4, 5];
    let ret: c_int = unsafe { bcmp(a.as_ptr().cast(), b.as_ptr().cast(), 5) };
    assert_eq!(ret, 0, "bcmp should return 0 for equal byte ranges");
}

#[test]
fn test_bcmp_differ() {
    let a: Vec<u8> = vec![1, 2, 3, 4, 5];
    let b: Vec<u8> = vec![1, 2, 0, 4, 5];
    let ret: c_int = unsafe { bcmp(a.as_ptr().cast(), b.as_ptr().cast(), 5) };
    assert_ne!(ret, 0, "bcmp should return non-zero for differing byte ranges");
}

#[test]
fn test_bcmp_zero_length() {
    let a: Vec<u8> = vec![1, 2, 3];
    let b: Vec<u8> = vec![4, 5, 6];
    let ret: c_int = unsafe { bcmp(a.as_ptr().cast(), b.as_ptr().cast(), 0) };
    assert_eq!(ret, 0, "bcmp with zero length should return 0");
}
