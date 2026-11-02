// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_ties_to_even() {
    // Halfway cases round to the nearest even integer.
    assert_eq!(lrint(0.5), 0);
    assert_eq!(lrint(1.5), 2);
    assert_eq!(lrint(2.5), 2);
    assert_eq!(lrint(3.5), 4);
    assert_eq!(lrint(-0.5), 0);
    assert_eq!(lrint(-1.5), -2);
    assert_eq!(lrint(-2.5), -2);
}

#[test]
fn test_non_ties() {
    assert_eq!(lrint(2.3), 2);
    assert_eq!(lrint(2.7), 3);
    assert_eq!(lrint(-2.3), -2);
    assert_eq!(lrint(-2.7), -3);
    assert_eq!(lrint(0.0), 0);
    assert_eq!(lrint(7.0), 7);
}

#[test]
fn test_rounding_modes() {
    use crate::fenv;

    // Round toward negative infinity.
    assert_eq!(fenv::fesetround(fenv::FE_DOWNWARD), 0);
    assert_eq!(lrint(2.7), 2);
    assert_eq!(lrint(-2.3), -3);

    // Round toward positive infinity.
    assert_eq!(fenv::fesetround(fenv::FE_UPWARD), 0);
    assert_eq!(lrint(2.3), 3);
    assert_eq!(lrint(-2.7), -2);

    // Round toward zero.
    assert_eq!(fenv::fesetround(fenv::FE_TOWARDZERO), 0);
    assert_eq!(lrint(2.7), 2);
    assert_eq!(lrint(-2.7), -2);

    // Restore the default mode and confirm ties-to-even.
    assert_eq!(fenv::fesetround(fenv::FE_TONEAREST), 0);
    assert_eq!(lrint(2.5), 2);
}
