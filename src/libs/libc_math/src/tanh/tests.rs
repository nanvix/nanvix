// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((tanh(0.0)).abs() < 1e-10);
}

#[test]
fn test_one() {
    assert!((tanh(1.0) - 0.761_594_155_955_764_9).abs() < 1e-10);
}

#[test]
fn test_odd_symmetry() {
    assert!((tanh(-1.0) + 0.761_594_155_955_764_9).abs() < 1e-10);
}

#[test]
fn test_saturation() {
    assert_eq!(tanh(30.0), 1.0);
    assert_eq!(tanh(-30.0), -1.0);
}

#[test]
fn test_nan() {
    assert!(tanh(f64::NAN).is_nan());
}

#[test]
fn test_signed_zero() {
    // tanh must preserve the sign of zero.
    assert_eq!(tanh(0.0).to_bits(), 0.0_f64.to_bits());
    assert_eq!(tanh(-0.0).to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn test_matches_std_over_range() {
    let mut x: f64 = -25.0;
    while x <= 25.0 {
        let got: f64 = tanh(x);
        let want: f64 = x.tanh();
        assert!((got - want).abs() <= want.abs() * 1e-14 + 1e-16, "tanh({x}) = {got}, want {want}");
        x += 0.011;
    }
}
