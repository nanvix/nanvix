// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::mempcpy;
use ::std::vec::Vec;
use ::sysapi::sys_types::c_size_t;

#[test]
fn test_mempcpy_copies_bytes() {
    let size: usize = 10;
    let src: Vec<u8> = (0..size)
        .map(|i| u8::try_from(i).expect("index fits in u8"))
        .collect();
    let mut dst: Vec<u8> = vec![0; size];

    unsafe {
        mempcpy(
            dst.as_mut_ptr().cast(),
            src.as_ptr().cast(),
            c_size_t::try_from(size).expect("size fits in c_size_t"),
        );
    }

    assert_eq!(dst, src);
}

#[test]
fn test_mempcpy_returns_end_pointer() {
    let size: usize = 10;
    let src: Vec<u8> = vec![0; size];
    let mut dst: Vec<u8> = vec![0; size];
    let dst_ptr: *mut core::ffi::c_void = dst.as_mut_ptr().cast();
    let ret: *mut core::ffi::c_void = unsafe {
        mempcpy(
            dst_ptr,
            src.as_ptr().cast(),
            c_size_t::try_from(size).expect("size fits in c_size_t"),
        )
    };
    let expected: usize = (dst_ptr as usize) + size;
    assert_eq!(ret as usize, expected, "mempcpy should return dest + len");
}

#[test]
fn test_mempcpy_zero_length() {
    let size: usize = 4;
    let src: Vec<u8> = vec![1; size];
    let mut dst: Vec<u8> = vec![0xFF; size];
    let dst_ptr: *mut core::ffi::c_void = dst.as_mut_ptr().cast();
    let ret: *mut core::ffi::c_void = unsafe { mempcpy(dst_ptr, src.as_ptr().cast(), 0) };
    assert_eq!(ret, dst_ptr, "mempcpy(.., 0) should return dest unchanged");
    assert!(dst.iter().all(|&b| b == 0xFF), "destination must be untouched");
}
