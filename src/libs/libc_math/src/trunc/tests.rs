// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive_fraction() {
    assert!((trunc(3.7) - 3.0).abs() < 1e-10);
}

#[test]
fn test_negative_fraction() {
    assert!((trunc(-3.7) - (-3.0)).abs() < 1e-10);
}

#[test]
fn test_integer() {
    assert!((trunc(5.0) - 5.0).abs() < 1e-10);
}

#[test]
fn test_small() {
    assert_eq!(trunc(0.9).to_bits(), 0.0_f64.to_bits());
    assert_eq!(trunc(-0.9).to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn test_zero() {
    assert_eq!(trunc(0.0).to_bits(), 0.0_f64.to_bits());
    assert_eq!(trunc(-0.0).to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn test_nan() {
    assert!(trunc(f64::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(trunc(f64::INFINITY), f64::INFINITY);
    assert_eq!(trunc(f64::NEG_INFINITY), f64::NEG_INFINITY);
}
