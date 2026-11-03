// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive_to_negative() {
    assert!((copysign(3.0, -1.0) - (-3.0)).abs() < 1e-10);
}

#[test]
fn test_negative_to_positive() {
    assert!((copysign(-3.0, 1.0) - 3.0).abs() < 1e-10);
}

#[test]
fn test_same_sign() {
    assert!((copysign(3.0, 1.0) - 3.0).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert_eq!(copysign(0.0, -1.0).to_bits(), (-0.0_f64).to_bits());
}
