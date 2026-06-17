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

/// Computes the gamma function of `x` using the Lanczos approximation.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `tgamma(x)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tgamma(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x < 0.5 {
        // The gamma function has poles at zero and the negative integers.
        //
        // At zero the one-sided limits are signed infinities: Γ(+0) = +∞ and Γ(-0) = -∞, so the sign
        // of the zero is preserved. At a negative integer the two-sided limit does not exist (the
        // function diverges to +∞ on one side and -∞ on the other); this is a domain error and yields
        // NaN per the C standard (Annex F).
        if x == 0.0 {
            return if x.is_sign_negative() {
                f64::from_bits(0xFFF0_0000_0000_0000) // -inf
            } else {
                f64::from_bits(0x7FF0_0000_0000_0000) // +inf
            };
        }
        // Reflection formula: Γ(x) = π / (sin(πx) · Γ(1 - x)).
        let s: f64 = crate::sin::sin(core::f64::consts::PI * x);
        if s == 0.0 {
            // Negative-integer pole: domain error.
            return f64::NAN;
        }
        core::f64::consts::PI / (s * tgamma(1.0 - x))
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
        const SQRT_2PI: f64 = 2.506_628_274_631_000_5;
        SQRT_2PI * crate::pow::pow(t, y + 0.5) * crate::exp::exp(-t) * a
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
        assert!((tgamma(1.0) - 1.0).abs() < 1e-9);
        assert!((tgamma(2.0) - 1.0).abs() < 1e-9);
        assert!((tgamma(3.0) - 2.0).abs() < 1e-9);
        assert!((tgamma(4.0) - 6.0).abs() < 1e-9);
        assert!((tgamma(5.0) - 24.0).abs() < 1e-8);
    }

    #[test]
    fn test_half() {
        // tgamma(0.5) == sqrt(pi).
        assert!((tgamma(0.5) - 1.772_453_850_905_515_9).abs() < 1e-9);
    }

    #[test]
    fn test_nan() {
        assert!(tgamma(f64::NAN).is_nan());
    }

    #[test]
    fn test_poles() {
        // Γ(+0) = +∞ and Γ(-0) = -∞: the sign of the zero is preserved.
        assert!(tgamma(0.0).is_infinite() && tgamma(0.0).is_sign_positive());
        assert!(tgamma(-0.0).is_infinite() && tgamma(-0.0).is_sign_negative());

        // Negative integers are domain errors and yield NaN.
        assert!(tgamma(-1.0).is_nan());
        assert!(tgamma(-2.0).is_nan());
        assert!(tgamma(-3.0).is_nan());
    }
}
