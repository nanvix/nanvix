// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::llabs;

#[test]
fn positive_value() {
    assert_eq!(unsafe { llabs(42) }, 42);
}

#[test]
fn negative_value() {
    assert_eq!(unsafe { llabs(-42) }, 42);
}

#[test]
fn large_value() {
    assert_eq!(unsafe { llabs(-1_000_000_000_000) }, 1_000_000_000_000);
}
