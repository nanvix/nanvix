// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strtof;
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
fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() < eps
}

#[test]
fn basic_decimal() {
    let s = b"3.25\0";
    let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 3.25, 1e-5), "expected 3.25, got {result}");
}

#[test]
fn negative_value() {
    let s = b"-1.5\0";
    let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, -1.5, 1e-5), "expected -1.5, got {result}");
}

#[test]
fn exponent() {
    let s = b"2.5e3\0";
    let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(approx_eq(result, 2500.0, 1.0), "expected 2500.0, got {result}");
}

#[test]
fn endptr_set() {
    let s = b"1.0xyz\0";
    let mut end: *mut c_char = core::ptr::null_mut();
    let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), &mut end) };
    assert!(approx_eq(result, 1.0, 1e-5), "expected 1.0, got {result}");
    assert!(!end.is_null());
    assert_eq!(unsafe { *end } as u8, b'x');
}

#[test]
fn overflow_sets_erange() {
    set_errno(0);
    // `1e40` is representable as `f64` but overflows `f32`.
    let s = b"1e40\0";
    let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert!(result.is_infinite(), "expected inf, got {result}");
    assert_eq!(get_errno(), ERANGE);
}

#[test]
fn underflow_sets_erange() {
    set_errno(0);
    let s = b"1e-50\0";
    let result: f32 = unsafe { strtof(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
    assert_eq!(result, 0.0);
    assert_eq!(get_errno(), ERANGE);
}
