// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LN2: f32 = core::f32::consts::LN_2;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the natural logarithm of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value (must be positive).
///
/// # Returns
///
/// The natural logarithm of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn logf(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x < 0.0 {
        return f32::from_bits(0x7FC0_0000);
    }
    if x == 0.0 {
        return f32::from_bits(0xFF80_0000);
    }
    if x.to_bits() == 0x7F80_0000 {
        return x;
    }

    let bits: u32 = x.to_bits();
    let exp_raw: u32 = (bits >> 23) & 0xFF;

    let (adj_bits, exp_adj): (u32, i64) = if exp_raw == 0 {
        let scaled: f32 = x * 8_388_608.0;
        (scaled.to_bits(), -23)
    } else {
        (bits, 0)
    };

    let exp_biased: u32 = (adj_bits >> 23) & 0xFF;
    let mut e: i64 = i64::from(exp_biased) - 127 + exp_adj;

    let m_bits: u32 = (adj_bits & 0x007F_FFFF) | 0x3F80_0000;
    let mut m: f32 = f32::from_bits(m_bits);

    if m > core::f32::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }

    let s: f32 = (m - 1.0) / (m + 1.0);
    let s2: f32 = s * s;

    let log_m: f32 =
        2.0 * s * (1.0 + s2 * (1.0 / 3.0 + s2 * (0.2 + s2 * (1.0 / 7.0 + s2 * (1.0 / 9.0)))));

    let e_f32: f32 = e as f32;
    e_f32 * LN2 + log_m
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!((logf(1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_e() {
        assert!((logf(2.718_282) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_zero() {
        assert_eq!(logf(0.0), f32::NEG_INFINITY);
    }

    #[test]
    fn test_negative() {
        assert!(logf(-1.0).is_nan());
    }
}
