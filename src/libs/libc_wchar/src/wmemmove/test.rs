// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wmemmove_non_overlapping() {
    let src: [wchar_t; 3] = [0x41, 0x42, 0x43];
    let mut dest: [wchar_t; 3] = [0; 3];
    let ret: *mut wchar_t = unsafe { wmemmove(dest.as_mut_ptr(), src.as_ptr(), 3) };
    assert_eq!(ret, dest.as_mut_ptr());
    assert_eq!(dest, [0x41, 0x42, 0x43]);
}

#[test]
fn test_wmemmove_overlap_forward() {
    // [1, 2, 3, 4, 5] — copy elements [0..3] to [1..4].
    let mut buf: [wchar_t; 5] = [1, 2, 3, 4, 5];
    unsafe {
        wmemmove(buf.as_mut_ptr().add(1), buf.as_ptr(), 3);
    }
    assert_eq!(buf, [1, 1, 2, 3, 5]);
}

#[test]
fn test_wmemmove_overlap_backward() {
    // [1, 2, 3, 4, 5] — copy elements [1..4] to [0..3].
    let mut buf: [wchar_t; 5] = [1, 2, 3, 4, 5];
    unsafe {
        wmemmove(buf.as_mut_ptr(), buf.as_ptr().add(1), 3);
    }
    assert_eq!(buf, [2, 3, 4, 4, 5]);
}

#[test]
fn test_wmemmove_zero_count() {
    let src: [wchar_t; 2] = [0x41, 0x42];
    let mut dest: [wchar_t; 2] = [-1; 2];
    unsafe { wmemmove(dest.as_mut_ptr(), src.as_ptr(), 0) };
    assert_eq!(dest, [-1, -1]);
}
