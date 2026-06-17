// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the hyperbolic cosine of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `cosh(x) = (e^x + e^-x) / 2`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn cosh(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    let e: f64 = crate::exp::exp(x.abs());
    (e + 1.0 / e) * 0.5
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((cosh(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((cosh(1.0) - 1.543_080_634_815_243_7).abs() < 1e-10);
    }

    #[test]
    fn test_even_symmetry() {
        assert!((cosh(-1.0) - cosh(1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_infinity() {
        assert_eq!(cosh(f64::INFINITY), f64::INFINITY);
        assert_eq!(cosh(f64::NEG_INFINITY), f64::INFINITY);
    }

    #[test]
    fn test_nan() {
        assert!(cosh(f64::NAN).is_nan());
    }
}
