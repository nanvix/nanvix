// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

#[cfg(test)]
const PI: f64 = core::f64::consts::PI;
const FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;
const FRAC_2_PI: f64 = core::f64::consts::FRAC_2_PI;

const SIN_COEFFS: [f64; 7] = [
    1.605_904_383_682_161_5e-10,
    -2.505_210_838_544_172e-8,
    2.755_731_922_398_589_3e-6,
    -1.984_126_984_126_984e-4,
    8.333_333_333_333_332e-3,
    -1.666_666_666_666_666_4e-1,
    1.0,
];

const COS_COEFFS: [f64; 7] = [
    2.087_675_698_786_81e-9,
    -2.755_731_922_398_589_3e-7,
    2.480_158_730_158_73e-5,
    -1.388_888_888_888_889e-3,
    4.166_666_666_666_666e-2,
    -0.5,
    1.0,
];

//==================================================================================================
// Internal Helpers
//==================================================================================================

fn sin_kernel(x: f64) -> f64 {
    let x2: f64 = x * x;
    let mut p: f64 = SIN_COEFFS[0];
    let mut i: usize = 1;
    while i < SIN_COEFFS.len() {
        p = p * x2 + SIN_COEFFS[i];
        i += 1;
    }
    x * p
}

fn cos_kernel(x: f64) -> f64 {
    let x2: f64 = x * x;
    let mut p: f64 = COS_COEFFS[0];
    let mut i: usize = 1;
    while i < COS_COEFFS.len() {
        p = p * x2 + COS_COEFFS[i];
        i += 1;
    }
    p
}

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
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((cos(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pi_over_2() {
        assert!((cos(FRAC_PI_2)).abs() < 1e-10);
    }

    #[test]
    fn test_pi() {
        assert!((cos(PI) - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pi_over_3() {
        assert!((cos(PI / 3.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!(cos(f64::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert!(cos(f64::INFINITY).is_nan());
    }
}
