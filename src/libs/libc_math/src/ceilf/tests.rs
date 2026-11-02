// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive_fraction() {
    assert!((ceilf(2.3) - 3.0).abs() < 1e-5);
}

#[test]
fn test_negative_fraction() {
    assert!((ceilf(-2.3) - (-2.0)).abs() < 1e-5);
}

#[test]
fn test_nan() {
    assert!(ceilf(f32::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(ceilf(f32::INFINITY), f32::INFINITY);
}
