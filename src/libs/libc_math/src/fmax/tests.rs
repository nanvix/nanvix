// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((fmax(2.0, 3.0) - 3.0).abs() < 1e-10);
    assert!((fmax(3.0, 2.0) - 3.0).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!((fmax(f64::NAN, 2.0) - 2.0).abs() < 1e-10);
    assert!((fmax(2.0, f64::NAN) - 2.0).abs() < 1e-10);
    assert!(fmax(f64::NAN, f64::NAN).is_nan());
}

#[test]
fn test_signed_zero() {
    // +0.0 is treated as larger than -0.0, regardless of argument order.
    assert!(!fmax(-0.0, 0.0).is_sign_negative());
    assert!(!fmax(0.0, -0.0).is_sign_negative());
    assert_eq!(fmax(-0.0, 0.0), 0.0);
}
