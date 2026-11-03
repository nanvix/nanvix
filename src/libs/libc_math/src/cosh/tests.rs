// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((cosh(0.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_one() {
    assert!((cosh(1.0) - 1.543_080_634_815_243_7).abs() < 1e-10);
}

#[test]
fn test_even_symmetry() {
    assert!((cosh(-1.0) - cosh(1.0)).abs() < 1e-10);
}

#[test]
fn test_infinity() {
    assert_eq!(cosh(f64::INFINITY), f64::INFINITY);
    assert_eq!(cosh(f64::NEG_INFINITY), f64::INFINITY);
}

#[test]
fn test_nan() {
    assert!(cosh(f64::NAN).is_nan());
}
