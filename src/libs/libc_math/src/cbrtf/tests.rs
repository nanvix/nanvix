// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_perfect_cube() {
    assert!((cbrtf(8.0) - 2.0).abs() < 1e-5);
}

#[test]
fn test_negative() {
    assert!((cbrtf(-8.0) - (-2.0)).abs() < 1e-5);
}

#[test]
fn test_zero() {
    assert_eq!(cbrtf(0.0), 0.0);
}
