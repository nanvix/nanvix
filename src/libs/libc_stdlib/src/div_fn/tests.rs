// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::div;

#[test]
fn positive_division() {
    let result = unsafe { div(10, 3) };
    assert_eq!(result.quot, 3);
    assert_eq!(result.rem, 1);
}

#[test]
fn negative_dividend() {
    let result = unsafe { div(-10, 3) };
    assert_eq!(result.quot, -3);
    assert_eq!(result.rem, -1);
}
