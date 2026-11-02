// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((fmod(5.3, 2.0) - 1.3).abs() < 1e-10);
}

#[test]
fn test_negative() {
    assert!((fmod(-5.3, 2.0) - (-1.3)).abs() < 1e-10);
}

#[test]
fn test_exact() {
    assert!((fmod(6.0, 3.0)).abs() < 1e-10);
}

#[test]
fn test_zero_divisor() {
    assert!(fmod(5.0, 0.0).is_nan());
}

#[test]
fn test_nan() {
    assert!(fmod(f64::NAN, 2.0).is_nan());
    assert!(fmod(2.0, f64::NAN).is_nan());
}

#[test]
fn test_infinite_divisor_returns_x() {
    // fmod(x, ±inf) == x for finite x.
    assert_eq!(fmod(2.0, f64::INFINITY), 2.0);
    assert_eq!(fmod(2.0, f64::NEG_INFINITY), 2.0);
    assert_eq!(fmod(-3.5, f64::INFINITY), -3.5);
    assert_eq!(fmod(1e300, f64::INFINITY), 1e300);
}

#[test]
fn test_infinite_dividend_is_nan() {
    assert!(fmod(f64::INFINITY, 2.0).is_nan());
    assert!(fmod(f64::NEG_INFINITY, 2.0).is_nan());
}

#[test]
fn test_signed_zero_dividend() {
    // fmod(±0, y) == ±0, preserving the sign of zero.
    assert_eq!(fmod(0.0, 3.0).to_bits(), 0.0_f64.to_bits());
    assert_eq!(fmod(-0.0, 3.0).to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn test_exact_matches_rem_operator() {
    // Rust's `%` on f64 is the C `fmod`; the result must be bit-identical.
    let xs: [f64; 9] = [5.3, -5.3, 6.0, 1e300, -1e-300, 12345.678, -0.1, 3.0, 2.5];
    let ys: [f64; 7] = [2.0, 3.0, 0.5, 1e-100, 7.0, -2.0, 1.25];
    for &x in &xs {
        for &y in &ys {
            assert_eq!(fmod(x, y).to_bits(), (x % y).to_bits(), "fmod({x}, {y}) mismatch");
        }
    }
}

#[test]
fn test_subnormal_operands() {
    let tiny: f64 = f64::from_bits(0x0000_0000_0000_0007); // subnormal
    assert_eq!(fmod(tiny, tiny).to_bits(), 0.0_f64.to_bits());
    assert_eq!(fmod(1.0, tiny).to_bits(), (1.0_f64 % tiny).to_bits());
}
