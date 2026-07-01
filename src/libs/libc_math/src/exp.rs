// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// `ln(2)` split into a leading part (exact in its high bits) and a low
/// correction, indexed by the sign of the argument so the reduction stays exact.
const LN2HI: [f64; 2] = [6.931_471_803_691_238e-1, -6.931_471_803_691_238e-1];
const LN2LO: [f64; 2] = [1.908_214_929_270_587_7e-10, -1.908_214_929_270_587_7e-10];
/// `+/-0.5`, added before truncation to round the reduction quotient.
const HALF: [f64; 2] = [0.5, -0.5];
/// `1 / ln(2)`.
const INVLN2: f64 = 1.442_695_040_888_963_4;
/// Overflow threshold: `exp(x)` overflows to `+inf` for `x > O_THRESHOLD`.
const O_THRESHOLD: f64 = 7.097_827_128_933_84e2;
/// Underflow threshold: `exp(x)` underflows to `0` for `x < U_THRESHOLD`.
const U_THRESHOLD: f64 = -7.451_332_191_019_411e2;
/// Minimax polynomial coefficients for the scaled remainder `x - r*R(r^2)`.
const P1: f64 = 1.666_666_666_666_660_2e-1;
const P2: f64 = -2.777_777_777_701_559_3e-3;
const P3: f64 = 6.613_756_321_437_934e-5;
const P4: f64 = -1.653_390_220_546_525_2e-6;
const P5: f64 = 4.138_136_797_057_238_5e-8;
/// `2^-1000`, used to defer final scaling for tiny results without underflow.
const TWOM1000: f64 = f64::from_bits(0x0170_0000_0000_0000);

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `e^x`.
///
/// # Description
///
/// Reduces the argument as `x = k*ln(2) + r` with `|r| <= 0.5*ln(2)`, evaluates
/// `e^r` from a minimax polynomial in `r`, and reconstructs the result by scaling
/// with `2^k`. This is the fdlibm algorithm and is accurate to within one unit in
/// the last place.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `e^x`. Overflow saturates to `+inf` and underflow to `0`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub extern "C" fn exp(x: f64) -> f64 {
    let hx: u32 = (x.to_bits() >> 32) as u32;
    let xsb: usize = ((hx >> 31) & 1) as usize; // sign bit of x
    let ax: u32 = hx & 0x7fff_ffff; // high word of |x|

    // Filter out non-finite and out-of-range arguments.
    if ax >= 0x4086_2E42 {
        // |x| >= 709.78...
        if ax >= 0x7ff0_0000 {
            // x is +/-inf or NaN.
            if x.is_nan() {
                return x + x;
            }
            return if xsb == 0 { x } else { 0.0 }; // exp(+inf)=inf, exp(-inf)=0
        }
        if x > O_THRESHOLD {
            return f64::INFINITY; // overflow
        }
        if x < U_THRESHOLD {
            return 0.0; // underflow
        }
    }

    // Argument reduction: x = k*ln(2) + (hi - lo), with r = hi - lo the reduced value.
    let mut k: i32 = 0;
    let mut hi: f64 = 0.0;
    let mut lo: f64 = 0.0;
    let mut r: f64 = x;
    if ax > 0x3fd6_2e42 {
        // |x| > 0.5*ln2
        if ax < 0x3FF0_A2B2 {
            // |x| < 1.5*ln2: a single step of ln2 suffices.
            hi = x - LN2HI[xsb];
            lo = LN2LO[xsb];
            k = 1 - (xsb as i32) - (xsb as i32);
        } else {
            k = (INVLN2 * x + HALF[xsb]) as i32;
            let t: f64 = f64::from(k);
            hi = x - t * LN2HI[0]; // t*LN2HI[0] is exact here
            lo = t * LN2LO[0];
        }
        r = hi - lo;
    } else if ax < 0x3e30_0000 {
        // |x| < 2^-28: exp(x) rounds to 1 + x.
        return 1.0 + x;
    }

    // Evaluate exp(r) on the reduced range and scale by 2^k.
    let t: f64 = r * r;
    let twopk: f64 = if k >= -1021 {
        f64::from_bits(((0x3ff + k) as u64) << 52)
    } else {
        f64::from_bits(((0x3ff + k + 1000) as u64) << 52)
    };
    let c: f64 = r - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    if k == 0 {
        return 1.0 - ((r * c) / (c - 2.0) - r);
    }
    let y: f64 = 1.0 - ((lo - (r * c) / (2.0 - c)) - hi);
    if k >= -1021 {
        if k == 1024 {
            // 2^1024 is not representable; scale as 2^1023 * 2 instead.
            return y * 2.0 * f64::from_bits(0x7fe0_0000_0000_0000);
        }
        y * twopk
    } else {
        y * twopk * TWOM1000
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
        assert!((exp(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((exp(1.0) - 2.718_281_828_459_045).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((exp(-1.0) - 0.367_879_441_171_442_3).abs() < 1e-10);
    }

    #[test]
    fn test_ln2() {
        assert!((exp(core::f64::consts::LN_2) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_matches_std_over_range() {
        // Compare against the host libm across the full finite range at <= a few ULP.
        let mut x: f64 = -700.0;
        while x <= 700.0 {
            let got: f64 = exp(x);
            let want: f64 = x.exp();
            assert!((got - want).abs() <= want.abs() * 1e-14, "exp({x}) = {got}, want {want}");
            x += 0.013;
        }
    }

    #[test]
    fn test_overflow() {
        assert_eq!(exp(710.0), f64::INFINITY);
    }

    #[test]
    fn test_underflow() {
        assert_eq!(exp(-746.0), 0.0);
    }

    #[test]
    fn test_nan() {
        assert!(exp(f64::NAN).is_nan());
    }

    #[test]
    fn test_overflow_boundary_is_finite() {
        // Inputs below `ln(f64::MAX)` (~= 709.7827) must not overflow prematurely.
        let v: f64 = exp(709.5);
        let expected: f64 = 709.5f64.exp();
        assert!(v.is_finite() && v > 0.0);
        assert!((v - expected).abs() <= expected * 1e-10);
    }

    #[test]
    fn test_underflow_boundary_is_subnormal() {
        // Inputs above `ln(2^-1075)` (~= -745.133) must not underflow to zero.
        let v: f64 = exp(-745.1);
        assert!(v > 0.0);
        assert!(v < f64::MIN_POSITIVE);
    }
}
