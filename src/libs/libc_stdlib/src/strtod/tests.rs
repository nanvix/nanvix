// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtod;
use crate::set_errno;
use ::sysapi::{
    errno::ERANGE,
    ffi::{
        c_char,
        c_int,
    },
};

fn get_errno() -> c_int {
    unsafe { *::sysapi::errno::__errno_location() }
}

/// Helper to compare floats within an epsilon.
fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

#[test]
fn basic_decimal() {
    let s = b"3.25\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 3.25, 1e-10), "expected 3.25, got {result}");
}

#[test]
fn negative_value() {
    let s = b"-1.5\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, -1.5, 1e-10), "expected -1.5, got {result}");
}

#[test]
fn exponent_positive() {
    let s = b"1e10\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 1e10, 1.0), "expected 1e10, got {result}");
}

#[test]
fn exponent_negative() {
    let s = b"1.5e-3\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 1.5e-3, 1e-12), "expected 1.5e-3, got {result}");
}

#[test]
fn hexadecimal_without_exponent() {
    let s = b"0x10tail\0";
    let mut end: *mut c_char = core::ptr::null_mut();
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
    assert!(approx_eq(result, 16.0, 1e-10), "expected 16.0, got {result}");
    assert_eq!(unsafe { *end } as u8, b't');
}

#[test]
fn hexadecimal_with_binary_exponent() {
    let s = b"0x1.8p+1\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 3.0, 1e-10), "expected 3.0, got {result}");
}

#[test]
fn no_decimal() {
    let s = b"42\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 42.0, 1e-10), "expected 42.0, got {result}");
}

#[test]
fn endptr_set() {
    let s = b"3.25abc\0";
    let mut end: *mut c_char = core::ptr::null_mut();
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
    assert!(approx_eq(result, 3.25, 1e-10), "expected 3.25, got {result}");
    assert!(!end.is_null());
    assert_eq!(unsafe { *end } as u8, b'a');
}

#[test]
fn leading_whitespace() {
    let s = b"  +42.5\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 42.5, 1e-10), "expected 42.5, got {result}");
}

#[test]
fn no_valid_conversion() {
    let s = b"abc\0";
    let mut end: *mut c_char = core::ptr::null_mut();
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
    assert!(approx_eq(result, 0.0, 1e-10), "expected 0.0, got {result}");
    assert_eq!(end, s.as_ptr().cast_mut().cast::<c_char>());
}

#[test]
fn parses_infinity() {
    let s = b"inf\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(result.is_infinite() && result > 0.0, "expected +inf, got {result}");
}

#[test]
fn parses_negative_infinity_long_form() {
    let s = b"-INFINITY\0";
    let mut end: *mut c_char = core::ptr::null_mut();
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
    assert!(result.is_infinite() && result < 0.0, "expected -inf, got {result}");
    // The whole token is consumed.
    assert_eq!(unsafe { *end } as u8, 0);
}

#[test]
fn parses_nan() {
    let s = b"NaN\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(result.is_nan(), "expected NaN, got {result}");
}

#[test]
fn parses_nan_with_sequence() {
    let s = b"nan(123)x\0";
    let mut end: *mut c_char = core::ptr::null_mut();
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
    assert!(result.is_nan(), "expected NaN, got {result}");
    // The parenthesized sequence is consumed; endptr points just past it.
    assert_eq!(unsafe { *end } as u8, b'x');
}

#[test]
fn overflow_sets_erange() {
    set_errno(0);
    let s = b"1e400\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(result.is_infinite(), "expected inf, got {result}");
    assert_eq!(get_errno(), ERANGE);
}

#[test]
fn underflow_sets_erange() {
    set_errno(0);
    let s = b"1e-4000\0";
    let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert_eq!(result, 0.0);
    assert_eq!(get_errno(), ERANGE);
}
