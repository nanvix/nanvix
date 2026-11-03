// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares the first `n` bytes of the memory areas `s1` and `s2` (legacy BSD interface).
///
/// # Returns
///
/// Zero if the byte ranges are equal, and non-zero otherwise.
///
/// # Safety
///
/// Both `s1` and `s2` must point to at least `n` readable bytes.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/bcmp.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn bcmp(s1: *const c_void, s2: *const c_void, n: c_size_t) -> c_int {
    let a: *const u8 = s1.cast::<u8>();
    let b: *const u8 = s2.cast::<u8>();
    let mut i: c_size_t = 0;
    while i < n {
        if *a.add(i as usize) != *b.add(i as usize) {
            return 1;
        }
        i += 1;
    }
    0
}

#[cfg(all(test, feature = "std"))]
mod test;
