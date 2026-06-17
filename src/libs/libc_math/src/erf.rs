// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the error function of `x`.
///
/// Uses the Abramowitz & Stegun 7.1.26 rational approximation.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `erf(x)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let sign: f64 = if x < 0.0 { -1.0 } else { 1.0 };
    let ax: f64 = x.abs();
    let t: f64 = 1.0 / (1.0 + 0.3275911 * ax);
    let poly: f64 =
        ((((1.061_405_429 * t - 1.453_152_027) * t + 1.421_413_741) * t - 0.284_496_736) * t
            + 0.254_829_592)
            * t;
    let y: f64 = 1.0 - poly * crate::exp::exp(-ax * ax);
    sign * y
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((erf(0.0)).abs() < 1e-7);
    }

    #[test]
    fn test_one() {
        // The Abramowitz & Stegun approximation is accurate to about 1.5e-7.
        assert!((erf(1.0) - 0.842_700_792_949_714_9).abs() < 1e-6);
    }

    #[test]
    fn test_odd_symmetry() {
        assert!((erf(-1.0) + erf(1.0)).abs() < 1e-12);
    }

    #[test]
    fn test_large() {
        assert!((erf(5.0) - 1.0).abs() < 1e-6);
        assert!((erf(-5.0) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_nan() {
        assert!(erf(f64::NAN).is_nan());
    }
}
