// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_one() {
    assert!((log(1.0)).abs() < 1e-10);
}

#[test]
fn test_e() {
    assert!((log(2.718_281_828_459_045) - 1.0).abs() < 1e-10);
}

#[test]
fn test_two() {
    assert!((log(2.0) - core::f64::consts::LN_2).abs() < 1e-10);
}

#[test]
fn test_ten() {
    assert!((log(10.0) - 2.302_585_092_994_046).abs() < 1e-10);
}

#[test]
fn test_matches_std_over_range() {
    // Multiplicative sweep across the exponent range, compared to the host libm.
    let mut x: f64 = 1e-300;
    while x < 1e300 {
        let got: f64 = log(x);
        let want: f64 = x.ln();
        assert!((got - want).abs() <= want.abs() * 1e-14 + 1e-15, "log({x}) = {got}, want {want}");
        x *= 1.037;
    }
}

#[test]
fn test_subnormal() {
    let d: f64 = f64::from_bits(0x0000_0000_0000_1000); // subnormal
    assert!((log(d) - d.ln()).abs() <= d.ln().abs() * 1e-14);
}

#[test]
fn test_zero() {
    assert_eq!(log(0.0), f64::NEG_INFINITY);
}

#[test]
fn test_negative() {
    assert!(log(-1.0).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(log(f64::INFINITY), f64::INFINITY);
}

#[test]
fn test_nan() {
    assert!(log(f64::NAN).is_nan());
}
