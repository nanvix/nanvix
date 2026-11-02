// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive() {
    assert_eq!(imaxabs(42), 42);
}

#[test]
fn test_negative() {
    assert_eq!(imaxabs(-42), 42);
}

#[test]
fn test_zero() {
    assert_eq!(imaxabs(0), 0);
}

#[test]
fn test_min() {
    // wrapping_abs of i64::MIN returns i64::MIN.
    assert_eq!(imaxabs(i64::MIN), i64::MIN);
}
