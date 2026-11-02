// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((expf(0.0) - 1.0).abs() < 1e-5);
}

#[test]
fn test_one() {
    assert!((expf(1.0) - 2.718_28).abs() < 1e-3);
}

#[test]
fn test_overflow() {
    assert_eq!(expf(89.0), f32::INFINITY);
}

#[test]
fn test_nan() {
    assert!(expf(f32::NAN).is_nan());
}

#[test]
fn test_overflow_boundary_is_finite() {
    // Inputs below `ln(f32::MAX)` (~= 88.7228) must not overflow prematurely.
    let v: f32 = expf(88.5);
    let expected: f32 = 88.5f32.exp();
    assert!(v.is_finite() && v > 0.0);
    assert!((v - expected).abs() <= expected * 1e-4);
}

#[test]
fn test_underflow_boundary_is_subnormal() {
    // Inputs above `ln(2^-150)` (~= -103.972) must not underflow to zero.
    let v: f32 = expf(-103.5);
    assert!(v > 0.0);
    assert!(v < f32::MIN_POSITIVE);
}
