// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcscmp_equal() {
    let s1: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let s2: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    assert_eq!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) }, 0);
}

#[test]
fn test_wcscmp_less() {
    let s1: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let s2: [wchar_t; 4] = [0x41, 0x42, 0x44, 0];
    assert!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) } < 0);
}

#[test]
fn test_wcscmp_greater() {
    let s1: [wchar_t; 4] = [0x41, 0x42, 0x44, 0];
    let s2: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    assert!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) } > 0);
}

#[test]
fn test_wcscmp_prefix() {
    let s1: [wchar_t; 3] = [0x41, 0x42, 0];
    let s2: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    assert!(unsafe { wcscmp(s1.as_ptr(), s2.as_ptr()) } < 0);
}
