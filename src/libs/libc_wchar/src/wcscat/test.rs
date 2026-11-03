// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcscat_basic() {
    // "AB" + "CD" = "ABCD"
    let mut dest: [wchar_t; 8] = [0x41, 0x42, 0, 0, 0, 0, 0, 0];
    let src: [wchar_t; 3] = [0x43, 0x44, 0];
    let ret: *mut wchar_t = unsafe { wcscat(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(ret, dest.as_mut_ptr());
    assert_eq!(dest[0], 0x41);
    assert_eq!(dest[1], 0x42);
    assert_eq!(dest[2], 0x43);
    assert_eq!(dest[3], 0x44);
    assert_eq!(dest[4], 0);
}

#[test]
fn test_wcscat_empty_src() {
    let mut dest: [wchar_t; 4] = [0x41, 0, 0, 0];
    let src: [wchar_t; 1] = [0];
    unsafe { wcscat(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(dest[0], 0x41);
    assert_eq!(dest[1], 0);
}

#[test]
fn test_wcscat_empty_dest() {
    let mut dest: [wchar_t; 4] = [0; 4];
    let src: [wchar_t; 3] = [0x41, 0x42, 0];
    unsafe { wcscat(dest.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(dest[0], 0x41);
    assert_eq!(dest[1], 0x42);
    assert_eq!(dest[2], 0);
}
