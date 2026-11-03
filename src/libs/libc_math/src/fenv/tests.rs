// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_rounding_mode_roundtrip() {
    // The default rounding mode is round-to-nearest.
    assert_eq!(fegetround(), FE_TONEAREST);

    for &mode in &[FE_DOWNWARD, FE_UPWARD, FE_TOWARDZERO, FE_TONEAREST] {
        assert_eq!(fesetround(mode), 0);
        assert_eq!(fegetround(), mode);
    }

    // Restore the default so other tests observe a clean environment.
    assert_eq!(fesetround(FE_TONEAREST), 0);
}

#[test]
fn test_invalid_rounding_mode() {
    // An unrecognized mode is rejected and does not change the environment.
    assert_ne!(fesetround(0x1234), 0);
}

#[test]
fn test_exceptions_are_noops() {
    assert_eq!(feclearexcept(0x3f), 0);
    assert_eq!(feraiseexcept(0x3f), 0);
    assert_eq!(fetestexcept(0x3f), 0);
}
