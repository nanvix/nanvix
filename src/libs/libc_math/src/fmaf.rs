// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `x * y + z` as a single ternary operation in single precision.
///
/// # Description
///
/// Performs a fused multiply-add: the product `x * y` and the sum with `z` are computed with a
/// single rounding step, yielding a correctly-rounded `f32` result without the intermediate
/// rounding (and possible cancellation) of a separate multiply and add.
///
/// # Parameters
///
/// - `x`: First factor.
/// - `y`: Second factor.
/// - `z`: Addend.
///
/// # Returns
///
/// The value of `x * y + z`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fmaf(x: f32, y: f32, z: f32) -> f32 {
    core::intrinsics::fmaf32(x, y, z)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert_eq!(fmaf(2.0, 3.0, 4.0), 10.0);
        assert_eq!(fmaf(3.0, 4.0, 5.0), 17.0);
    }

    #[test]
    fn test_zero_factor() {
        assert_eq!(fmaf(0.0, 5.0, 7.0), 7.0);
    }

    #[test]
    fn test_nan() {
        assert!(fmaf(f32::NAN, 1.0, 1.0).is_nan());
    }
}
