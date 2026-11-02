// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Reference values from a correctly-rounded erf (e.g. glibc / MPFR).
const CASES: [(f64, f64); 7] = [
    (0.0, 0.0),
    (0.1, 0.112_462_916_018_284_89),
    (0.5, 0.520_499_877_813_046_5),
    (1.0, 0.842_700_792_949_714_9),
    (2.0, 0.995_322_265_018_952_7),
    (3.0, 0.999_977_909_503_001_4),
    (4.0, 0.999_999_984_582_742_1),
];

#[test]
fn test_reference_values() {
    for &(x, want) in &CASES {
        let got: f64 = erf(x);
        assert!((got - want).abs() <= want.abs() * 1e-14 + 1e-15, "erf({x}) = {got}, want {want}");
        // erf is odd.
        let got_neg: f64 = erf(-x);
        assert!((got_neg + want).abs() <= want.abs() * 1e-14 + 1e-15);
    }
}

#[test]
fn test_zero() {
    assert_eq!(erf(0.0), 0.0);
    assert_eq!(erf(-0.0).to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn test_limits() {
    assert_eq!(erf(f64::INFINITY), 1.0);
    assert_eq!(erf(f64::NEG_INFINITY), -1.0);
}

#[test]
fn test_nan() {
    assert!(erf(f64::NAN).is_nan());
}
