// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_squares() {
    assert!((pow(2.0, 2.0) - 4.0).abs() < 1e-10);
    assert!((pow(3.0, 3.0) - 27.0).abs() < 1e-8);
}

#[test]
fn test_zero_exponent() {
    assert!((pow(5.0, 0.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_one_exponent() {
    assert!((pow(5.0, 1.0) - 5.0).abs() < 1e-10);
}

#[test]
fn test_sqrt() {
    assert!((pow(4.0, 0.5) - 2.0).abs() < 1e-10);
}

#[test]
fn test_negative_base() {
    assert!((pow(-2.0, 3.0) - (-8.0)).abs() < 1e-8);
    assert!((pow(-2.0, 2.0) - 4.0).abs() < 1e-8);
}

#[test]
fn test_negative_base_non_integer() {
    assert!(pow(-2.0, 1.5).is_nan());
}

#[test]
fn test_nan() {
    assert!(pow(f64::NAN, 2.0).is_nan());
}

#[test]
fn test_nan_special_cases() {
    // pow(x, +-0) == 1 for any x, including NaN.
    assert_eq!(pow(f64::NAN, 0.0), 1.0);
    assert_eq!(pow(f64::NAN, -0.0), 1.0);
    // pow(1, y) == 1 for any y, including NaN.
    assert_eq!(pow(1.0, f64::NAN), 1.0);
}

#[test]
fn test_signed_zero() {
    // Odd integer exponents preserve the sign of zero.
    assert!(pow(-0.0, 3.0).is_sign_negative());
    assert_eq!(pow(-0.0, 3.0), 0.0);
    assert_eq!(pow(-0.0, -3.0), f64::NEG_INFINITY);
    // Non-odd exponents yield positive results.
    assert!(!pow(-0.0, 2.0).is_sign_negative());
    assert_eq!(pow(-0.0, -2.0), f64::INFINITY);
    assert!(!pow(0.0, 3.0).is_sign_negative());
    assert_eq!(pow(0.0, -3.0), f64::INFINITY);
}

#[test]
fn test_negative_base_large_even_integer() {
    // 2^53 + 2 is an exact even integer in f64, so the result must be real
    // (positive), not NaN. |-2|^(2^53+2) overflows to +inf.
    let y: f64 = 9_007_199_254_740_994.0;
    assert_eq!(pow(-2.0, y), f64::INFINITY);
    // A base with magnitude < 1 underflows to +0 (still positive, not NaN).
    let v: f64 = pow(-0.5, y);
    assert_eq!(v, 0.0);
    assert!(v.is_sign_positive());
}
