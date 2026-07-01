// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// A tiny value used to raise the inexact flag and produce correctly-signed
/// results in the saturated tails.
pub(crate) const TINY_ERF: f64 = 1e-300;

/// `erf(1)` rounded to 53 bits, the anchor for the `[0.84375, 1.25]` branch.
pub(crate) const ERX: f64 = 8.450_629_115_104_675e-1;

// Coefficients for the approximation of `erf` on `[0, 0.84375]`.
pub(crate) const EFX8: f64 = 1.027_033_336_764_100_7;
pub(crate) const PP0: f64 = 1.283_791_670_955_125_6e-1;
pub(crate) const PP1: f64 = -3.250_421_072_470_015e-1;
pub(crate) const PP2: f64 = -2.848_174_957_559_851e-2;
pub(crate) const PP3: f64 = -5.770_270_296_489_442e-3;
pub(crate) const PP4: f64 = -2.376_301_665_665_016_3e-5;
pub(crate) const QQ1: f64 = 3.979_172_239_591_553_5e-1;
pub(crate) const QQ2: f64 = 6.502_224_998_876_73e-2;
pub(crate) const QQ3: f64 = 5.081_306_281_875_766e-3;
pub(crate) const QQ4: f64 = 1.324_947_380_043_216_4e-4;
pub(crate) const QQ5: f64 = -3.960_228_278_775_368e-6;

// Coefficients for the approximation of `erf` on `[0.84375, 1.25]`.
pub(crate) const PA0: f64 = -2.362_118_560_752_659_4e-3;
pub(crate) const PA1: f64 = 4.148_561_186_837_483_3e-1;
pub(crate) const PA2: f64 = -3.722_078_760_357_013e-1;
pub(crate) const PA3: f64 = 3.183_466_199_011_617_5e-1;
pub(crate) const PA4: f64 = -1.108_946_942_823_966_8e-1;
pub(crate) const PA5: f64 = 3.547_830_432_561_823_6e-2;
pub(crate) const PA6: f64 = -2.166_375_594_868_791e-3;
pub(crate) const QA1: f64 = 1.064_208_804_008_442_3e-1;
pub(crate) const QA2: f64 = 5.403_979_177_021_71e-1;
pub(crate) const QA3: f64 = 7.182_865_441_419_627e-2;
pub(crate) const QA4: f64 = 1.261_712_198_087_616_4e-1;
pub(crate) const QA5: f64 = 1.363_708_391_202_905e-2;
pub(crate) const QA6: f64 = 1.198_449_984_679_910_7e-2;

// Coefficients for the approximation of `erfc` on `[1.25, 1/0.35]`.
pub(crate) const RA0: f64 = -9.864_944_034_847_148e-3;
pub(crate) const RA1: f64 = -6.938_585_727_071_818e-1;
pub(crate) const RA2: f64 = -1.055_862_622_532_329_1e1;
pub(crate) const RA3: f64 = -6.237_533_245_032_600_6e1;
pub(crate) const RA4: f64 = -1.623_966_694_625_734_7e2;
pub(crate) const RA5: f64 = -1.846_050_929_067_110_4e2;
pub(crate) const RA6: f64 = -8.128_743_550_630_66e1;
pub(crate) const RA7: f64 = -9.814_329_344_169_145;
pub(crate) const SA1: f64 = 1.965_127_166_743_925_7e1;
pub(crate) const SA2: f64 = 1.376_577_541_435_190_4e2;
pub(crate) const SA3: f64 = 4.345_658_774_752_292_3e2;
pub(crate) const SA4: f64 = 6.453_872_717_332_679e2;
pub(crate) const SA5: f64 = 4.290_081_400_275_678_3e2;
pub(crate) const SA6: f64 = 1.086_350_055_417_794_4e2;
pub(crate) const SA7: f64 = 6.570_249_770_319_282;
pub(crate) const SA8: f64 = -6.042_441_521_485_81e-2;

