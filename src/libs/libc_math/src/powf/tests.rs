// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_square() {
    assert!((powf(2.0, 2.0) - 4.0).abs() < 1e-3);
}

#[test]
fn test_zero_exp() {
    assert!((powf(5.0, 0.0) - 1.0).abs() < 1e-5);
}

#[test]
fn test_nan() {
    assert!(powf(f32::NAN, 2.0).is_nan());
}
