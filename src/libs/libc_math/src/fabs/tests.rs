// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive() {
    let result: f64 = fabs(3.14);
    assert!((result - 3.14).abs() < 1e-10);
}

#[test]
fn test_negative() {
    let result: f64 = fabs(-2.72);
    assert!((result - 2.72).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert_eq!(fabs(0.0).to_bits(), 0u64);
    assert_eq!(fabs(-0.0).to_bits(), 0u64);
}

#[test]
fn test_nan() {
    assert!(fabs(f64::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(fabs(f64::INFINITY), f64::INFINITY);
    assert_eq!(fabs(f64::NEG_INFINITY), f64::INFINITY);
}
