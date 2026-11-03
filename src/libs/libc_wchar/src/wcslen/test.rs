// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcslen_empty() {
    let s: [wchar_t; 1] = [0];
    assert_eq!(unsafe { wcslen(s.as_ptr()) }, 0);
}

#[test]
fn test_wcslen_hello() {
    // "hello" as wide characters
    let s: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
    assert_eq!(unsafe { wcslen(s.as_ptr()) }, 5);
}

#[test]
fn test_wcslen_single() {
    let s: [wchar_t; 2] = [0x41, 0];
    assert_eq!(unsafe { wcslen(s.as_ptr()) }, 1);
}
