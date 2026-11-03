// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_first_quadrant() {
    assert!((atan2f(1.0, 1.0) - PI / 4.0).abs() < 1e-3);
}

#[test]
fn test_axes() {
    assert!((atan2f(1.0, 0.0) - FRAC_PI_2).abs() < 1e-4);
}

#[test]
fn test_nan() {
    assert!(atan2f(f32::NAN, 1.0).is_nan());
}
