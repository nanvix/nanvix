// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcsrchr_found_last() {
    // Search for 0x41 which appears at positions 0 and 3.
    let s: [wchar_t; 5] = [0x41, 0x42, 0x43, 0x41, 0];
    let result: *mut wchar_t = unsafe { wcsrchr(s.as_ptr(), 0x41) };
    assert!(!result.is_null());
    // Should point to the last occurrence (index 3).
    assert_eq!(result, s.as_ptr().wrapping_add(3) as *mut wchar_t);
}

#[test]
fn test_wcsrchr_not_found() {
    let s: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let result: *mut wchar_t = unsafe { wcsrchr(s.as_ptr(), 0x44) };
    assert!(result.is_null());
}

#[test]
fn test_wcsrchr_single_occurrence() {
    let s: [wchar_t; 4] = [0x41, 0x42, 0x43, 0];
    let result: *mut wchar_t = unsafe { wcsrchr(s.as_ptr(), 0x42) };
    assert!(!result.is_null());
    assert_eq!(result, s.as_ptr().wrapping_add(1) as *mut wchar_t);
}
