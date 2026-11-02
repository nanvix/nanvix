// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((sinh(0.0)).abs() < 1e-10);
}

#[test]
fn test_one() {
    assert!((sinh(1.0) - 1.175_201_193_643_801_4).abs() < 1e-10);
}

#[test]
fn test_odd_symmetry() {
    assert!((sinh(-1.0) + 1.175_201_193_643_801_4).abs() < 1e-10);
}

#[test]
fn test_infinity() {
    assert_eq!(sinh(f64::INFINITY), f64::INFINITY);
    assert_eq!(sinh(f64::NEG_INFINITY), f64::NEG_INFINITY);
}

#[test]
fn test_nan() {
    assert!(sinh(f64::NAN).is_nan());
}
