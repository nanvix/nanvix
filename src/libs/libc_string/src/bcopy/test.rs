// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::bcopy;

#[test]
fn test_bcopy_basic() {
    let src: [u8; 5] = [1, 2, 3, 4, 5];
    let mut dest: [u8; 5] = [0; 5];
    unsafe { bcopy(src.as_ptr().cast(), dest.as_mut_ptr().cast(), 5) };
    assert_eq!(dest, [1, 2, 3, 4, 5]);
}

#[test]
fn test_bcopy_overlapping() {
    let mut buf: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    unsafe {
        let src: *const core::ffi::c_void = buf.as_ptr().cast();
        let dest: *mut core::ffi::c_void = buf[2..].as_mut_ptr().cast();
        bcopy(src, dest, 4);
    }
    assert_eq!(buf, [1, 2, 1, 2, 3, 4, 7, 8]);
}
