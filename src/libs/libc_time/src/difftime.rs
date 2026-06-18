// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::time_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the difference between two calendar times as a floating-point number of seconds.
///
/// # Parameters
///
/// - `time1`: The later time value.
/// - `time0`: The earlier time value.
///
/// # Returns
///
/// The number of seconds elapsed from `time0` to `time1`, as a `f64`.
///
/// # Safety
///
/// This function is safe for all input values.
///
#[allow(clippy::cast_precision_loss)]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn difftime(time1: time_t, time0: time_t) -> f64 {
    // Compute the difference in floating point to avoid overflowing `time_t` for extreme inputs.
    (time1 as f64) - (time0 as f64)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::difftime;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_difftime_positive() {
        let result: f64 = difftime(100 as time_t, 50 as time_t);
        assert!((result - 50.0).abs() < f64::EPSILON, "Expected 50.0, got {result}");
    }

    #[test]
    fn test_difftime_negative() {
        let result: f64 = difftime(50 as time_t, 100 as time_t);
        assert!((result - (-50.0)).abs() < f64::EPSILON, "Expected -50.0, got {result}");
    }

    #[test]
    fn test_difftime_zero() {
        let result: f64 = difftime(100 as time_t, 100 as time_t);
        assert!((result - 0.0).abs() < f64::EPSILON, "Expected 0.0, got {result}");
    }

    #[test]
    fn test_difftime_large_values() {
        let t1: time_t = 1_000_000_000;
        let t0: time_t = 0;
        let result: f64 = difftime(t1, t0);
        assert!((result - 1_000_000_000.0).abs() < f64::EPSILON, "Expected 1e9, got {result}");
    }
}
