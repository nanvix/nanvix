// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_round_up() {
    assert!((roundf(2.5) - 3.0).abs() < 1e-5);
}

#[test]
fn test_round_down() {
    assert!((roundf(2.3) - 2.0).abs() < 1e-5);
}

#[test]
fn test_negative() {
    assert!((roundf(-2.5) - (-3.0)).abs() < 1e-5);
}

#[test]
fn test_nan() {
    assert!(roundf(f32::NAN).is_nan());
}
