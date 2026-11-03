// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Finds the position of the first (least significant) set bit in `i`.
///
/// # Returns
///
/// The 1-based index of the first set bit, or 0 if `i` is zero.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/ffs.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ffs(i: c_int) -> c_int {
    if i == 0 {
        return 0;
    }
    // For a non-zero `i32`, `trailing_zeros()` ∈ 0..=31, so the 1-based index
    // (`+ 1`) is 1..=32 and always converts cleanly to `c_int` (the `Err` arm is
    // unreachable). Using `try_from` avoids an unchecked `as` cast, which this
    // crate forbids (clippy::cast_possible_truncation / cast_possible_wrap).
    match c_int::try_from(i.trailing_zeros()) {
        Ok(n) => n + 1,
        Err(_) => 0,
    }
}

#[cfg(all(test, feature = "std"))]
mod test;
