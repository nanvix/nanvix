// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_one() {
    assert!((acos(1.0)).abs() < 1e-10);
}

#[test]
fn test_zero() {
    assert!((acos(0.0) - FRAC_PI_2).abs() < 1e-10);
}

#[test]
fn test_negative_one() {
    assert!((acos(-1.0) - PI).abs() < 1e-10);
}

#[test]
fn test_half() {
    let pi_over_3: f64 = PI / 3.0;
    assert!((acos(0.5) - pi_over_3).abs() < 1e-10);
}

#[test]
fn test_out_of_range() {
    assert!(acos(1.5).is_nan());
}
