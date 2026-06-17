// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the hyperbolic tangent of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `tanh(x)`, saturating to `+/-1` for large magnitudes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tanh(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x > 20.0 {
        return 1.0;
    }
    if x < -20.0 {
        return -1.0;
    }
    let e2: f64 = crate::exp::exp(2.0 * x);
    (e2 - 1.0) / (e2 + 1.0)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((tanh(0.0)).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((tanh(1.0) - 0.761_594_155_955_764_9).abs() < 1e-10);
    }

    #[test]
    fn test_odd_symmetry() {
        assert!((tanh(-1.0) + 0.761_594_155_955_764_9).abs() < 1e-10);
    }

    #[test]
    fn test_saturation() {
        assert_eq!(tanh(30.0), 1.0);
        assert_eq!(tanh(-30.0), -1.0);
    }

    #[test]
    fn test_nan() {
        assert!(tanh(f64::NAN).is_nan());
    }
}
