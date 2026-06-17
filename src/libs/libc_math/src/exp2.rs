// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LN2: f64 = core::f64::consts::LN_2;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `2` raised to the power `x`.
///
/// # Description
///
/// Decomposes `x` into an integer part `k` and a fractional part `f` with `|f| <= 0.5`, computing
/// `2^f` via `exp(f * ln 2)` and scaling by `2^k`. Integer inputs therefore yield exact results.
///
/// # Parameters
///
/// - `x`: Exponent.
///
/// # Returns
///
/// The value of `2^x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn exp2(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x >= 1024.0 {
        return f64::INFINITY;
    }
    if x <= -1075.0 {
        return 0.0;
    }

    let k: f64 = crate::round::round(x);
    let f: f64 = x - k;
    let two_f: f64 = if f == 0.0 {
        1.0
    } else {
        crate::exp::exp(f * LN2)
    };

    #[allow(clippy::cast_possible_truncation)]
    let ki: i32 = k as i32;
    crate::ldexp::ldexp(two_f, ki)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_integer_powers() {
        assert_eq!(exp2(0.0), 1.0);
        assert_eq!(exp2(3.0), 8.0);
        assert_eq!(exp2(10.0), 1024.0);
        assert_eq!(exp2(-1.0), 0.5);
    }

    #[test]
    fn test_half() {
        assert!((exp2(0.5) - core::f64::consts::SQRT_2).abs() < 1e-10);
    }

    #[test]
    fn test_overflow() {
        assert_eq!(exp2(1024.0), f64::INFINITY);
    }

    #[test]
    fn test_underflow() {
        assert_eq!(exp2(-1075.0), 0.0);
    }

    #[test]
    fn test_nan() {
        assert!(exp2(f64::NAN).is_nan());
    }
}
