// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_toward_larger() {
    assert_eq!(nextafter(1.0, 2.0), 1.0 + f64::EPSILON);
}

#[test]
fn test_toward_smaller() {
    assert!(nextafter(1.0, 0.0) < 1.0);
    assert_eq!(nextafter(1.0, 0.0), 1.0 - f64::EPSILON / 2.0);
}

#[test]
fn test_equal() {
    assert_eq!(nextafter(3.5, 3.5), 3.5);
}

#[test]
fn test_from_zero() {
    assert_eq!(nextafter(0.0, 1.0), f64::from_bits(1));
    assert_eq!(nextafter(0.0, -1.0), -f64::from_bits(1));
}

#[test]
fn test_nan() {
    assert!(nextafter(f64::NAN, 1.0).is_nan());
    assert!(nextafter(1.0, f64::NAN).is_nan());
}
