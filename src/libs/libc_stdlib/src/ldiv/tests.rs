// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ldiv;

#[test]
fn positive_division() {
    let result = unsafe { ldiv(17, 5) };
    assert_eq!(result.quot, 3);
    assert_eq!(result.rem, 2);
}

#[test]
fn negative_dividend() {
    let result = unsafe { ldiv(-17, 5) };
    assert_eq!(result.quot, -3);
    assert_eq!(result.rem, -2);
}
