// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wmemset_fill() {
    let mut buf: [wchar_t; 4] = [0; 4];
    let ret: *mut wchar_t = unsafe { wmemset(buf.as_mut_ptr(), 0x58, 4) };
    assert_eq!(ret, buf.as_mut_ptr());
    assert_eq!(buf, [0x58, 0x58, 0x58, 0x58]);
}

#[test]
fn test_wmemset_zero_count() {
    let mut buf: [wchar_t; 3] = [-1; 3];
    unsafe { wmemset(buf.as_mut_ptr(), 0x41, 0) };
    assert_eq!(buf, [-1, -1, -1]);
}
