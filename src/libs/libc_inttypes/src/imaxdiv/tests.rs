// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_normal() {
    let result = imaxdiv(10, 3);
    assert_eq!(result.quot, 3);
    assert_eq!(result.rem, 1);
}

#[test]
fn test_negative_numer() {
    let result = imaxdiv(-10, 3);
    assert_eq!(result.quot, -3);
    assert_eq!(result.rem, -1);
}

#[test]
fn test_both_negative() {
    let result = imaxdiv(-10, -3);
    assert_eq!(result.quot, 3);
    assert_eq!(result.rem, -1);
}

#[test]
fn test_exact() {
    let result = imaxdiv(12, 4);
    assert_eq!(result.quot, 3);
    assert_eq!(result.rem, 0);
}

#[test]
fn test_divide_by_zero() {
    // Division by zero is undefined in C; this implementation returns {0, 0} without trapping.
    let result = imaxdiv(10, 0);
    assert_eq!(result.quot, 0);
    assert_eq!(result.rem, 0);
}

#[test]
fn test_overflow() {
    // INTMAX_MIN / -1 is not representable; this implementation returns {0, 0} without trapping.
    let result = imaxdiv(intmax_t::MIN, -1);
    assert_eq!(result.quot, 0);
    assert_eq!(result.rem, 0);
}
