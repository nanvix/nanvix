// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_perfect_cubes() {
    assert!((cbrt(8.0) - 2.0).abs() < 1e-10);
    assert!((cbrt(27.0) - 3.0).abs() < 1e-10);
}

#[test]
fn test_negative() {
    assert!((cbrt(-8.0) - (-2.0)).abs() < 1e-10);
}

#[test]
fn test_one() {
    assert!((cbrt(1.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert_eq!(cbrt(0.0), 0.0);
}

#[test]
fn test_nan() {
    assert!(cbrt(f64::NAN).is_nan());
}
