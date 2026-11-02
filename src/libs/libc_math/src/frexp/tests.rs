// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    let mut e: c_int = 0;
    let f: f64 = unsafe { frexp(8.0, &mut e) };
    assert!((f - 0.5).abs() < 1e-10);
    assert_eq!(e, 4);
}

#[test]
fn test_fraction() {
    let mut e: c_int = 0;
    let f: f64 = unsafe { frexp(0.75, &mut e) };
    assert!((f - 0.75).abs() < 1e-10);
    assert_eq!(e, 0);
}

#[test]
fn test_one() {
    let mut e: c_int = 0;
    let f: f64 = unsafe { frexp(1.0, &mut e) };
    assert!((f - 0.5).abs() < 1e-10);
    assert_eq!(e, 1);
}

#[test]
fn test_zero() {
    let mut e: c_int = 0;
    let f: f64 = unsafe { frexp(0.0, &mut e) };
    assert_eq!(f, 0.0);
    assert_eq!(e, 0);
}

#[test]
fn test_negative() {
    let mut e: c_int = 0;
    let f: f64 = unsafe { frexp(-4.0, &mut e) };
    assert!((f - (-0.5)).abs() < 1e-10);
    assert_eq!(e, 3);
}
