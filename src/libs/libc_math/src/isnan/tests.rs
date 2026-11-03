// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_isnanf() {
    assert_eq!(__isnanf(f32::NAN), 1);
    assert_eq!(__isnanf(1.0), 0);
    assert_eq!(__isnanf(f32::INFINITY), 0);
}

#[test]
fn test_isnand() {
    assert_eq!(__isnand(f64::NAN), 1);
    assert_eq!(__isnand(1.0), 0);
    assert_eq!(__isnand(f64::INFINITY), 0);
}
