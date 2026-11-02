// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the absolute value of a single-precision floating-point number.
///
/// # Description
///
/// Returns the absolute value of `x` by clearing the sign bit.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The absolute value of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fabsf(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7FFF_FFFF)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
