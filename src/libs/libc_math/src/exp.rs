// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LN2: f64 = core::f64::consts::LN_2;
const LN2_INV: f64 = core::f64::consts::LOG2_E;

/// Taylor series coefficients for e^r: 1/n! from n=12 down to n=0.
const EXP_COEFFS: [f64; 13] = [
    1.0 / 479_001_600.0,
    1.0 / 39_916_800.0,
    1.0 / 3_628_800.0,
    1.0 / 362_880.0,
    1.0 / 40_320.0,
    1.0 / 5_040.0,
    1.0 / 720.0,
    1.0 / 120.0,
    1.0 / 24.0,
    1.0 / 6.0,
    0.5,
    1.0,
    1.0,
];

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `e^x`.
///
/// # Description
///
/// Uses range reduction: `e^x = 2^k * e^r` where `r = x - k*ln(2)` and `|r| <= ln(2)/2`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `e^x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }

    // Range reduction: x = k*ln(2) + r.
    let k_f: f64 = crate::round::round(x * LN2_INV);
    let r: f64 = x - k_f * LN2;

    // Horner evaluation of degree-12 Taylor polynomial.
    let mut poly: f64 = EXP_COEFFS[0];
    let mut i: usize = 1;
    while i < EXP_COEFFS.len() {
        poly = poly * r + EXP_COEFFS[i];
        i += 1;
    }

    // Multiply by 2^k.
    #[allow(clippy::cast_possible_truncation)]
    let k_i64: i64 = k_f as i64;

    // The result is `poly * 2^k`. Inputs just below the overflow boundary
    // (`ln(f64::MAX) ~= 709.7827`) reduce to `k == 1024`, whose result is still
    // finite even though `2^1024` is not a representable exponent. Scale as
    // `2^1023 * 2` so those values round correctly while true overflows saturate
    // to `+inf`.
    if k_i64 > 1024 {
        return f64::from_bits(0x7FF0_0000_0000_0000);
    }
    if k_i64 == 1024 {
        return poly * f64::from_bits(0x7FE0_0000_0000_0000) * 2.0;
    }
    if k_i64 < -1022 {
        let scale: f64 = f64::from_bits(0x0010_0000_0000_0000);
        let adj: i64 = k_i64 + 1022;
        if adj < -1022 {
            return 0.0;
        }
        let biased: u64 = (adj + 1023) as u64;
        return poly * scale * f64::from_bits(biased << 52);
    }

    let biased: u64 = (k_i64 + 1023) as u64;
    poly * f64::from_bits(biased << 52)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((exp(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((exp(1.0) - 2.718_281_828_459_045).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((exp(-1.0) - 0.367_879_441_171_442_3).abs() < 1e-10);
    }

    #[test]
    fn test_ln2() {
        assert!((exp(LN2) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_overflow() {
        assert_eq!(exp(710.0), f64::INFINITY);
    }

    #[test]
    fn test_underflow() {
        assert_eq!(exp(-746.0), 0.0);
    }

    #[test]
    fn test_nan() {
        assert!(exp(f64::NAN).is_nan());
    }

    #[test]
    fn test_overflow_boundary_is_finite() {
        // Inputs below `ln(f64::MAX)` (~= 709.7827) must not overflow prematurely.
        let v: f64 = exp(709.5);
        let expected: f64 = 709.5f64.exp();
        assert!(v.is_finite() && v > 0.0);
        assert!((v - expected).abs() <= expected * 1e-10);
    }

    #[test]
    fn test_underflow_boundary_is_subnormal() {
        // Inputs above `ln(2^-1075)` (~= -745.133) must not underflow to zero.
        let v: f64 = exp(-745.1);
        assert!(v > 0.0);
        assert!(v < f64::MIN_POSITIVE);
    }
}
