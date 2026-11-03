// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive_fraction() {
    assert!((floorf(2.7) - 2.0).abs() < 1e-5);
}

#[test]
fn test_negative_fraction() {
    assert!((floorf(-2.3) - (-3.0)).abs() < 1e-5);
}

#[test]
fn test_nan() {
    assert!(floorf(f32::NAN).is_nan());
}
