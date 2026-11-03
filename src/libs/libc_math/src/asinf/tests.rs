// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((asinf(0.0)).abs() < 1e-5);
}

#[test]
fn test_one() {
    assert!((asinf(1.0) - FRAC_PI_2).abs() < 1e-4);
}

#[test]
fn test_out_of_range() {
    assert!(asinf(1.5).is_nan());
}
