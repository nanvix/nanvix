// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Multiplies a floating-point number by a power of two.
///
/// # Description
///
/// Equivalent to [`ldexp`](crate::ldexp::ldexp). Computes `x * 2^n`.
///
/// # Parameters
///
/// - `x`: Base value.
/// - `n`: Exponent.
///
/// # Returns
///
/// The value `x * 2^n`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn scalbn(x: f64, n: c_int) -> f64 {
    crate::ldexp::ldexp(x, n)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert!((scalbn(1.0, 4) - 16.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((scalbn(16.0, -4) - 1.0).abs() < 1e-10);
    }
}
