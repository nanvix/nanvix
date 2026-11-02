// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcsncmp_equal_prefix() {
    let s1: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let s2: [wchar_t; 4] = [0x41, 0x42, 0x44, 0];
    assert_eq!(unsafe { wcsncmp(s1.as_ptr(), s2.as_ptr(), 2) }, 0);
}

#[test]
fn test_wcsncmp_different_in_range() {
    let s1: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let s2: [wchar_t; 4] = [0x41, 0x42, 0x44, 0];
    assert!(unsafe { wcsncmp(s1.as_ptr(), s2.as_ptr(), 3) } < 0);
}

#[test]
fn test_wcsncmp_n_zero() {
    let s1: [wchar_t; 2] = [0x41, 0];
    let s2: [wchar_t; 2] = [0x42, 0];
    assert_eq!(unsafe { wcsncmp(s1.as_ptr(), s2.as_ptr(), 0) }, 0);
}

#[test]
fn test_wcsncmp_equal_full() {
    let s1: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let s2: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    assert_eq!(unsafe { wcsncmp(s1.as_ptr(), s2.as_ptr(), 3) }, 0);
}
