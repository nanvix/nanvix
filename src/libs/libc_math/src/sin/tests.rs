// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((sin(0.0)).abs() < 1e-10);
}

#[test]
fn test_pi_over_2() {
    assert!((sin(FRAC_PI_2) - 1.0).abs() < 1e-10);
}

#[test]
fn test_pi() {
    assert!((sin(PI)).abs() < 1e-10);
}

#[test]
fn test_negative_pi_over_2() {
    assert!((sin(-FRAC_PI_2) - (-1.0)).abs() < 1e-10);
}

#[test]
fn test_pi_over_6() {
    assert!((sin(PI / 6.0) - 0.5).abs() < 1e-10);
}

#[test]
fn test_large_value() {
    assert!((sin(100.0 * PI)).abs() < 1e-6);
}

#[test]
fn test_nan() {
    assert!(sin(f64::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert!(sin(f64::INFINITY).is_nan());
}
