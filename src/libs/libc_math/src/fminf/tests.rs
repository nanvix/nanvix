// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((fminf(2.0, 3.0) - 2.0).abs() < 1e-5);
}

#[test]
fn test_nan() {
    assert!((fminf(f32::NAN, 2.0) - 2.0).abs() < 1e-5);
}

#[test]
fn test_signed_zero() {
    // -0.0 is treated as smaller than +0.0, regardless of argument order.
    assert!(fminf(-0.0, 0.0).is_sign_negative());
    assert!(fminf(0.0, -0.0).is_sign_negative());
}
