// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(any(feature = "std", test))]
use crate::set_errno;
use crate::signal::sigset_t;
#[cfg(any(feature = "std", test))]
use ::sysapi::errno::EINTR;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Atomically replaces the calling thread's signal mask with `mask` and suspends the thread until a
/// signal whose action is to run a handler is delivered. Once the handler returns, the previous
/// mask is restored.
///
/// POSIX specifies that `sigsuspend()` always returns `-1` with `errno` set to `EINTR` once it is
/// interrupted by a caught signal.
///
/// # Parameters
///
/// - `mask`: The signal mask to install while suspended.
///
/// # Returns
///
/// Always returns `-1`; on the expected path `errno` is `EINTR`.
///
/// # Safety
///
/// This function is unsafe because it is part of the C ABI surface and dereferences `mask`, which
/// must be a valid pointer to a `sigset_t`.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sigsuspend.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigsuspend(mask: *const sigset_t) -> c_int {
    unsafe { sigsuspend_impl(mask) }
}

/// Guest implementation of [`sigsuspend`]: delegates the suspension to the Nanvix kernel.
///
/// # Safety
///
/// This function forwards a raw pointer to the backend. The caller must ensure `mask` is a valid
/// pointer to a `sigset_t`.
#[cfg(not(any(feature = "std", test)))]
unsafe fn sigsuspend_impl(mask: *const sigset_t) -> c_int {
    extern "C" {
        fn __nanvix_sigsuspend(mask: *const sigset_t) -> c_int;
    }

    unsafe { __nanvix_sigsuspend(mask) }
}

/// Host-only implementation of [`sigsuspend`] used by unit tests: reports immediate interruption
/// without blocking.
///
/// # Safety
///
/// This function does not dereference `mask`.
#[cfg(any(feature = "std", test))]
unsafe fn sigsuspend_impl(_mask: *const sigset_t) -> c_int {
    set_errno(EINTR);
    -1
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn sigsuspend_always_reports_interruption() {
        let mask: sigset_t = 0;
        // SAFETY: `mask` points to a valid `sigset_t` for the duration of the call.
        assert_eq!(unsafe { sigsuspend(&mask) }, -1);
    }
}
