// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((fmaxf(2.0, 3.0) - 3.0).abs() < 1e-5);
}

#[test]
fn test_nan() {
    assert!((fmaxf(f32::NAN, 2.0) - 2.0).abs() < 1e-5);
}

#[test]
fn test_signed_zero() {
    // +0.0 is treated as larger than -0.0, regardless of argument order.
    assert!(!fmaxf(-0.0, 0.0).is_sign_negative());
    assert!(!fmaxf(0.0, -0.0).is_sign_negative());
}
