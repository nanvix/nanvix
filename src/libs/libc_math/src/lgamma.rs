// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Lanczos approximation parameter `g`.
const G: f64 = 7.0;

/// Lanczos coefficients (g = 7, n = 9).
const C: [f64; 9] = [
    0.999_999_999_999_809_9,
    676.520_368_121_885_1,
    -1_259.139_216_722_402_8,
    771.323_428_777_653_1,
    -176.615_029_162_140_6,
    12.507_343_278_686_905,
    -0.138_571_095_265_720_12,
    9.984_369_578_019_572e-6,
    1.505_632_735_149_311_6e-7,
];

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Computes the natural logarithm of the absolute value of the gamma function of `x`.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `ln(|Γ(x)|)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn lgamma(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x < 0.5 {
        // Reflection: ln Γ(x) = ln(π / |sin(πx)|) - ln Γ(1 - x).
        let s: f64 = crate::sin::sin(core::f64::consts::PI * x).abs();
        if s == 0.0 {
            return f64::INFINITY;
        }
        crate::log::log(core::f64::consts::PI / s) - lgamma(1.0 - x)
    } else {
        let y: f64 = x - 1.0;
        let mut a: f64 = C[0];
        let mut denom: f64 = y + 1.0;
        let mut i: usize = 1;
        while i < 9 {
            a += C[i] / denom;
            denom += 1.0;
            i += 1;
        }
        let t: f64 = y + G + 0.5;
        const HALF_LN_2PI: f64 = 0.918_938_533_204_672_8;
        HALF_LN_2PI + (y + 0.5) * crate::log::log(t) - t + crate::log::log(a)
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_integer_factorials() {
        // lgamma(n) == ln((n - 1)!).
        assert!((lgamma(1.0)).abs() < 1e-9);
        assert!((lgamma(2.0)).abs() < 1e-9);
        assert!((lgamma(3.0) - core::f64::consts::LN_2).abs() < 1e-9);
        assert!((lgamma(5.0) - 3.178_053_830_347_945).abs() < 1e-9);
    }

    #[test]
    fn test_half() {
        // lgamma(0.5) == ln(sqrt(pi)).
        assert!((lgamma(0.5) - 0.572_364_942_924_700_4).abs() < 1e-9);
    }

    #[test]
    fn test_reflection() {
        // lgamma is finite for negative non-integer arguments.
        assert!(lgamma(-0.5).is_finite());
    }

    #[test]
    fn test_nan() {
        assert!(lgamma(f64::NAN).is_nan());
    }
}
