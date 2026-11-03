// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert_eq!(log1p(0.0), 0.0);
}

#[test]
fn test_one() {
    assert!((log1p(1.0) - core::f64::consts::LN_2).abs() < 1e-10);
}

#[test]
fn test_negative_one() {
    assert_eq!(log1p(-1.0), f64::NEG_INFINITY);
}

#[test]
fn test_below_minus_one() {
    assert!(log1p(-2.0).is_nan());
}

#[test]
fn test_nan() {
    assert!(log1p(f64::NAN).is_nan());
}
