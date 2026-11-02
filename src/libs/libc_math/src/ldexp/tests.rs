// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((ldexp(1.0, 3) - 8.0).abs() < 1e-10);
}

#[test]
fn test_fraction() {
    assert!((ldexp(1.5, 2) - 6.0).abs() < 1e-10);
}

#[test]
fn test_negative_exp() {
    assert!((ldexp(8.0, -3) - 1.0).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert_eq!(ldexp(0.0, 10), 0.0);
}

#[test]
fn test_nan() {
    assert!(ldexp(f64::NAN, 5).is_nan());
}
