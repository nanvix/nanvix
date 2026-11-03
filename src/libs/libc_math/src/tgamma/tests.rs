// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_integer_factorials() {
    assert!((tgamma(1.0) - 1.0).abs() < 1e-9);
    assert!((tgamma(2.0) - 1.0).abs() < 1e-9);
    assert!((tgamma(3.0) - 2.0).abs() < 1e-9);
    assert!((tgamma(4.0) - 6.0).abs() < 1e-9);
    assert!((tgamma(5.0) - 24.0).abs() < 1e-8);
}

#[test]
fn test_half() {
    // tgamma(0.5) == sqrt(pi).
    assert!((tgamma(0.5) - 1.772_453_850_905_515_9).abs() < 1e-9);
}

#[test]
fn test_nan() {
    assert!(tgamma(f64::NAN).is_nan());
}

#[test]
fn test_poles() {
    // Γ(+0) = +∞ and Γ(-0) = -∞: the sign of the zero is preserved.
    assert!(tgamma(0.0).is_infinite() && tgamma(0.0).is_sign_positive());
    assert!(tgamma(-0.0).is_infinite() && tgamma(-0.0).is_sign_negative());

    // Negative integers are domain errors and yield NaN.
    assert!(tgamma(-1.0).is_nan());
    assert!(tgamma(-2.0).is_nan());
    assert!(tgamma(-3.0).is_nan());
}
