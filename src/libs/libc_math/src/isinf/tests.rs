// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_isinff() {
    assert_eq!(__isinff(f32::INFINITY), 1);
    assert_eq!(__isinff(f32::NEG_INFINITY), 1);
    assert_eq!(__isinff(1.0), 0);
    assert_eq!(__isinff(f32::NAN), 0);
}

#[test]
fn test_isinfd() {
    assert_eq!(__isinfd(f64::INFINITY), 1);
    assert_eq!(__isinfd(f64::NEG_INFINITY), 1);
    assert_eq!(__isinfd(1.0), 0);
    assert_eq!(__isinfd(f64::NAN), 0);
}
