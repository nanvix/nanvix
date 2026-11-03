// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcsncpy_exact() {
    // Source fits exactly in n (including null).
    let src: [wchar_t; 3] = [0x41, 0x42, 0];
    let mut dest: [wchar_t; 3] = [-1; 3];
    unsafe { wcsncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
    assert_eq!(dest, [0x41, 0x42, 0]);
}

#[test]
fn test_wcsncpy_shorter_src() {
    // Source is shorter than n: remainder should be zero-padded.
    let src: [wchar_t; 2] = [0x41, 0];
    let mut dest: [wchar_t; 5] = [-1; 5];
    unsafe { wcsncpy(dest.as_mut_ptr(), src.as_ptr(), 5) };
    assert_eq!(dest, [0x41, 0, 0, 0, 0]);
}

#[test]
fn test_wcsncpy_longer_src() {
    // Source is longer than n: result is not null-terminated.
    let src: [wchar_t; 6] = [0x41, 0x42, 0x43, 0x44, 0x45, 0];
    let mut dest: [wchar_t; 3] = [-1; 3];
    unsafe { wcsncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
    assert_eq!(dest, [0x41, 0x42, 0x43]);
}
