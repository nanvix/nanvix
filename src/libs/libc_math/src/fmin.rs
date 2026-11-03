// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Returns the minimum of two floating-point values (NaN-aware).
///
/// # Description
///
/// If one argument is NaN, the other is returned. If both are NaN, NaN is returned.
///
/// # Parameters
///
/// - `x`: First value.
/// - `y`: Second value.
///
/// # Returns
///
/// The minimum of `x` and `y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fmin(x: f64, y: f64) -> f64 {
    if x.is_nan() {
        return y;
    }
    if y.is_nan() {
        return x;
    }
    // Handle signed zeros (and any mixed signs): -0.0 compares less than +0.0,
    // so return the negatively-signed operand when the signs differ (C Annex
    // F.10.9.2).
    if x.is_sign_negative() != y.is_sign_negative() {
        return if x.is_sign_negative() { x } else { y };
    }
    if x < y {
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
