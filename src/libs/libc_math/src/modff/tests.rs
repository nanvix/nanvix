// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive() {
    let mut i: f32 = 0.0;
    let f: f32 = unsafe { modff(3.75, &mut i) };
    assert!((i - 3.0).abs() < 1e-5);
    assert!((f - 0.75).abs() < 1e-5);
}

#[test]
fn test_negative() {
    let mut i: f32 = 0.0;
    let f: f32 = unsafe { modff(-3.75, &mut i) };
    assert!((i - (-3.0)).abs() < 1e-5);
    assert!((f - (-0.75)).abs() < 1e-5);
}

#[test]
fn test_zero() {
    let mut i: f32 = 0.0;
    let f: f32 = unsafe { modff(0.0, &mut i) };
    assert_eq!(i, 0.0);
    assert_eq!(f, 0.0);
}
