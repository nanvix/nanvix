// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sin::{
    cos_kernel,
    sin_kernel,
};

//==================================================================================================
// Constants
//==================================================================================================

const FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;
const FRAC_2_PI: f64 = core::f64::consts::FRAC_2_PI;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the cosine of `x` (in radians).
///
/// # Description
///
/// Uses quadrant-based range reduction followed by polynomial approximation.
///
/// # Parameters
///
/// - `x`: Input angle in radians.
///
/// # Returns
///
/// The cosine of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn cos(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.to_bits() & 0x7FF0_0000_0000_0000 == 0x7FF0_0000_0000_0000 {
        return f64::from_bits(0x7FF8_0000_0000_0000);
    }

    let k_f: f64 = crate::round::round(x * FRAC_2_PI);
    #[allow(clippy::cast_possible_truncation)]
    let k: i64 = k_f as i64;
    let r: f64 = x - k_f * FRAC_PI_2;
    let quadrant: u64 = (k as u64) & 3;

    match quadrant {
        0 => cos_kernel(r),
        1 => -sin_kernel(r),
        2 => -cos_kernel(r),
        3 => sin_kernel(r),
        _ => cos_kernel(r),
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
