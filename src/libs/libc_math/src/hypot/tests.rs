// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_345() {
    assert!((hypot(3.0, 4.0) - 5.0).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert!((hypot(0.0, 5.0) - 5.0).abs() < 1e-10);
    assert!((hypot(3.0, 0.0) - 3.0).abs() < 1e-10);
}

#[test]
fn test_inf() {
    assert_eq!(hypot(f64::INFINITY, 1.0), f64::INFINITY);
    assert_eq!(hypot(1.0, f64::INFINITY), f64::INFINITY);
}

#[test]
fn test_nan() {
    assert!(hypot(f64::NAN, 1.0).is_nan());
}
