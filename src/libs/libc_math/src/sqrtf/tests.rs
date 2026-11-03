// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_perfect_squares() {
    assert!((sqrtf(4.0) - 2.0).abs() < 1e-5);
    assert!((sqrtf(9.0) - 3.0).abs() < 1e-5);
}

#[test]
fn test_non_perfect() {
    assert!((sqrtf(2.0) - 1.41421).abs() < 1e-4);
}

#[test]
fn test_negative() {
    assert!(sqrtf(-1.0).is_nan());
}

#[test]
fn test_zero() {
    assert_eq!(sqrtf(0.0), 0.0);
}
