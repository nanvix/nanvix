// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::signal::sigset_t;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Stores, in the location referenced by `set`, the set of signals that are pending on the calling
/// process and blocked from delivery to the calling thread.
///
/// # Parameters
///
/// - `set`: Receives the pending-but-blocked signal set.
///
/// # Returns
///
/// Zero on success, or `-1` on error with `errno` set.
///
/// # Safety
///
/// This function is unsafe because it is part of the C ABI surface and dereferences `set`, which
/// must be a valid pointer to a `sigset_t`.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sigpending.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigpending(set: *mut sigset_t) -> c_int {
    unsafe { sigpending_impl(set) }
}

/// Guest implementation of [`sigpending`]: queries the Nanvix kernel.
///
/// # Safety
///
/// This function forwards a raw pointer to the backend. The caller must ensure `set` is a valid
/// pointer to a `sigset_t`.
#[cfg(not(any(feature = "std", test)))]
unsafe fn sigpending_impl(set: *mut sigset_t) -> c_int {
    extern "C" {
        fn __nanvix_sigpending(set: *mut sigset_t) -> c_int;
    }

    unsafe { __nanvix_sigpending(set) }
}

/// Host-only implementation of [`sigpending`] used by unit tests: reports an empty pending set.
///
/// # Safety
///
/// This function dereferences `set` when it is non-null. The caller must ensure it is a valid
/// pointer to a `sigset_t`.
#[cfg(any(feature = "std", test))]
unsafe fn sigpending_impl(set: *mut sigset_t) -> c_int {
    if !set.is_null() {
        unsafe {
            *set = 0;
        }
    }
    0
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn sigpending_reports_empty_set() {
        let mut set: sigset_t = 0xffff_ffff_ffff_ffff;
        // SAFETY: `set` points to a valid `sigset_t` for the duration of the call.
        assert_eq!(unsafe { sigpending(&mut set) }, 0);
        assert_eq!(set, 0);
    }
}
