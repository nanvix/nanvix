// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the next representable value after `x` in the direction of `y`.
///
/// # Parameters
///
/// - `x`: Starting value.
/// - `y`: Direction value.
///
/// # Returns
///
/// The next representable double after `x` toward `y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn nextafter(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x == y {
        return y;
    }
    if x == 0.0 {
        // Smallest subnormal in the direction of y.
        let tiny: f64 = f64::from_bits(1);
        return if y > 0.0 { tiny } else { -tiny };
    }

    let bits: u64 = x.to_bits();
    // Moving away from zero increases the magnitude bits; toward zero decreases.
    let away_from_zero: bool = (y > x) == (x > 0.0);
    let next: u64 = if away_from_zero { bits + 1 } else { bits - 1 };
    f64::from_bits(next)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
