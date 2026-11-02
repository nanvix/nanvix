// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

const PI: f64 = core::f64::consts::PI;

#[test]
fn test_zero() {
    assert!((tan(0.0)).abs() < 1e-10);
}

#[test]
fn test_pi_over_4() {
    assert!((tan(PI / 4.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_negative() {
    assert!((tan(-PI / 4.0) - (-1.0)).abs() < 1e-10);
}

#[test]
fn test_nan() {
    assert!(tan(f64::NAN).is_nan());
}

#[test]
fn test_infinity() {
    assert!(tan(f64::INFINITY).is_nan());
}

#[test]
fn test_matches_std_over_range() {
    // Sweep across many periods (within the Cody-Waite range) vs the host libm.
    let mut x: f64 = -1000.0;
    while x <= 1000.0 {
        let got: f64 = tan(x);
        let want: f64 = x.tan();
        // Skip points extremely close to a pole, where the true value is huge
        // and a relative comparison is dominated by reduction round-off.
        if want.abs() < 1e6 {
            assert!(
                (got - want).abs() <= want.abs() * 1e-13 + 1e-13,
                "tan({x}) = {got}, want {want}"
            );
        }
        x += 0.1;
    }
}

#[test]
fn test_large_argument() {
    for &x in &[1.0e5_f64, 3.0e5, -7.0e5, 123456.789] {
        let got: f64 = tan(x);
        let want: f64 = x.tan();
        if want.abs() < 1e6 {
            assert!((got - want).abs() <= want.abs() * 1e-11 + 1e-11);
        }
    }
}
