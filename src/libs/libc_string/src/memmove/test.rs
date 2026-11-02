// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::memmove;
use ::sysapi::ffi::c_void;

#[test]
fn test_memmove_zero_length() {
    let mut buf: [u8; 4] = [1, 2, 3, 4];
    let original: [u8; 4] = buf;
    unsafe {
        let dst: *mut c_void = buf.as_mut_ptr().cast::<c_void>();
        let src: *const c_void = buf.as_ptr().cast::<c_void>();
        let ret: *mut c_void = memmove(dst, src, 0);
        assert_eq!(ret, dst);
    }
    assert_eq!(buf, original);
}

#[test]
fn test_memmove_self_copy() {
    let mut buf: [u8; 4] = [10, 20, 30, 40];
    unsafe {
        let dst: *mut c_void = buf.as_mut_ptr().cast::<c_void>();
        let src: *const c_void = buf.as_ptr().cast::<c_void>();
        memmove(dst, src, u32::try_from(buf.len()).expect("buf len fits in u32"));
    }
    assert_eq!(buf, [10, 20, 30, 40]);
}

#[test]
fn test_memmove_non_overlapping() {
    let mut buf: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    unsafe {
        let dst: *mut c_void = buf[4..].as_mut_ptr().cast::<c_void>();
        let src: *const c_void = buf[..4].as_ptr().cast::<c_void>();
        memmove(dst, src, 4);
    }
    assert_eq!(buf, [0, 1, 2, 3, 0, 1, 2, 3]);
}

#[test]
fn test_memmove_overlapping_forward() {
    // Destination starts before source: forward copy is fine.
    let mut buf: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    unsafe {
        let dst: *mut c_void = buf.as_mut_ptr().cast::<c_void>(); // start at index 0
        let src: *const c_void = buf[2..].as_ptr().cast::<c_void>(); // start at index 2
        memmove(dst, src, 4); // copy 4 bytes: 3,4,5,6 into start
    }
    assert_eq!(buf, [3, 4, 5, 6, 5, 6, 7, 8]);
}

#[test]
fn test_memmove_overlapping_backward() {
    // Destination starts after source: must copy backwards to preserve data.
    let mut buf: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    unsafe {
        let dst: *mut c_void = buf[2..].as_mut_ptr().cast::<c_void>(); // index 2
        let src: *const c_void = buf.as_ptr().cast::<c_void>(); // index 0
        memmove(dst, src, 4); // copy 1,2,3,4 into positions 2..5
    }
    assert_eq!(buf, [1, 2, 1, 2, 3, 4, 7, 8]);
}
