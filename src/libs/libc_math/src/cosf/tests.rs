// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((cosf(0.0) - 1.0).abs() < 1e-5);
}

#[test]
fn test_pi_over_2() {
    assert!((cosf(FRAC_PI_2)).abs() < 1e-4);
}

#[test]
fn test_nan() {
    assert!(cosf(f32::NAN).is_nan());
}
