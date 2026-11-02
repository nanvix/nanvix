// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcschr_found() {
    let s: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let result: *mut wchar_t = unsafe { wcschr(s.as_ptr(), 0x42) };
    assert!(!result.is_null());
    assert_eq!(unsafe { *result }, 0x42);
}

#[test]
fn test_wcschr_not_found() {
    let s: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let result: *mut wchar_t = unsafe { wcschr(s.as_ptr(), 0x44) };
    assert!(result.is_null());
}

#[test]
fn test_wcschr_null_char() {
    let s: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let result: *mut wchar_t = unsafe { wcschr(s.as_ptr(), 0) };
    assert!(!result.is_null());
    assert_eq!(unsafe { *result }, 0);
}
