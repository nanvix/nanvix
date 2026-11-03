// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_int,
    c_uint,
};

//==================================================================================================
// Global State
//==================================================================================================

/// Internal LCG seed state. Initialized to match the state produced by `srand(1)`, as required
/// when `rand()` is called before any call to `srand()`.
static mut NEXT: u64 = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Generates a pseudo-random integer in the range `[0, 2147483647]`.
///
/// # Returns
///
/// A pseudo-random integer.
///
/// # Safety
///
/// This function is unsafe because it accesses global mutable state.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/rand.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn rand() -> c_int {
    // 64-bit LCG (constants from musl); return the high 31 bits, which have the best
    // distribution. This yields RAND_MAX == 0x7fffffff, as expected by portable software.
    NEXT = NEXT.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    c_int::try_from(NEXT >> 33).unwrap_or_default()
}

///
/// # Description
///
/// Seeds the pseudo-random number generator used by `rand()`.
///
/// # Parameters
///
/// - `seed`: Seed value.
///
/// # Safety
///
/// This function is unsafe because it accesses global mutable state.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/srand.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn srand(seed: c_uint) {
    // Compute `seed - 1` in 32 bits (matching musl's `unsigned` arithmetic) before widening, so
    // that `srand(0)` yields a state of `0xffff_ffff` rather than `0xffff_ffff_ffff_ffff`.
    NEXT = u64::from(seed.wrapping_sub(1));
}

#[cfg(all(test, feature = "std"))]
mod tests;
