// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the inverse hyperbolic tangent of `x`.
///
/// # Parameters
///
/// - `x`: Input value (must satisfy `|x| <= 1`).
///
/// # Returns
///
/// The value `atanh(x) = 0.5 * ln((1 + x) / (1 - x))`, or NaN for `|x| > 1`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn atanh(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x == 1.0 {
        return f64::INFINITY;
    }
    if x == -1.0 {
        return f64::NEG_INFINITY;
    }
    if x.abs() > 1.0 {
        return f64::NAN;
    }
    0.5 * crate::log::log((1.0 + x) / (1.0 - x))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert_eq!(atanh(0.0), 0.0);
    }

    #[test]
    fn test_half() {
        assert!((atanh(0.5) - 0.549_306_144_334_054_9).abs() < 1e-10);
    }

    #[test]
    fn test_odd_symmetry() {
        assert!((atanh(-0.5) + 0.549_306_144_334_054_9).abs() < 1e-10);
    }

    #[test]
    fn test_unit_poles() {
        assert_eq!(atanh(1.0), f64::INFINITY);
        assert_eq!(atanh(-1.0), f64::NEG_INFINITY);
    }

    #[test]
    fn test_out_of_domain() {
        assert!(atanh(2.0).is_nan());
        assert!(atanh(-2.0).is_nan());
    }

    #[test]
    fn test_nan() {
        assert!(atanh(f64::NAN).is_nan());
    }
}
