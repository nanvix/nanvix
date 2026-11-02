// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((atanf(0.0)).abs() < 1e-5);
}

#[test]
fn test_one() {
    assert!((atanf(1.0) - FRAC_PI_4).abs() < 1e-4);
}

#[test]
fn test_nan() {
    assert!(atanf(f32::NAN).is_nan());
}
