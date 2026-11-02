// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert!((exp(0.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_one() {
    assert!((exp(1.0) - 2.718_281_828_459_045).abs() < 1e-10);
}

#[test]
fn test_negative() {
    assert!((exp(-1.0) - 0.367_879_441_171_442_3).abs() < 1e-10);
}

#[test]
fn test_ln2() {
    assert!((exp(core::f64::consts::LN_2) - 2.0).abs() < 1e-10);
}

#[test]
fn test_matches_std_over_range() {
    // Compare against the host libm across the full finite range at <= a few ULP.
    let mut x: f64 = -700.0;
    while x <= 700.0 {
        let got: f64 = exp(x);
        let want: f64 = x.exp();
        assert!((got - want).abs() <= want.abs() * 1e-14, "exp({x}) = {got}, want {want}");
        x += 0.013;
    }
}

#[test]
fn test_overflow() {
    assert_eq!(exp(710.0), f64::INFINITY);
}

#[test]
fn test_underflow() {
    assert_eq!(exp(-746.0), 0.0);
}

#[test]
fn test_nan() {
    assert!(exp(f64::NAN).is_nan());
}

#[test]
fn test_overflow_boundary_is_finite() {
    // Inputs below `ln(f64::MAX)` (~= 709.7827) must not overflow prematurely.
    let v: f64 = exp(709.5);
    let expected: f64 = 709.5f64.exp();
    assert!(v.is_finite() && v > 0.0);
    assert!((v - expected).abs() <= expected * 1e-10);
}

#[test]
fn test_underflow_boundary_is_subnormal() {
    // Inputs above `ln(2^-1075)` (~= -745.133) must not underflow to zero.
    let v: f64 = exp(-745.1);
    assert!(v > 0.0);
    assert!(v < f64::MIN_POSITIVE);
}
