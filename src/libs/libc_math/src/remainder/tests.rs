// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert_eq!(remainder(5.0, 3.0), -1.0);
    assert_eq!(remainder(7.0, 4.0), -1.0);
}

#[test]
fn test_ties_to_even() {
    assert_eq!(remainder(5.0, 2.0), 1.0);
}

#[test]
fn test_infinite_divisor() {
    assert_eq!(remainder(1.0, f64::INFINITY), 1.0);
}

#[test]
fn test_domain_errors() {
    assert!(remainder(1.0, 0.0).is_nan());
    assert!(remainder(f64::INFINITY, 1.0).is_nan());
    assert!(remainder(f64::NAN, 1.0).is_nan());
}