// Coefficients for the approximation of `erfc` on `[1/0.35, 28]`.
pub(crate) const RB0: f64 = -9.864_942_924_700_1e-3;
pub(crate) const RB1: f64 = -7.992_832_376_805_23e-1;
pub(crate) const RB2: f64 = -1.775_795_491_775_475_2e1;
pub(crate) const RB3: f64 = -1.606_363_848_558_219_2e2;
pub(crate) const RB4: f64 = -6.375_664_433_683_896e2;
pub(crate) const RB5: f64 = -1.025_095_131_611_077_2e3;
pub(crate) const RB6: f64 = -4.835_191_916_086_514e2;
pub(crate) const SB1: f64 = 3.033_806_074_348_246e1;
pub(crate) const SB2: f64 = 3.257_925_129_965_739e2;
pub(crate) const SB3: f64 = 1.536_729_586_084_437e3;
pub(crate) const SB4: f64 = 3.199_858_219_508_595_5e3;
pub(crate) const SB5: f64 = 2.553_050_406_433_164_4e3;
pub(crate) const SB6: f64 = 4.745_285_412_069_553_7e2;
pub(crate) const SB7: f64 = -2.244_095_244_658_582e1;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the error function of `x`.
///
/// # Description
///
/// Uses the fdlibm piecewise rational approximations (accurate to within one unit
/// in the last place): a direct polynomial near zero, an approximation anchored at
/// `erf(1)` on `[0.84375, 1.25]`, and `erf(x) = 1 - erfc(x)` computed from a scaled
/// exponential for larger arguments.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `erf(x)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub extern "C" fn erf(x: f64) -> f64 {
    let hx: i32 = (x.to_bits() >> 32) as i32;
    let ix: i32 = hx & 0x7fff_ffff;
    if ix >= 0x7ff0_0000 {
        // erf(+/-inf) = +/-1, erf(NaN) = NaN.
        let i: i32 = ((x.to_bits() >> 63) as i32) << 1;
        return f64::from(1 - i) + 1.0 / x;
    }

    if ix < 0x3feb_0000 {
        // |x| < 0.84375
        if ix < 0x3e30_0000 {
            // |x| < 2^-28: avoid underflow.
            return 0.125 * (8.0 * x + EFX8 * x);
        }
        let z: f64 = x * x;
        let r: f64 = PP0 + z * (PP1 + z * (PP2 + z * (PP3 + z * PP4)));
        let s: f64 = 1.0 + z * (QQ1 + z * (QQ2 + z * (QQ3 + z * (QQ4 + z * QQ5))));
        let y: f64 = r / s;
        return x + x * y;
    }
    if ix < 0x3ff4_0000 {
        // 0.84375 <= |x| < 1.25
        let s: f64 = f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff) - 1.0;
        let p: f64 = PA0 + s * (PA1 + s * (PA2 + s * (PA3 + s * (PA4 + s * (PA5 + s * PA6)))));
        let q: f64 = 1.0 + s * (QA1 + s * (QA2 + s * (QA3 + s * (QA4 + s * (QA5 + s * QA6)))));
        if hx >= 0 {
            return ERX + p / q;
        }
        return -ERX - p / q;
    }
    if ix >= 0x4018_0000 {
        // |x| >= 6
        if hx >= 0 {
            return 1.0 - TINY_ERF;
        }
        return TINY_ERF - 1.0;
    }
    let ax: f64 = f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff);
    let s: f64 = 1.0 / (ax * ax);
    let (r, big_s): (f64, f64) = if ix < 0x4006_db6d {
        // |x| < 1/0.35
        let r: f64 =
            RA0 + s * (RA1 + s * (RA2 + s * (RA3 + s * (RA4 + s * (RA5 + s * (RA6 + s * RA7))))));
        let big_s: f64 = 1.0
            + s * (SA1
                + s * (SA2 + s * (SA3 + s * (SA4 + s * (SA5 + s * (SA6 + s * (SA7 + s * SA8)))))));
        (r, big_s)
    } else {
        // |x| >= 1/0.35
        let r: f64 = RB0 + s * (RB1 + s * (RB2 + s * (RB3 + s * (RB4 + s * (RB5 + s * RB6)))));
        let big_s: f64 =
            1.0 + s * (SB1 + s * (SB2 + s * (SB3 + s * (SB4 + s * (SB5 + s * (SB6 + s * SB7))))));
        (r, big_s)
    };
    let z: f64 = f64::from_bits(ax.to_bits() & 0xffff_ffff_0000_0000); // high word only
    let r2: f64 =
        crate::exp::exp(-z * z - 0.5625) * crate::exp::exp((z - ax) * (z + ax) + r / big_s);
    if hx >= 0 {
        1.0 - r2 / ax
    } else {
        r2 / ax - 1.0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    /// Reference values from a correctly-rounded erf (e.g. glibc / MPFR).
    const CASES: [(f64, f64); 7] = [
        (0.0, 0.0),
        (0.1, 0.112_462_916_018_284_89),
        (0.5, 0.520_499_877_813_046_5),
        (1.0, 0.842_700_792_949_714_9),
        (2.0, 0.995_322_265_018_952_7),
        (3.0, 0.999_977_909_503_001_4),
        (4.0, 0.999_999_984_582_742_1),
    ];

    #[test]
    fn test_reference_values() {
        for &(x, want) in &CASES {
            let got: f64 = erf(x);
            assert!(
                (got - want).abs() <= want.abs() * 1e-14 + 1e-15,
                "erf({x}) = {got}, want {want}"
            );
            // erf is odd.
            let got_neg: f64 = erf(-x);
            assert!((got_neg + want).abs() <= want.abs() * 1e-14 + 1e-15);
        }
    }

    #[test]
    fn test_zero() {
        assert_eq!(erf(0.0), 0.0);
        assert_eq!(erf(-0.0).to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn test_limits() {
        assert_eq!(erf(f64::INFINITY), 1.0);
        assert_eq!(erf(f64::NEG_INFINITY), -1.0);
    }

    #[test]
    fn test_nan() {
        assert!(erf(f64::NAN).is_nan());
    }
}
