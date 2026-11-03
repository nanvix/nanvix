// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::memccpy;
use ::std::vec::Vec;
use ::sysapi::{
    ffi::c_int,
    sys_types::c_size_t,
};

#[test]
fn test_memccpy_char_found() {
    let src: Vec<u8> = vec![1, 2, 3, 4, 5];
    let mut dst: Vec<u8> = vec![0; 5];
    let ret: *mut core::ffi::c_void = unsafe {
        memccpy(
            dst.as_mut_ptr().cast(),
            src.as_ptr().cast(),
            3 as c_int,
            c_size_t::try_from(src.len()).expect("len fits in c_size_t"),
        )
    };
    assert!(!ret.is_null(), "memccpy should return non-null when char is found");
    // Bytes up to and including the target should be copied.
    assert_eq!(dst[0], 1);
    assert_eq!(dst[1], 2);
    assert_eq!(dst[2], 3);
    // Return pointer should be one past the copied target byte.
    let expected_offset: usize = 3;
    let actual_offset: usize = (ret as usize) - (dst.as_ptr() as usize);
    assert_eq!(actual_offset, expected_offset, "return pointer should be one past target");
}

#[test]
fn test_memccpy_char_not_found() {
    let src: Vec<u8> = vec![1, 2, 3, 4, 5];
    let mut dst: Vec<u8> = vec![0; 5];
    let ret: *mut core::ffi::c_void = unsafe {
        memccpy(
            dst.as_mut_ptr().cast(),
            src.as_ptr().cast(),
            9 as c_int,
            c_size_t::try_from(src.len()).expect("len fits in c_size_t"),
        )
    };
    assert!(ret.is_null(), "memccpy should return null when char is not found");
    // All bytes should still be copied.
    assert_eq!(dst, src);
}

#[test]
fn test_memccpy_zero_length() {
    let src: Vec<u8> = vec![1, 2, 3];
    let mut dst: Vec<u8> = vec![0xFF; 3];
    let ret: *mut core::ffi::c_void =
        unsafe { memccpy(dst.as_mut_ptr().cast(), src.as_ptr().cast(), 1 as c_int, 0) };
    assert!(ret.is_null(), "memccpy with zero length should return null");
    assert!(dst.iter().all(|&b| b == 0xFF), "destination should be unchanged");
}
