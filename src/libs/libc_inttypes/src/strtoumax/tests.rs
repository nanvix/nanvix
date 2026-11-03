// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::sysapi::ffi::c_char;

#[test]
fn test_parse_decimal() {
    // "123\0"
    let s: [c_char; 4] = [0x31, 0x32, 0x33, 0];
    let result = unsafe { strtoumax(s.as_ptr(), ::core::ptr::null_mut(), 10) };
    assert_eq!(result, 123);
}

#[test]
fn test_parse_hex_prefix() {
    // "0xFFFFFFFF\0"
    let s: [c_char; 11] = [
        0x30, 0x78, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0,
    ];
    let result = unsafe { strtoumax(s.as_ptr(), ::core::ptr::null_mut(), 16) };
    assert_eq!(result, 0xFFFF_FFFF);
}

#[test]
fn test_parse_hex_base16() {
    // "FF\0"
    let s: [c_char; 3] = [0x46, 0x46, 0];
    let result = unsafe { strtoumax(s.as_ptr(), ::core::ptr::null_mut(), 16) };
    assert_eq!(result, 255);
}

#[test]
fn test_endptr() {
    // "123abc\0"
    let s: [c_char; 7] = [0x31, 0x32, 0x33, 0x61, 0x62, 0x63, 0];
    let mut endptr: *mut c_char = ::core::ptr::null_mut();
    let result = unsafe { strtoumax(s.as_ptr(), &mut endptr, 10) };
    assert_eq!(result, 123);
    // endptr should point to 'a' (0x61).
    assert_eq!(unsafe { *endptr }, 0x61);
}

#[test]
fn test_invalid_base_too_small() {
    // base 1 is invalid: no conversion, endptr == nptr, returns 0, errno == EINVAL.
    let s: [c_char; 4] = [0x31, 0x32, 0x33, 0];
    let mut endptr: *mut c_char = ::core::ptr::null_mut();
    unsafe {
        *::sysapi::errno::__errno_location() = 0;
    }
    let result = unsafe { strtoumax(s.as_ptr(), &mut endptr, 1) };
    assert_eq!(result, 0);
    assert!(::core::ptr::eq(endptr, s.as_ptr()));
    let errno = unsafe { *::sysapi::errno::__errno_location() };
    assert_eq!(errno, ::sysapi::errno::EINVAL);
}

#[test]
fn test_invalid_base_too_large() {
    // base 37 is invalid: no conversion, endptr == nptr, returns 0, errno == EINVAL.
    let s: [c_char; 4] = [0x31, 0x32, 0x33, 0];
    let mut endptr: *mut c_char = ::core::ptr::null_mut();
    unsafe {
        *::sysapi::errno::__errno_location() = 0;
    }
    let result = unsafe { strtoumax(s.as_ptr(), &mut endptr, 37) };
    assert_eq!(result, 0);
    assert!(::core::ptr::eq(endptr, s.as_ptr()));
    let errno = unsafe { *::sysapi::errno::__errno_location() };
    assert_eq!(errno, ::sysapi::errno::EINVAL);
}

#[test]
fn test_overflow_sets_errno() {
    // "18446744073709551616\0" == u64::MAX + 1
    let s: [c_char; 21] = [
        0x31, 0x38, 0x34, 0x34, 0x36, 0x37, 0x34, 0x34, 0x30, 0x37, 0x33, 0x37, 0x30, 0x39, 0x35,
        0x35, 0x31, 0x36, 0x31, 0x36, 0,
    ];
    unsafe {
        *::sysapi::errno::__errno_location() = 0;
    }
    let result = unsafe { strtoumax(s.as_ptr(), ::core::ptr::null_mut(), 10) };
    assert_eq!(result, u64::MAX);
    let errno = unsafe { *::sysapi::errno::__errno_location() };
    assert_eq!(errno, ::sysapi::errno::ERANGE);
}

#[test]
fn test_negative_wraps_without_errno() {
    // "-1\0"
    let s: [c_char; 3] = [0x2D, 0x31, 0];
    unsafe {
        *::sysapi::errno::__errno_location() = 0;
    }
    let result = unsafe { strtoumax(s.as_ptr(), ::core::ptr::null_mut(), 10) };
    assert_eq!(result, u64::MAX);
    let errno = unsafe { *::sysapi::errno::__errno_location() };
    assert_eq!(errno, 0);
}

#[test]
fn test_hex_prefix_without_digit_base0() {
    // "0x\0" — the `0x` prefix has no following hex digit, so only the leading `0` is parsed
    // and `endptr` stops at 'x'.
    let s: [c_char; 3] = [0x30, 0x78, 0];
    let mut endptr: *mut c_char = ::core::ptr::null_mut();
    let result = unsafe { strtoumax(s.as_ptr(), &mut endptr, 0) };
    assert_eq!(result, 0);
    // endptr should point at 'x' (0x78).
    assert_eq!(unsafe { *endptr }, 0x78);
    assert!(::core::ptr::eq(endptr, unsafe { s.as_ptr().add(1) }));
}

#[test]
fn test_hex_prefix_without_digit_base16() {
    // "0x\0" with explicit base 16 — the leading `0` is parsed as a hex digit and `endptr`
    // stops at 'x'.
    let s: [c_char; 3] = [0x30, 0x78, 0];
    let mut endptr: *mut c_char = ::core::ptr::null_mut();
    let result = unsafe { strtoumax(s.as_ptr(), &mut endptr, 16) };
    assert_eq!(result, 0);
    assert_eq!(unsafe { *endptr }, 0x78);
    assert!(::core::ptr::eq(endptr, unsafe { s.as_ptr().add(1) }));
}

#[test]
fn test_hex_prefix_invalid_digit_base16() {
    // "0xG\0" — 'G' is not a hex digit, so the `0x` prefix is not consumed; the leading `0`
    // is parsed and `endptr` stops at 'x'.
    let s: [c_char; 4] = [0x30, 0x78, 0x47, 0];
    let mut endptr: *mut c_char = ::core::ptr::null_mut();
    let result = unsafe { strtoumax(s.as_ptr(), &mut endptr, 16) };
    assert_eq!(result, 0);
    assert_eq!(unsafe { *endptr }, 0x78);
    assert!(::core::ptr::eq(endptr, unsafe { s.as_ptr().add(1) }));
}
