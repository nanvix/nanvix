// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    digit_value,
    is_space,
    vsscanf,
    Cursor,
};
use ::std::vec::Vec;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

/// Variadic helper that forwards to [`vsscanf`], used to construct a real `VaList`.
///
/// # Safety
///
/// `s` and `fmt` must be valid null-terminated strings and the variadic pointers must match
/// the conversions in `fmt`.
unsafe extern "C" fn run(s: *const c_char, fmt: *const c_char, args: ...) -> c_int {
    unsafe { vsscanf(s, fmt, args) }
}

/// Collects the bytes of a null-terminated `c_char` buffer into a `Vec<u8>`.
fn cstr_bytes(buf: &[c_char]) -> Vec<u8> {
    buf.iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect()
}

#[test]
fn is_space_matches_ascii_whitespace() {
    for b in [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c] {
        assert!(is_space(b));
    }
    for b in [b'a', b'0', b'.', 0] {
        assert!(!is_space(b));
    }
}

#[test]
fn digit_value_respects_base() {
    assert_eq!(digit_value(b'9', 10), Some(9));
    assert_eq!(digit_value(b'a', 16), Some(10));
    assert_eq!(digit_value(b'F', 16), Some(15));
    assert_eq!(digit_value(b'7', 8), Some(7));
    assert_eq!(digit_value(b'8', 8), None);
    assert_eq!(digit_value(b'g', 16), None);
}

#[test]
fn cursor_skips_whitespace_and_bumps() {
    let data = c"  ab";
    let mut cur = Cursor {
        p: data.as_ptr().cast::<u8>(),
        consumed: 0,
    };
    // SAFETY: `cur` points at a valid null-terminated buffer.
    unsafe {
        cur.skip_ws();
        assert_eq!(cur.peek(), b'a');
        assert_eq!(cur.bump(), b'a');
        assert_eq!(cur.bump(), b'b');
        assert_eq!(cur.peek(), 0);
        // A bump at end-of-input does not advance past the terminator.
        assert_eq!(cur.bump(), 0);
    }
    // `consumed` counts every byte consumed, including the two skipped spaces.
    assert_eq!(cur.consumed, 4);
}

#[test]
fn scan_two_decimals() {
    let mut a: c_int = 0;
    let mut b: c_int = 0;
    // SAFETY: both pointers are valid and match the `%d` conversions.
    let n = unsafe { run(c"12 34".as_ptr(), c"%d %d".as_ptr(), &raw mut a, &raw mut b) };
    assert_eq!(n, 2);
    assert_eq!(a, 12);
    assert_eq!(b, 34);
}

#[test]
fn scan_negative_decimal() {
    let mut a: c_int = 0;
    // SAFETY: the pointer is valid and matches the `%d` conversion.
    let n = unsafe { run(c"-42".as_ptr(), c"%d".as_ptr(), &raw mut a) };
    assert_eq!(n, 1);
    assert_eq!(a, -42);
}

#[test]
fn scan_hex_with_prefix() {
    let mut a: c_int = 0;
    // SAFETY: the pointer is valid and matches the `%x` conversion.
    let n = unsafe { run(c"0xff".as_ptr(), c"%x".as_ptr(), &raw mut a) };
    assert_eq!(n, 1);
    assert_eq!(a, 255);
}

#[test]
fn scan_string() {
    let mut buf = [0 as c_char; 8];
    // SAFETY: the buffer is large enough for "hello" plus a null terminator.
    let n = unsafe { run(c"hello world".as_ptr(), c"%s".as_ptr(), buf.as_mut_ptr()) };
    assert_eq!(n, 1);
    assert_eq!(cstr_bytes(&buf), b"hello");
}

#[test]
fn scan_respects_field_width() {
    let mut a: c_int = 0;
    let mut b: c_int = 0;
    // SAFETY: both pointers are valid and match the width-limited `%d` conversions.
    let n = unsafe { run(c"1234".as_ptr(), c"%2d%2d".as_ptr(), &raw mut a, &raw mut b) };
    assert_eq!(n, 2);
    assert_eq!(a, 12);
    assert_eq!(b, 34);
}

#[test]
fn scan_suppresses_assignment() {
    let mut a: c_int = 0;
    // SAFETY: the single pointer matches the non-suppressed `%d` conversion.
    let n = unsafe { run(c"7 8".as_ptr(), c"%*d %d".as_ptr(), &raw mut a) };
    assert_eq!(n, 1);
    assert_eq!(a, 8);
}

#[test]
fn scan_reports_eof_on_empty_input() {
    let mut a: c_int = 0;
    // SAFETY: the pointer is valid; no conversion will be performed.
    let n = unsafe { run(c"".as_ptr(), c"%d".as_ptr(), &raw mut a) };
    assert_eq!(n, -1);
}

#[test]
fn scan_stops_at_matching_failure() {
    let mut a: c_int = 0;
    let mut b: c_int = 0;
    // SAFETY: both pointers are valid; the second conversion fails to match.
    let n = unsafe { run(c"12 xy".as_ptr(), c"%d %d".as_ptr(), &raw mut a, &raw mut b) };
    assert_eq!(n, 1);
    assert_eq!(a, 12);
}
