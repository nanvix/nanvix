// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `x` raised to the power `y`.
///
/// # Description
///
/// Integer exponents are evaluated by exact binary exponentiation (so results such as
/// `pow(2, -1022)` reproduce the exact IEEE-754 value), and fractional exponents fall back to
/// `exp(y * log(x))` with special-case handling per IEEE 754.
///
/// # Parameters
///
/// - `x`: Base.
/// - `y`: Exponent.
///
/// # Returns
///
/// The value `x^y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn pow(x: f64, y: f64) -> f64 {
    // x^0 = 1 for any x, including NaN (C Annex F.10.4.4).
    if y == 0.0 {
        return 1.0;
    }

    // 1^y = 1 for any y, including NaN (C Annex F.10.4.4).
    if x == 1.0 {
        return 1.0;
    }

    // Any other NaN operand propagates NaN.
    if x.is_nan() || y.is_nan() {
        return f64::from_bits(0x7FF8_0000_0000_0000);
    }

    // x^1 = x.
    if y == 1.0 {
        return x;
    }

    // pow(-1, ±inf) == 1 (C Annex F.10.4.4 / IEEE-754).
    if x == -1.0 && y.is_infinite() {
        return 1.0;
    }

    // Handle x == 0, preserving the sign of zero per IEEE 754 / C Annex F.10.4.4.
    // The result is negative only when the base is -0 and the exponent is an odd
    // integer.
    if x == 0.0 {
        let y_trunc: f64 = crate::trunc::trunc(y);
        let y_is_odd_integer: bool =
            y == y_trunc && crate::trunc::trunc(y_trunc * 0.5) != y_trunc * 0.5;
        let negative: bool = x.is_sign_negative() && y_is_odd_integer;
        if y > 0.0 {
            return if negative { -0.0 } else { 0.0 };
        }
        // y < 0 here (y == 0 handled above): pole at zero yields infinity.
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // Integer exponent: evaluate by exact binary exponentiation. This reproduces the exact
    // IEEE-754 result for exactly-representable values (e.g. powers of two and integer bases),
    // which the `exp(y * log(x))` approximation cannot guarantee.
    let y_trunc: f64 = crate::trunc::trunc(y);
    let y_is_integer: bool = y == y_trunc;
    if y_is_integer && crate::fabs::fabs(y) < 9_007_199_254_740_992.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut n: u64 = crate::fabs::fabs(y) as u64;
        let mut base: f64 = x;
        let mut acc: f64 = 1.0;
        while n > 0 {
            if n & 1 == 1 {
                acc *= base;
            }
            n >>= 1;
            if n > 0 {
                base *= base;
            }
        }
        return if y < 0.0 { 1.0 / acc } else { acc };
    }

    // Negative base.
    if x < 0.0 {
        // A real result exists only for integer exponents.
        if !y_is_integer {
            return f64::from_bits(0x7FF8_0000_0000_0000); // NaN
        }
        // Integer exponents with magnitude below 2^53 were handled above. Every
        // f64 whose magnitude is >= 2^53 is an exact even integer, so the result
        // is the positive magnitude |x|^y.
        return crate::exp::exp(y * crate::log::log(-x));
    }

    // General case: positive base, fractional exponent.
    crate::exp::exp(y * crate::log::log(x))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_squares() {
        assert!((pow(2.0, 2.0) - 4.0).abs() < 1e-10);
        assert!((pow(3.0, 3.0) - 27.0).abs() < 1e-8);
    }

    #[test]
    fn test_zero_exponent() {
        assert!((pow(5.0, 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_one_exponent() {
        assert!((pow(5.0, 1.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt() {
        assert!((pow(4.0, 0.5) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative_base() {
        assert!((pow(-2.0, 3.0) - (-8.0)).abs() < 1e-8);
        assert!((pow(-2.0, 2.0) - 4.0).abs() < 1e-8);
    }

    #[test]
    fn test_negative_base_non_integer() {
        assert!(pow(-2.0, 1.5).is_nan());
    }

    #[test]
    fn test_nan() {
        assert!(pow(f64::NAN, 2.0).is_nan());
    }

    #[test]
    fn test_nan_special_cases() {
        // pow(x, +-0) == 1 for any x, including NaN.
        assert_eq!(pow(f64::NAN, 0.0), 1.0);
        assert_eq!(pow(f64::NAN, -0.0), 1.0);
        // pow(1, y) == 1 for any y, including NaN.
        assert_eq!(pow(1.0, f64::NAN), 1.0);
    }

    #[test]
    fn test_signed_zero() {
        // Odd integer exponents preserve the sign of zero.
        assert!(pow(-0.0, 3.0).is_sign_negative());
        assert_eq!(pow(-0.0, 3.0), 0.0);
        assert_eq!(pow(-0.0, -3.0), f64::NEG_INFINITY);
        // Non-odd exponents yield positive results.
        assert!(!pow(-0.0, 2.0).is_sign_negative());
        assert_eq!(pow(-0.0, -2.0), f64::INFINITY);
        assert!(!pow(0.0, 3.0).is_sign_negative());
        assert_eq!(pow(0.0, -3.0), f64::INFINITY);
    }

    #[test]
    fn test_negative_base_large_even_integer() {
        // 2^53 + 2 is an exact even integer in f64, so the result must be real
        // (positive), not NaN. |-2|^(2^53+2) overflows to +inf.
        let y: f64 = 9_007_199_254_740_994.0;
        assert_eq!(pow(-2.0, y), f64::INFINITY);
        // A base with magnitude < 1 underflows to +0 (still positive, not NaN).
        let v: f64 = pow(-0.5, y);
        assert_eq!(v, 0.0);
        assert!(v.is_sign_positive());
    }
}
