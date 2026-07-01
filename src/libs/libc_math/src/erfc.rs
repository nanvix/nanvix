// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::erf::{
    ERX,
    PA0,
    PA1,
    PA2,
    PA3,
    PA4,
    PA5,
    PA6,
    PP0,
    PP1,
    PP2,
    PP3,
    PP4,
    QA1,
    QA2,
    QA3,
    QA4,
    QA5,
    QA6,
    QQ1,
    QQ2,
    QQ3,
    QQ4,
    QQ5,
    RA0,
    RA1,
    RA2,
    RA3,
    RA4,
    RA5,
    RA6,
    RA7,
    RB0,
    RB1,
    RB2,
    RB3,
    RB4,
    RB5,
    RB6,
    SA1,
    SA2,
    SA3,
    SA4,
    SA5,
    SA6,
    SA7,
    SA8,
    SB1,
    SB2,
    SB3,
    SB4,
    SB5,
    SB6,
    SB7,
    TINY_ERF,
};

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the complementary error function of `x`, `erfc(x) = 1 - erf(x)`.
///
/// # Description
///
/// Uses the fdlibm piecewise rational approximations. For larger arguments it
/// evaluates the tail directly from a scaled exponential rather than as
/// `1 - erf(x)`, avoiding the catastrophic cancellation that destroys precision in
/// the tail. Accurate to within one unit in the last place.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `erfc(x)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub extern "C" fn erfc(x: f64) -> f64 {
    let hx: i32 = (x.to_bits() >> 32) as i32;
    let ix: i32 = hx & 0x7fff_ffff;
    if ix >= 0x7ff0_0000 {
        // erfc(NaN) = NaN, erfc(+inf) = 0, erfc(-inf) = 2.
        let i: i32 = ((x.to_bits() >> 63) as i32) << 1;
        return f64::from(i) + 1.0 / x;
    }

    if ix < 0x3feb_0000 {
        // |x| < 0.84375
        if ix < 0x3c70_0000 {
            // |x| < 2^-56
            return 1.0 - x;
        }
        let z: f64 = x * x;
        let r: f64 = PP0 + z * (PP1 + z * (PP2 + z * (PP3 + z * PP4)));
        let s: f64 = 1.0 + z * (QQ1 + z * (QQ2 + z * (QQ3 + z * (QQ4 + z * QQ5))));
        let y: f64 = r / s;
        if hx < 0x3fd0_0000 {
            // x < 1/4
            return 1.0 - (x + x * y);
        }
        let r2: f64 = x * y + (x - 0.5);
        return 0.5 - r2;
    }
    if ix < 0x3ff4_0000 {
        // 0.84375 <= |x| < 1.25
        let s: f64 = f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff) - 1.0;
        let p: f64 = PA0 + s * (PA1 + s * (PA2 + s * (PA3 + s * (PA4 + s * (PA5 + s * PA6)))));
        let q: f64 = 1.0 + s * (QA1 + s * (QA2 + s * (QA3 + s * (QA4 + s * (QA5 + s * QA6)))));
        if hx >= 0 {
            let z: f64 = 1.0 - ERX;
            return z - p / q;
        }
        let z: f64 = ERX + p / q;
        return 1.0 + z;
    }
    if ix < 0x403c_0000 {
        // |x| < 28
        let ax: f64 = f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff);
        let s: f64 = 1.0 / (ax * ax);
        let (r, big_s): (f64, f64) = if ix < 0x4006_db6d {
            // |x| < 1/0.35 ~= 2.857
            let r: f64 = RA0
                + s * (RA1 + s * (RA2 + s * (RA3 + s * (RA4 + s * (RA5 + s * (RA6 + s * RA7))))));
            let big_s: f64 = 1.0
                + s * (SA1
                    + s * (SA2
                        + s * (SA3 + s * (SA4 + s * (SA5 + s * (SA6 + s * (SA7 + s * SA8)))))));
            (r, big_s)
        } else {
            // |x| >= 1/0.35 ~= 2.857
            if hx < 0 && ix >= 0x4018_0000 {
                return 2.0 - TINY_ERF; // x < -6
            }
            let r: f64 = RB0 + s * (RB1 + s * (RB2 + s * (RB3 + s * (RB4 + s * (RB5 + s * RB6)))));
            let big_s: f64 = 1.0
                + s * (SB1 + s * (SB2 + s * (SB3 + s * (SB4 + s * (SB5 + s * (SB6 + s * SB7))))));
            (r, big_s)
        };
        let z: f64 = f64::from_bits(ax.to_bits() & 0xffff_ffff_0000_0000); // high word only
        let r2: f64 =
            crate::exp::exp(-z * z - 0.5625) * crate::exp::exp((z - ax) * (z + ax) + r / big_s);
        if hx > 0 {
            r2 / ax
        } else {
            2.0 - r2 / ax
        }
    } else if hx > 0 {
        TINY_ERF * TINY_ERF
    } else {
        2.0 - TINY_ERF
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    /// Reference values from a correctly-rounded erfc (e.g. glibc / MPFR). The
    /// large-argument cases exercise the tail path where `1 - erf(x)` would cancel.
    const CASES: [(f64, f64); 6] = [
        (0.5, 0.479_500_122_186_953_5),
        (1.0, 0.157_299_207_050_285_13),
        (2.0, 4.677_734_981_047_266e-3),
        (3.0, 2.209_049_699_858_544e-5),
        (5.0, 1.537_459_794_428_035e-12),
        (10.0, 2.088_487_583_762_544_7e-45),
    ];

    #[test]
    fn test_reference_values() {
        for &(x, want) in &CASES {
            let got: f64 = erfc(x);
            assert!((got - want).abs() <= want.abs() * 1e-13, "erfc({x}) = {got}, want {want}");
        }
    }

    #[test]
    fn test_erfc_plus_erf_is_one() {
        for &x in &[-1.5_f64, -0.3, 0.0, 0.3, 0.9, 1.5, 2.5] {
            assert!((erfc(x) + crate::erf::erf(x) - 1.0).abs() <= 1e-14);
        }
    }

    #[test]
    fn test_zero() {
        assert_eq!(erfc(0.0), 1.0);
    }

    #[test]
    fn test_limits() {
        assert_eq!(erfc(f64::INFINITY), 0.0);
        assert_eq!(erfc(f64::NEG_INFINITY), 2.0);
        assert_eq!(erfc(30.0), 0.0); // underflows to zero
    }

    #[test]
    fn test_nan() {
        assert!(erfc(f64::NAN).is_nan());
    }
}
