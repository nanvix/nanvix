// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_fpclassifyf() {
    assert_eq!(__fpclassifyf(1.0), FP_NORMAL);
    assert_eq!(__fpclassifyf(0.0), FP_ZERO);
    assert_eq!(__fpclassifyf(f32::INFINITY), FP_INFINITE);
    assert_eq!(__fpclassifyf(f32::NAN), FP_NAN);
    assert_eq!(__fpclassifyf(1.0e-40), FP_SUBNORMAL);
}

#[test]
fn test_fpclassifyd() {
    assert_eq!(__fpclassifyd(1.0), FP_NORMAL);
    assert_eq!(__fpclassifyd(0.0), FP_ZERO);
    assert_eq!(__fpclassifyd(f64::INFINITY), FP_INFINITE);
    assert_eq!(__fpclassifyd(f64::NAN), FP_NAN);
    assert_eq!(__fpclassifyd(5.0e-324), FP_SUBNORMAL);
}
