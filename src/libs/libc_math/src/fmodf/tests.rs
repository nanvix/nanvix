// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((fmodf(5.3, 2.0) - 1.3).abs() < 1e-5);
}

#[test]
fn test_negative() {
    assert!((fmodf(-5.3, 2.0) - (-1.3)).abs() < 1e-5);
}

#[test]
fn test_zero_divisor() {
    assert!(fmodf(5.0, 0.0).is_nan());
}
