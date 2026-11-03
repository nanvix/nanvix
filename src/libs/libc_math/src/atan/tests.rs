// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((atan(0.0)).abs() < 1e-10);
}

#[test]
fn test_one() {
    assert!((atan(1.0) - FRAC_PI_4).abs() < 1e-10);
}

#[test]
fn test_negative_one() {
    assert!((atan(-1.0) - (-FRAC_PI_4)).abs() < 1e-10);
}

#[test]
fn test_large() {
    assert!((atan(1000.0) - FRAC_PI_2).abs() < 1e-3);
}

#[test]
fn test_infinity() {
    assert!((atan(f64::INFINITY) - FRAC_PI_2).abs() < 1e-10);
    assert!((atan(f64::NEG_INFINITY) - (-FRAC_PI_2)).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!(atan(f64::NAN).is_nan());
}
