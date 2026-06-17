// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sysapi::ffi::c_long;

/// Rounds `x` to the nearest integer (per the current rounding mode) as a `long`.
///
/// `lrint` rounds according to the current rounding mode (see `fesetround`),
/// whose default is round-to-nearest, ties-to-even (IEEE-754 `roundTiesToEven`).
/// In the default mode this differs from C `round()` (ties away from zero): e.g.
/// `lrint(0.5) == 0`, `lrint(2.5) == 2`, while `round(0.5) == 1`. Code that
/// depends on the default mode (for example ECMAScript `Uint8ClampedArray`
/// element conversion, which is specified as round-half-to-even) relies on this.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The rounded integer value of `x`, as a `long`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn lrint(x: f64) -> c_long {
    let f: f64 = crate::floor::floor(x);
    let diff: f64 = x - f;
    let rounded: f64 = match crate::fenv::fegetround() {
        // Round toward negative infinity.
        crate::fenv::FE_DOWNWARD => f,
        // Round toward positive infinity.
        crate::fenv::FE_UPWARD => {
            if diff > 0.0 {
                f + 1.0
            } else {
                f
            }
        },
        // Round toward zero (truncate).
        crate::fenv::FE_TOWARDZERO => crate::trunc::trunc(x),
        // Round to nearest, ties to even (`FE_TONEAREST`, the default).
        _ => {
            if diff < 0.5 {
                f
            } else if diff > 0.5 {
                f + 1.0
            } else {
                // Exact tie: round to the even neighbor. `f` is integral, so it
                // is even iff `f / 2` is also integral (`floor(f / 2) == f / 2`).
                // This holds for every representable magnitude, unlike an
                // integer-cast parity check.
                let half: f64 = f * 0.5;
                if crate::floor::floor(half) == half {
                    f
                } else {
                    f + 1.0
                }
            }
        },
    };
    rounded as c_long
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
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
}
