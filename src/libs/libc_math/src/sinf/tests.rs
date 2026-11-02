// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((sinf(0.0)).abs() < 1e-5);
}

#[test]
fn test_pi_over_2() {
    assert!((sinf(FRAC_PI_2) - 1.0).abs() < 1e-5);
}

#[test]
fn test_pi_over_6() {
    assert!((sinf(core::f32::consts::PI / 6.0) - 0.5).abs() < 1e-4);
}

#[test]
fn test_nan() {
    assert!(sinf(f32::NAN).is_nan());
}
