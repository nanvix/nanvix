// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((fmin(2.0, 3.0) - 2.0).abs() < 1e-10);
    assert!((fmin(3.0, 2.0) - 2.0).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!((fmin(f64::NAN, 2.0) - 2.0).abs() < 1e-10);
    assert!((fmin(2.0, f64::NAN) - 2.0).abs() < 1e-10);
    assert!(fmin(f64::NAN, f64::NAN).is_nan());
}

#[test]
fn test_negative() {
    assert!((fmin(-1.0, -2.0) - (-2.0)).abs() < 1e-10);
}

#[test]
fn test_signed_zero() {
    // -0.0 is treated as smaller than +0.0, regardless of argument order.
    assert!(fmin(-0.0, 0.0).is_sign_negative());
    assert!(fmin(0.0, -0.0).is_sign_negative());
    assert_eq!(fmin(-0.0, 0.0), 0.0);
}
