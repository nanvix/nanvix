// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ungetc;
use crate::streams::FILE;

/// Builds a stream with the end-of-file indicator set and an empty push-back slot.
fn make_file() -> FILE {
    FILE {
        fd: 0,
        eof: 1,
        error: 0,
        ungetc_buf: -1,
        orientation: 0,
    }
}

#[test]
fn stores_character_and_clears_eof() {
    let mut f: FILE = make_file();
    // SAFETY: `f` is a valid FILE on the stack.
    let r: i32 = unsafe { ungetc(i32::from(b'A'), &raw mut f) };
    assert_eq!(r, i32::from(b'A'));
    assert_eq!(f.ungetc_buf, i32::from(b'A'));
    assert_eq!(f.eof, 0);
}

#[test]
fn converts_value_to_unsigned_char() {
    let mut f: FILE = make_file();
    // 0x1FF is truncated to 0xFF when converted to unsigned char.
    // SAFETY: `f` is a valid FILE on the stack.
    let r: i32 = unsafe { ungetc(0x1FF, &raw mut f) };
    assert_eq!(r, 0xFF);
    assert_eq!(f.ungetc_buf, 0xFF);
}

#[test]
fn rejects_eof() {
    let mut f: FILE = make_file();
    // SAFETY: `f` is a valid FILE on the stack.
    let r: i32 = unsafe { ungetc(-1, &raw mut f) };
    assert_eq!(r, -1);
    assert_eq!(f.ungetc_buf, -1);
    // The end-of-file indicator must remain untouched on failure.
    assert_eq!(f.eof, 1);
}

#[test]
fn rejects_second_pushback() {
    let mut f: FILE = make_file();
    // SAFETY: `f` is a valid FILE on the stack.
    unsafe {
        assert_eq!(ungetc(i32::from(b'x'), &raw mut f), i32::from(b'x'));
        assert_eq!(ungetc(i32::from(b'y'), &raw mut f), -1);
    }
    assert_eq!(f.ungetc_buf, i32::from(b'x'));
}

#[test]
fn rejects_null_stream() {
    // SAFETY: a null stream is explicitly handled and returns EOF.
    let r: i32 = unsafe { ungetc(i32::from(b'A'), ::core::ptr::null_mut()) };
    assert_eq!(r, -1);
}
