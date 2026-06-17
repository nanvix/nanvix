// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

const LN2: f32 = core::f32::consts::LN_2;
const LN2_INV: f32 = core::f32::consts::LOG2_E;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `e^x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `e^x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn expf(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }

    let k_f: f32 = crate::roundf::roundf(x * LN2_INV);
    let r: f32 = x - k_f * LN2;

    let poly: f32 = 1.0
        + r * (1.0
            + r * (0.5
                + r * (1.0 / 6.0 + r * (1.0 / 24.0 + r * (1.0 / 120.0 + r * (1.0 / 720.0))))));

    #[allow(clippy::cast_possible_truncation)]
    let k_i64: i64 = k_f as i64;

    // The result is `poly * 2^k`. Inputs just below the overflow boundary
    // (`ln(f32::MAX) ~= 88.7228`) reduce to `k == 128`, whose result is still
    // finite even though `2^128` is not a representable exponent. Scale as
    // `2^127 * 2` so those values round correctly while true overflows saturate
    // to `+inf`.
    if k_i64 > 128 {
        return f32::from_bits(0x7F80_0000);
    }
    if k_i64 == 128 {
        return poly * f32::from_bits(0x7F00_0000) * 2.0;
    }
    if k_i64 < -126 {
        let scale: f32 = f32::from_bits(0x0080_0000);
        let adj: i64 = k_i64 + 126;
        if adj < -126 {
            return 0.0;
        }
        let biased: u64 = (adj + 127) as u64;
        #[allow(clippy::cast_possible_truncation)]
        let biased_u32: u32 = biased as u32;
        return poly * scale * f32::from_bits(biased_u32 << 23);
    }

    let biased: u64 = (k_i64 + 127) as u64;
    #[allow(clippy::cast_possible_truncation)]
    let biased_u32: u32 = biased as u32;
    poly * f32::from_bits(biased_u32 << 23)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((expf(0.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_one() {
        assert!((expf(1.0) - 2.718_28).abs() < 1e-3);
    }

    #[test]
    fn test_overflow() {
        assert_eq!(expf(89.0), f32::INFINITY);
    }

    #[test]
    fn test_nan() {
        assert!(expf(f32::NAN).is_nan());
    }

    #[test]
    fn test_overflow_boundary_is_finite() {
        // Inputs below `ln(f32::MAX)` (~= 88.7228) must not overflow prematurely.
        let v: f32 = expf(88.5);
        let expected: f32 = 88.5f32.exp();
        assert!(v.is_finite() && v > 0.0);
        assert!((v - expected).abs() <= expected * 1e-4);
    }

    #[test]
    fn test_underflow_boundary_is_subnormal() {
        // Inputs above `ln(2^-150)` (~= -103.972) must not underflow to zero.
        let v: f32 = expf(-103.5);
        assert!(v > 0.0);
        assert!(v < f32::MIN_POSITIVE);
    }
}
