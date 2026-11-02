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
mod tests;
