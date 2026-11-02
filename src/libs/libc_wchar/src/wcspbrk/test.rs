// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcspbrk_found() {
    // ws1 = "hello world", ws2 = "ow" -> first match is 'o' at index 4.
    let ws1: [wchar_t; 12] = [
        0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x77, 0x6F, 0x72, 0x6C, 0x64, 0,
    ];
    let ws2: [wchar_t; 3] = [0x6F, 0x77, 0];
    let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
    assert_eq!(result.cast_const(), unsafe { ws1.as_ptr().add(4) });
}

#[test]
fn test_wcspbrk_not_found() {
    let ws1: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
    let ws2: [wchar_t; 4] = [0x78, 0x79, 0x7A, 0];
    let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
    assert!(result.is_null());
}

#[test]
fn test_wcspbrk_empty_set() {
    // An empty match set never matches.
    let ws1: [wchar_t; 6] = [0x68, 0x65, 0x6C, 0x6C, 0x6F, 0];
    let ws2: [wchar_t; 1] = [0];
    let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
    assert!(result.is_null());
}

#[test]
fn test_wcspbrk_first_char_matches() {
    let ws1: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let ws2: [wchar_t; 2] = [0x61, 0];
    let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
    assert_eq!(result.cast_const(), ws1.as_ptr());
}

#[test]
fn test_wcspbrk_empty_haystack() {
    // An empty haystack never matches, regardless of the set.
    let ws1: [wchar_t; 1] = [0];
    let ws2: [wchar_t; 2] = [0x61, 0];
    let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
    assert!(result.is_null());
}

#[test]
fn test_wcspbrk_duplicate_set() {
    // Duplicate characters in the set do not change the leftmost-match result.
    let ws1: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let ws2: [wchar_t; 4] = [0x7A, 0x7A, 0x62, 0];
    let result: *mut wchar_t = unsafe { wcspbrk(ws1.as_ptr(), ws2.as_ptr()) };
    assert_eq!(result.cast_const(), unsafe { ws1.as_ptr().add(1) });
}
