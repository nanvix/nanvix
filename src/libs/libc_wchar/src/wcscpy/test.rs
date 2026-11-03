// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcscpy_empty() {
    let src: [wchar_t; 1] = [0];
    let mut dest: [wchar_t; 1] = [-1];
    unsafe { wcscpy(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(dest[0], 0);
}

#[test]
fn test_wcscpy_hello() {
    let src: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
    let mut dest: [wchar_t; 6] = [0; 6];
    let ret: *mut wchar_t = unsafe { wcscpy(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(ret, dest.as_mut_ptr());
    assert_eq!(dest, src);
}
