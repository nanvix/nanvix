// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wmemcmp_equal() {
    let s1: [wchar_t; 3] = [0x41, 0x42, 0x43];
    let s2: [wchar_t; 3] = [0x41, 0x42, 0x43];
    assert_eq!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 3) }, 0);
}

#[test]
fn test_wmemcmp_less() {
    let s1: [wchar_t; 3] = [0x41, 0x42, 0x43];
    let s2: [wchar_t; 3] = [0x41, 0x42, 0x44];
    assert!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 3) } < 0);
}

#[test]
fn test_wmemcmp_greater() {
    let s1: [wchar_t; 3] = [0x41, 0x42, 0x44];
    let s2: [wchar_t; 3] = [0x41, 0x42, 0x43];
    assert!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 3) } > 0);
}

#[test]
fn test_wmemcmp_zero_count() {
    let s1: [wchar_t; 2] = [0x41, 0x42];
    let s2: [wchar_t; 2] = [0x43, 0x44];
    assert_eq!(unsafe { wmemcmp(s1.as_ptr(), s2.as_ptr(), 0) }, 0);
}
