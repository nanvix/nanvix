// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::labs;

#[test]
fn positive_value() {
    assert_eq!(unsafe { labs(42) }, 42);
}

#[test]
fn negative_value() {
    assert_eq!(unsafe { labs(-42) }, 42);
}

#[test]
fn zero_value() {
    assert_eq!(unsafe { labs(0) }, 0);
}
