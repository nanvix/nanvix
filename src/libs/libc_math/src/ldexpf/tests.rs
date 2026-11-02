// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((ldexpf(1.0, 3) - 8.0).abs() < 1e-5);
}

#[test]
fn test_negative_exp() {
    assert!((ldexpf(8.0, -3) - 1.0).abs() < 1e-5);
}

#[test]
fn test_zero() {
    assert_eq!(ldexpf(0.0, 10), 0.0);
}
