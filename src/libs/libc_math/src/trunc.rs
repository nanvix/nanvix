// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Truncates a floating-point number toward zero.
///
/// # Description
///
/// Returns the nearest integer not greater in magnitude than `x`, effectively rounding toward zero.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The truncated value of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn trunc(x: f64) -> f64 {
    let bits: u64 = x.to_bits();
    let exp_raw: u64 = (bits >> 52) & 0x7FF;

    // If exponent indicates value is already integer (or NaN/inf), return as-is.
    if exp_raw >= 1023 + 52 {
        return x;
    }

    // If exponent indicates |x| < 1, return ±0.0.
    if exp_raw < 1023 {
        return f64::from_bits(bits & 0x8000_0000_0000_0000);
    }

    // Mask out fractional mantissa bits.
    let shift: u64 = exp_raw - 1023;
    let mask: u64 = 0x000F_FFFF_FFFF_FFFF >> shift;
    f64::from_bits(bits & !mask)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive_fraction() {
        assert!((trunc(3.7) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative_fraction() {
        assert!((trunc(-3.7) - (-3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_integer() {
        assert!((trunc(5.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_small() {
        assert_eq!(trunc(0.9).to_bits(), 0.0_f64.to_bits());
        assert_eq!(trunc(-0.9).to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn test_zero() {
        assert_eq!(trunc(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(trunc(-0.0).to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn test_nan() {
        assert!(trunc(f64::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(trunc(f64::INFINITY), f64::INFINITY);
        assert_eq!(trunc(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }
}
