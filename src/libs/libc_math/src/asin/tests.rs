// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((asin(0.0)).abs() < 1e-10);
}

#[test]
fn test_half() {
    let pi_over_6: f64 = 3.141_592_653_589_793 / 6.0;
    assert!((asin(0.5) - pi_over_6).abs() < 1e-10);
}

#[test]
fn test_one() {
    assert!((asin(1.0) - FRAC_PI_2).abs() < 1e-10);
}

#[test]
fn test_negative_one() {
    assert!((asin(-1.0) - (-FRAC_PI_2)).abs() < 1e-10);
}

#[test]
fn test_out_of_range() {
    assert!(asin(1.5).is_nan());
}

#[test]
fn test_nan() {
    assert!(asin(f64::NAN).is_nan());
}
