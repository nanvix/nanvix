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
// Constants
//==================================================================================================

/// Modulus for the LCG output range. `rand()` returns values in `[0, RAND_RANGE - 1]`.
const RAND_RANGE: c_uint = 32768;

//==================================================================================================
// Global State
//==================================================================================================

/// Internal LCG seed state.
static mut NEXT: c_uint = 1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Generates a pseudo-random integer in the range `[0, 32767]`.
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
    NEXT = NEXT.wrapping_mul(1_103_515_245).wrapping_add(12345);
    c_int::try_from((NEXT / 65536) % RAND_RANGE).unwrap_or_default()
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
    NEXT = seed;
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        rand,
        srand,
    };

    const RAND_MAX: i32 = 32767;

    #[test]
    fn deterministic_sequence() {
        unsafe { srand(1) };
        let first = unsafe { rand() };
        let second = unsafe { rand() };
        unsafe { srand(1) };
        assert_eq!(unsafe { rand() }, first);
        assert_eq!(unsafe { rand() }, second);
    }

    #[test]
    fn values_in_range() {
        unsafe { srand(42) };
        for _ in 0..100 {
            let val = unsafe { rand() };
            assert!(val >= 0);
            assert!(val <= RAND_MAX);
        }
    }
}
