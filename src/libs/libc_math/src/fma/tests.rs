// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert_eq!(fma(2.0, 3.0, 4.0), 10.0);
    assert_eq!(fma(3.0, 4.0, 5.0), 17.0);
}

#[test]
fn test_zero_factor() {
    assert_eq!(fma(0.0, 5.0, 7.0), 7.0);
    assert_eq!(fma(5.0, 0.0, 7.0), 7.0);
}

#[test]
fn test_single_rounding() {
    // (1 + eps) * (1 - eps) = 1 - eps^2 exactly; adding -1 leaves -eps^2, which a single
    // rounding preserves but a naive multiply-then-add would round away to 0.
    let hi: f64 = 1.0 + f64::EPSILON;
    let lo: f64 = 1.0 - f64::EPSILON;
    assert_eq!(fma(hi, lo, -1.0), -(f64::EPSILON * f64::EPSILON));
    assert_eq!(hi * lo - 1.0, 0.0);
}

#[test]
fn test_nan() {
    assert!(fma(f64::NAN, 1.0, 1.0).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(fma(1.0, 1.0, f64::INFINITY), f64::INFINITY);
}
