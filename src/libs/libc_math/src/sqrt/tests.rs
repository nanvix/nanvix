// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_perfect_squares() {
    assert!((sqrt(4.0) - 2.0).abs() < 1e-10);
    assert!((sqrt(9.0) - 3.0).abs() < 1e-10);
    assert!((sqrt(16.0) - 4.0).abs() < 1e-10);
}

#[test]
fn test_non_perfect() {
    assert!((sqrt(2.0) - 1.41421356237).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert_eq!(sqrt(0.0), 0.0);
}

#[test]
fn test_one() {
    assert!((sqrt(1.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_negative() {
    assert!(sqrt(-1.0).is_nan());
}

#[test]
fn test_nan() {
    assert!(sqrt(f64::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(sqrt(f64::INFINITY), f64::INFINITY);
}
