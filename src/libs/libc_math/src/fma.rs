// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Helpers
//==================================================================================================

/// Splits `x` into high and low parts using Veltkamp's algorithm, such that `hi + lo == x` and
/// each part holds at most 26 significand bits.
fn split(x: f64) -> (f64, f64) {
    // 2^27 + 1.
    const FACTOR: f64 = 134_217_729.0;
    let t: f64 = FACTOR * x;
    let hi: f64 = t - (t - x);
    let lo: f64 = x - hi;
    (hi, lo)
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `x * y + z` as a single ternary operation, rounded once.
///
/// # Description
///
/// Uses Dekker's two-product algorithm to obtain the exact product `x * y = p + e`, then a two-sum
/// to add `z` with a single final rounding. This avoids the double rounding of the naive
/// expression on targets without a hardware fused-multiply-add instruction.
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
pub extern "C" fn fma(x: f64, y: f64, z: f64) -> f64 {
    // Non-finite or zero factors: the naive expression already yields IEEE-correct propagation.
    if !x.is_finite() || !y.is_finite() || !z.is_finite() || x == 0.0 || y == 0.0 {
        return x * y + z;
    }

    // TwoProduct: x * y = p + e exactly.
    let p: f64 = x * y;
    if !p.is_finite() {
        return p + z;
    }
    let (xh, xl): (f64, f64) = split(x);
    let (yh, yl): (f64, f64) = split(y);
    let e: f64 = ((xh * yh - p) + xh * yl + xl * yh) + xl * yl;

    // TwoSum: p + z = s + t exactly.
    let s: f64 = p + z;
    let bb: f64 = s - p;
    let t: f64 = (p - (s - bb)) + (z - bb);

    s + (t + e)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
