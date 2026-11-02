// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_integer_factorials() {
    // lgamma(n) == ln((n - 1)!).
    assert!((lgamma(1.0)).abs() < 1e-9);
    assert!((lgamma(2.0)).abs() < 1e-9);
    assert!((lgamma(3.0) - core::f64::consts::LN_2).abs() < 1e-9);
    assert!((lgamma(5.0) - 3.178_053_830_347_945).abs() < 1e-9);
}

#[test]
fn test_half() {
    // lgamma(0.5) == ln(sqrt(pi)).
    assert!((lgamma(0.5) - 0.572_364_942_924_700_4).abs() < 1e-9);
}

#[test]
fn test_reflection() {
    // lgamma is finite for negative non-integer arguments.
    assert!(lgamma(-0.5).is_finite());
}

#[test]
fn test_nan() {
    assert!(lgamma(f64::NAN).is_nan());
}
