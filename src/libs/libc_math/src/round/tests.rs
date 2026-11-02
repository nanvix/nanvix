// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_round_up() {
    assert!((round(2.5) - 3.0).abs() < 1e-10);
}

#[test]
fn test_round_down() {
    assert!((round(2.3) - 2.0).abs() < 1e-10);
}

#[test]
fn test_negative_round() {
    assert!((round(-2.5) - (-3.0)).abs() < 1e-10);
    assert!((round(-2.3) - (-2.0)).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!(round(f64::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(round(f64::INFINITY), f64::INFINITY);
}
