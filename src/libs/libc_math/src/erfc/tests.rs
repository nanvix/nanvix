// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Reference values from a correctly-rounded erfc (e.g. glibc / MPFR). The
/// large-argument cases exercise the tail path where `1 - erf(x)` would cancel.
const CASES: [(f64, f64); 6] = [
    (0.5, 0.479_500_122_186_953_5),
    (1.0, 0.157_299_207_050_285_13),
    (2.0, 4.677_734_981_047_266e-3),
    (3.0, 2.209_049_699_858_544e-5),
    (5.0, 1.537_459_794_428_035e-12),
    (10.0, 2.088_487_583_762_544_7e-45),
];

#[test]
fn test_reference_values() {
    for &(x, want) in &CASES {
        let got: f64 = erfc(x);
        assert!((got - want).abs() <= want.abs() * 1e-13, "erfc({x}) = {got}, want {want}");
    }
}

#[test]
fn test_erfc_plus_erf_is_one() {
    for &x in &[-1.5_f64, -0.3, 0.0, 0.3, 0.9, 1.5, 2.5] {
        assert!((erfc(x) + crate::erf::erf(x) - 1.0).abs() <= 1e-14);
    }
}

#[test]
fn test_zero() {
    assert_eq!(erfc(0.0), 1.0);
}

#[test]
fn test_limits() {
    assert_eq!(erfc(f64::INFINITY), 0.0);
    assert_eq!(erfc(f64::NEG_INFINITY), 2.0);
    assert_eq!(erfc(30.0), 0.0); // underflows to zero
}

#[test]
fn test_nan() {
    assert!(erfc(f64::NAN).is_nan());
}
