// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcscoll_equal() {
    let s1: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let s2: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    assert_eq!(unsafe { wcscoll(s1.as_ptr(), s2.as_ptr()) }, 0);
}

#[test]
fn test_wcscoll_uses_posix_codepoint_order() {
    let lower: [wchar_t; 2] = [0x61, 0];
    let upper: [wchar_t; 2] = [0x41, 0];
    assert!(unsafe { wcscoll(lower.as_ptr(), upper.as_ptr()) } > 0);
}
