// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

const PI: f32 = core::f32::consts::PI;

#[test]
fn test_zero() {
    assert!((tanf(0.0)).abs() < 1e-5);
}

#[test]
fn test_pi_over_4() {
    assert!((tanf(PI / 4.0) - 1.0).abs() < 1e-4);
}

#[test]
fn test_nan() {
    assert!(tanf(f32::NAN).is_nan());
}
