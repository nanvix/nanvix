// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_integer_powers() {
    assert_eq!(exp2(0.0), 1.0);
    assert_eq!(exp2(3.0), 8.0);
    assert_eq!(exp2(10.0), 1024.0);
    assert_eq!(exp2(-1.0), 0.5);
}

#[test]
fn test_half() {
    assert!((exp2(0.5) - core::f64::consts::SQRT_2).abs() < 1e-10);
}

#[test]
fn test_overflow() {
    assert_eq!(exp2(1024.0), f64::INFINITY);
}

#[test]
fn test_underflow() {
    assert_eq!(exp2(-1075.0), 0.0);
}

#[test]
fn test_nan() {
    assert!(exp2(f64::NAN).is_nan());
}
