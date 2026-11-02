// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert_eq!(fmaf(2.0, 3.0, 4.0), 10.0);
    assert_eq!(fmaf(3.0, 4.0, 5.0), 17.0);
}

#[test]
fn test_zero_factor() {
    assert_eq!(fmaf(0.0, 5.0, 7.0), 7.0);
}

#[test]
fn test_nan() {
    assert!(fmaf(f32::NAN, 1.0, 1.0).is_nan());
}
