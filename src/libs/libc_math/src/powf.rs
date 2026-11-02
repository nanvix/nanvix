// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `x` raised to the power `y` (single-precision).
///
/// # Parameters
///
/// - `x`: Base.
/// - `y`: Exponent.
///
/// # Returns
///
/// The value `x^y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn powf(x: f32, y: f32) -> f32 {
    // Evaluate in double precision (reusing the exact integer-exponent path in `pow`) and round
    // once back to single precision. This is at least as accurate as a native single-precision
    // `expf(y * logf(x))` and yields exact results for integer exponents.
    #[allow(clippy::cast_possible_truncation)]
    let result: f32 = crate::pow::pow(f64::from(x), f64::from(y)) as f32;
    result
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
