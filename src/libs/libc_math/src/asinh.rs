// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the inverse hyperbolic sine of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `asinh(x) = sign(x) * ln(|x| + sqrt(x^2 + 1))`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn asinh(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() || x == 0.0 {
        return x;
    }
    let a: f64 = x.abs();
    let r: f64 = crate::log::log(a + crate::sqrt::sqrt(a * a + 1.0));
    if x < 0.0 {
        -r
    } else {
        r
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert_eq!(asinh(0.0), 0.0);
    }

    #[test]
    fn test_positive() {
        assert!((asinh(1.0) - 0.881_373_587_019_543).abs() < 1e-10);
    }

    #[test]
    fn test_odd_symmetry() {
        assert!((asinh(-1.0) + 0.881_373_587_019_543).abs() < 1e-10);
    }

    #[test]
    fn test_infinity() {
        assert_eq!(asinh(f64::INFINITY), f64::INFINITY);
        assert_eq!(asinh(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    #[test]
    fn test_nan() {
        assert!(asinh(f64::NAN).is_nan());
    }
}
