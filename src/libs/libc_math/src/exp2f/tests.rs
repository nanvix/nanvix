// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_integer_powers() {
    assert_eq!(exp2f(0.0), 1.0);
    assert_eq!(exp2f(3.0), 8.0);
    assert_eq!(exp2f(-1.0), 0.5);
}

#[test]
fn test_half() {
    assert!((exp2f(0.5) - core::f32::consts::SQRT_2).abs() < 1e-6);
}

#[test]
fn test_nan() {
    assert!(exp2f(f32::NAN).is_nan());
}
