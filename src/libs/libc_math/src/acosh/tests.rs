// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_one() {
    assert!((acosh(1.0)).abs() < 1e-10);
}

#[test]
fn test_two() {
    assert!((acosh(2.0) - 1.316_957_896_924_816_6).abs() < 1e-10);
}

#[test]
fn test_below_one() {
    assert!(acosh(0.5).is_nan());
}

#[test]
fn test_nan() {
    assert!(acosh(f64::NAN).is_nan());
}
