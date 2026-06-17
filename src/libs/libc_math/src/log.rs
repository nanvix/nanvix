// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LN2: f64 = core::f64::consts::LN_2;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the natural logarithm of `x`.
///
/// # Description
///
/// Decomposes `x` into mantissa `m` and exponent `e`, then computes
/// `log(x) = e * ln(2) + log(m)` using the substitution `s = (m-1)/(m+1)` for fast convergence.
///
/// # Parameters
///
/// - `x`: Input value (must be positive).
///
/// # Returns
///
/// The natural logarithm of `x`. Returns NaN for negative inputs, -inf for zero.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn log(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x < 0.0 {
        return f64::from_bits(0x7FF8_0000_0000_0000);
    }
    if x == 0.0 {
        return f64::from_bits(0xFFF0_0000_0000_0000);
    }
    if x.to_bits() == 0x7FF0_0000_0000_0000 {
        return x;
    }

    let bits: u64 = x.to_bits();
    let exp_raw: u64 = (bits >> 52) & 0x7FF;

    // Handle subnormals by scaling up.
    let (adj_bits, exp_adj): (u64, i64) = if exp_raw == 0 {
        let scaled: f64 = x * 4_503_599_627_370_496.0; // 2^52
        (scaled.to_bits(), -52)
    } else {
        (bits, 0)
    };

    let exp_biased: u64 = (adj_bits >> 52) & 0x7FF;
    #[allow(clippy::cast_possible_wrap)]
    let mut e: i64 = (exp_biased as i64) - 1023 + exp_adj;

    // Extract mantissa in [1, 2).
    let m_bits: u64 = (adj_bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000;
    let mut m: f64 = f64::from_bits(m_bits);

    // Reduce m to [sqrt(2)/2, sqrt(2)] for better convergence.
    if m > core::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }

    // Use substitution s = (m-1)/(m+1), then log(m) = 2*(s + s^3/3 + s^5/5 + ...).
    let s: f64 = (m - 1.0) / (m + 1.0);
    let s2: f64 = s * s;

    let log_m: f64 = 2.0
        * s
        * (1.0
            + s2 * (1.0 / 3.0
                + s2 * (0.2
                    + s2 * (1.0 / 7.0
                        + s2 * (1.0 / 9.0
                            + s2 * (1.0 / 11.0 + s2 * (1.0 / 13.0 + s2 * (1.0 / 15.0))))))));

    let e_f64: f64 = e as f64;
    e_f64 * LN2 + log_m
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!((log(1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_e() {
        assert!((log(2.718_281_828_459_045) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_two() {
        assert!((log(2.0) - LN2).abs() < 1e-10);
    }

    #[test]
    fn test_ten() {
        assert!((log(10.0) - 2.302_585_092_994_046).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(log(0.0), f64::NEG_INFINITY);
    }

    #[test]
    fn test_negative() {
        assert!(log(-1.0).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(log(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn test_nan() {
        assert!(log(f64::NAN).is_nan());
    }
}
