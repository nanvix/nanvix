// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::atof;
use ::sysapi::ffi::c_char;

/// Helper to compare floats within an epsilon.
fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

#[test]
fn basic_decimal() {
    let s = b"3.25\0";
    let result: f64 = unsafe { atof(s.as_ptr().cast::<c_char>()) };
    assert!(approx_eq(result, 3.25, 1e-10), "expected 3.25, got {result}");
}

#[test]
fn negative_value() {
    let s = b"-2.5\0";
    let result: f64 = unsafe { atof(s.as_ptr().cast::<c_char>()) };
    assert!(approx_eq(result, -2.5, 1e-10), "expected -2.5, got {result}");
}

#[test]
fn no_valid_conversion() {
    let s = b"abc\0";
    let result: f64 = unsafe { atof(s.as_ptr().cast::<c_char>()) };
    assert!(approx_eq(result, 0.0, 1e-10), "expected 0.0, got {result}");
}
