// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive() {
    let result: f32 = fabsf(3.14);
    assert!((result - 3.14).abs() < 1e-5);
}

#[test]
fn test_negative() {
    let result: f32 = fabsf(-2.72);
    assert!((result - 2.72).abs() < 1e-5);
}

#[test]
fn test_zero() {
    assert_eq!(fabsf(0.0).to_bits(), 0u32);
    assert_eq!(fabsf(-0.0).to_bits(), 0u32);
}

#[test]
fn test_nan() {
    assert!(fabsf(f32::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(fabsf(f32::INFINITY), f32::INFINITY);
    assert_eq!(fabsf(f32::NEG_INFINITY), f32::INFINITY);
}
