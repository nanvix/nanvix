// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert_eq!(atanh(0.0), 0.0);
}

#[test]
fn test_half() {
    assert!((atanh(0.5) - 0.549_306_144_334_054_9).abs() < 1e-10);
}

#[test]
fn test_odd_symmetry() {
    assert!((atanh(-0.5) + 0.549_306_144_334_054_9).abs() < 1e-10);
}

#[test]
fn test_unit_poles() {
    assert_eq!(atanh(1.0), f64::INFINITY);
    assert_eq!(atanh(-1.0), f64::NEG_INFINITY);
}

#[test]
fn test_out_of_domain() {
    assert!(atanh(2.0).is_nan());
    assert!(atanh(-2.0).is_nan());
}

#[test]
fn test_nan() {
    assert!(atanh(f64::NAN).is_nan());
}
