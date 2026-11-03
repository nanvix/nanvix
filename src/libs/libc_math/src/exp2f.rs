// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `2` raised to the power `x` in single precision.
///
/// # Parameters
///
/// - `x`: Exponent.
///
/// # Returns
///
/// The value of `2^x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn exp2f(x: f32) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let result: f32 = crate::exp2::exp2(f64::from(x)) as f32;
    result
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
