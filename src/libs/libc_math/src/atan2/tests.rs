// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_first_quadrant() {
    assert!((atan2(1.0, 1.0) - PI / 4.0).abs() < 1e-10);
}

#[test]
fn test_second_quadrant() {
    assert!((atan2(1.0, -1.0) - 3.0 * PI / 4.0).abs() < 1e-10);
}

#[test]
fn test_third_quadrant() {
    assert!((atan2(-1.0, -1.0) - (-3.0 * PI / 4.0)).abs() < 1e-10);
}

#[test]
fn test_axes() {
    assert!((atan2(1.0, 0.0) - FRAC_PI_2).abs() < 1e-10);
    assert!((atan2(-1.0, 0.0) - (-FRAC_PI_2)).abs() < 1e-10);
    assert!((atan2(0.0, 1.0)).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!(atan2(f64::NAN, 1.0).is_nan());
}
