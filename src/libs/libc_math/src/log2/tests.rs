// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_one() {
    assert!((log2(1.0)).abs() < 1e-10);
}

#[test]
fn test_two() {
    assert!((log2(2.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_eight() {
    assert!((log2(8.0) - 3.0).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert_eq!(log2(0.0), f64::NEG_INFINITY);
}

#[test]
fn test_negative() {
    assert!(log2(-1.0).is_nan());
}
