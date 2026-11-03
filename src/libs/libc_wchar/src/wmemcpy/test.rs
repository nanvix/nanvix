// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wmemcpy_basic() {
    let src: [wchar_t; 3] = [0x41, 0x42, 0x43];
    let mut dest: [wchar_t; 3] = [0; 3];
    let ret: *mut wchar_t = unsafe { wmemcpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
    assert_eq!(ret, dest.as_mut_ptr());
    assert_eq!(dest, [0x41, 0x42, 0x43]);
}

#[test]
fn test_wmemcpy_zero() {
    let src: [wchar_t; 3] = [0x41, 0x42, 0x43];
    let mut dest: [wchar_t; 3] = [-1; 3];
    unsafe { wmemcpy(dest.as_mut_ptr(), src.as_ptr(), 0) };
    assert_eq!(dest, [-1, -1, -1]);
}
