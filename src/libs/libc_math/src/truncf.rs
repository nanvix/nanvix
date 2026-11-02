// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Truncates a single-precision floating-point number toward zero.
///
/// # Description
///
/// Returns the nearest integer not greater in magnitude than `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The truncated value of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn truncf(x: f32) -> f32 {
    let bits: u32 = x.to_bits();
    let exp_raw: u32 = (bits >> 23) & 0xFF;

    if exp_raw >= 127 + 23 {
        return x;
    }
    if exp_raw < 127 {
        return f32::from_bits(bits & 0x8000_0000);
    }

    let shift: u32 = exp_raw - 127;
    let mask: u32 = 0x007F_FFFF >> shift;
    f32::from_bits(bits & !mask)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
