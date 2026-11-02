// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive() {
    let mut i: f64 = 0.0;
    let f: f64 = unsafe { modf(3.75, &mut i) };
    assert!((i - 3.0).abs() < 1e-10);
    assert!((f - 0.75).abs() < 1e-10);
}

#[test]
fn test_negative() {
    let mut i: f64 = 0.0;
    let f: f64 = unsafe { modf(-3.75, &mut i) };
    assert!((i - (-3.0)).abs() < 1e-10);
    assert!((f - (-0.75)).abs() < 1e-10);
}

#[test]
fn test_integer() {
    let mut i: f64 = 0.0;
    let f: f64 = unsafe { modf(5.0, &mut i) };
    assert!((i - 5.0).abs() < 1e-10);
    assert!(f.abs() < 1e-10);
}

#[test]
fn test_zero() {
    let mut i: f64 = 0.0;
    let f: f64 = unsafe { modf(0.0, &mut i) };
    assert_eq!(i, 0.0);
    assert_eq!(f, 0.0);
}
