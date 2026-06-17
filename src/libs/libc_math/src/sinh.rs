// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the hyperbolic sine of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `sinh(x) = (e^x - e^-x) / 2`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn sinh(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() {
        return x;
    }
    let e: f64 = crate::exp::exp(x);
    (e - 1.0 / e) * 0.5
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((sinh(0.0)).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((sinh(1.0) - 1.175_201_193_643_801_4).abs() < 1e-10);
    }

    #[test]
    fn test_odd_symmetry() {
        assert!((sinh(-1.0) + 1.175_201_193_643_801_4).abs() < 1e-10);
    }

    #[test]
    fn test_infinity() {
        assert_eq!(sinh(f64::INFINITY), f64::INFINITY);
        assert_eq!(sinh(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    #[test]
    fn test_nan() {
        assert!(sinh(f64::NAN).is_nan());
    }
}
