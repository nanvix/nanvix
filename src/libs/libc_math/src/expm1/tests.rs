// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_zero() {
    assert_eq!(expm1(0.0), 0.0);
}

#[test]
fn test_one() {
    assert!((expm1(1.0) - 1.718_281_828_459_045).abs() < 1e-10);
}

#[test]
fn test_negative() {
    assert!((expm1(-1.0) + 0.632_120_558_828_557_7).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!(expm1(f64::NAN).is_nan());
}

#[test]
fn test_signed_zero() {
    assert_eq!(expm1(0.0).to_bits(), 0.0_f64.to_bits());
    assert_eq!(expm1(-0.0).to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn test_small_precision() {
    // Near zero, expm1(x) must not lose precision the way exp(x) - 1 does.
    for &x in &[1e-8_f64, -1e-8, 1e-5, -1e-5, 1e-12] {
        let want: f64 = x.exp_m1();
        assert!((expm1(x) - want).abs() <= want.abs() * 1e-14 + 1e-300);
    }
}

#[test]
fn test_matches_std_over_range() {
    let mut x: f64 = -40.0;
    while x <= 700.0 {
        let got: f64 = expm1(x);
        let want: f64 = x.exp_m1();
        assert!(
            (got - want).abs() <= want.abs() * 1e-14 + 1e-300,
            "expm1({x}) = {got}, want {want}"
        );
        x += 0.017;
    }
}
