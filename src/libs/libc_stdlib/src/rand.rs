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
mod tests {
    use super::{
        rand,
        srand,
    };
    use crate::RAND_MAX;

    /// Collects `N` consecutive values from `rand()`.
    fn sequence<const N: usize>() -> [i32; N] {
        ::core::array::from_fn(|_| unsafe { rand() })
    }

    // All assertions share a single test because `rand()`/`srand()` operate on a process-global
    // seed. Keeping them in one test serializes access and lets the implicit-seed case observe the
    // pristine initial state before any call to `srand()`.
    #[test]
    fn rand_behaviour() {
        // Calling `rand()` before any call to `srand()` must yield the same sequence as `srand(1)`.
        let implicit: [i32; 8] = sequence();
        unsafe { srand(1) };
        let seeded_one: [i32; 8] = sequence();
        assert_eq!(implicit, seeded_one);

        // The selected LCG matches musl's sequence.
        assert_eq!(
            seeded_one,
            [
                0,
                740_882_966,
                1_616_430_695,
                1_708_849_955,
                1_669_437_588,
                406_334_850,
                276_737_754,
                1_296_416_700,
            ]
        );

        // Re-seeding with the same value repeats the sequence.
        unsafe { srand(1) };
        let first: [i32; 8] = sequence();
        unsafe { srand(1) };
        let second: [i32; 8] = sequence();
        assert_eq!(first, second);

        // Generated values stay within `[0, RAND_MAX]`.
        unsafe { srand(42) };
        for _ in 0..100 {
            let val = unsafe { rand() };
            assert!((0..=RAND_MAX).contains(&val));
        }
    }
}
