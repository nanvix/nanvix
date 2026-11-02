// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the IEEE-754 remainder of `x / y`.
///
/// The result is `x - n*y`, where `n` is the integer nearest the exact value of `x/y`; when
/// `|x/y - n| == 1/2`, `n` is chosen to be even.
///
/// # Parameters
///
/// - `x`: Dividend.
/// - `y`: Divisor.
///
/// # Returns
///
/// The value `remainder(x, y)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn remainder(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() || x.is_infinite() || y == 0.0 {
        return f64::NAN;
    }
    if y.is_infinite() {
        return x;
    }
    let q: f64 = x / y;
    let n: f64 = round_ties_even(q);
    x - n * y
}

/// Rounds `v` to the nearest integer, rounding halves to the nearest even integer.
fn round_ties_even(v: f64) -> f64 {
    let floor: f64 = crate::floor::floor(v);
    let diff: f64 = v - floor;
    if diff < 0.5 {
        floor
    } else if diff > 0.5 {
        floor + 1.0
    } else {
        // Halfway: pick the even neighbor.
        let half: f64 = floor * 0.5;
        if half == crate::floor::floor(half) {
            floor
        } else {
            floor + 1.0
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
