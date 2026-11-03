// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_one() {
    assert!((logf(1.0)).abs() < 1e-5);
}

#[test]
fn test_e() {
    assert!((logf(2.718_282) - 1.0).abs() < 1e-4);
}

#[test]
fn test_zero() {
    assert_eq!(logf(0.0), f32::NEG_INFINITY);
}

#[test]
fn test_negative() {
    assert!(logf(-1.0).is_nan());
}
