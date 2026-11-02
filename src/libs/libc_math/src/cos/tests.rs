// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

const PI: f64 = core::f64::consts::PI;

#[test]
fn test_zero() {
    assert!((cos(0.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_pi_over_2() {
    assert!((cos(FRAC_PI_2)).abs() < 1e-10);
}

#[test]
fn test_pi() {
    assert!((cos(PI) - (-1.0)).abs() < 1e-10);
}

#[test]
fn test_pi_over_3() {
    assert!((cos(PI / 3.0) - 0.5).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!(cos(f64::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert!(cos(f64::INFINITY).is_nan());
}
