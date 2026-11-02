// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive_fraction() {
    assert!((truncf(3.7) - 3.0).abs() < 1e-5);
}

#[test]
fn test_negative_fraction() {
    assert!((truncf(-3.7) - (-3.0)).abs() < 1e-5);
}

#[test]
fn test_small() {
    assert_eq!(truncf(0.9).to_bits(), 0.0_f32.to_bits());
    assert_eq!(truncf(-0.9).to_bits(), (-0.0_f32).to_bits());
}

#[test]
fn test_nan() {
    assert!(truncf(f32::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert_eq!(truncf(f32::INFINITY), f32::INFINITY);
    assert_eq!(truncf(f32::NEG_INFINITY), f32::NEG_INFINITY);
}
