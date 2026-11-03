// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::lldiv;

#[test]
fn positive_division() {
    let result = unsafe { lldiv(100, 7) };
    assert_eq!(result.quot, 14);
    assert_eq!(result.rem, 2);
}

#[test]
fn large_values() {
    let result = unsafe { lldiv(1_000_000_000_000, 7) };
    assert_eq!(result.quot, 142_857_142_857);
    assert_eq!(result.rem, 1);
}
