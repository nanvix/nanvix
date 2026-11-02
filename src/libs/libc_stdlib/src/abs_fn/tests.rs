// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::abs;

#[test]
fn positive_value() {
    assert_eq!(unsafe { abs(42) }, 42);
}

#[test]
fn negative_value() {
    assert_eq!(unsafe { abs(-42) }, 42);
}

#[test]
fn zero_value() {
    assert_eq!(unsafe { abs(0) }, 0);
}
