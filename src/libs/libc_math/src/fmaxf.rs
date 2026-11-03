// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Returns the maximum of two single-precision values (NaN-aware).
///
/// # Parameters
///
/// - `x`: First value.
/// - `y`: Second value.
///
/// # Returns
///
/// The maximum of `x` and `y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fmaxf(x: f32, y: f32) -> f32 {
    if x.is_nan() {
        return y;
    }
    if y.is_nan() {
        return x;
    }
    // Handle signed zeros (and any mixed signs): +0.0 compares greater than
    // -0.0, so return the positively-signed operand when the signs differ (C
    // Annex F.10.9.2).
    if x.is_sign_negative() != y.is_sign_negative() {
        return if x.is_sign_negative() { y } else { x };
    }
    if x > y {
        x
    } else {
        y
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
