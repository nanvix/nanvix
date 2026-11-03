// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcsstr_found() {
    // haystack = "hello", needle = "ll".
    let haystack: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
    let needle: [wchar_t; 3] = [0x6C, 0x6C, 0];
    let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
    assert_eq!(result.cast_const(), unsafe { haystack.as_ptr().add(2) });
}

#[test]
fn test_wcsstr_not_found() {
    let haystack: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let needle: [wchar_t; 2] = [0x64, 0];
    let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
    assert!(result.is_null());
}

#[test]
fn test_wcsstr_empty_needle() {
    // An empty needle matches at the start of the haystack.
    let haystack: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let needle: [wchar_t; 1] = [0];
    let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
    assert_eq!(result.cast_const(), haystack.as_ptr());
}

#[test]
fn test_wcsstr_whole_string() {
    let haystack: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let needle: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let result: *mut wchar_t = unsafe { wcsstr(haystack.as_ptr(), needle.as_ptr()) };
    assert_eq!(result.cast_const(), haystack.as_ptr());
}
