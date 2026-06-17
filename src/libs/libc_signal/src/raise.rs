// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
#[cfg(not(feature = "std"))]
use ::sysapi::sys_types::pid_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a signal to the calling process (equivalent to `kill(getpid(), sig)`).
///
/// # Parameters
///
/// - `sig`: The signal number to send.
///
/// # Returns
///
/// Zero on success, or a non-zero value on error.
///
/// # Safety
///
/// This function is unsafe because it calls external system calls (`getpid` and `kill`) that
/// modify process state.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn raise(sig: c_int) -> c_int {
    // SAFETY: delegates to the platform-specific implementation, which upholds this function's
    // contract.
    unsafe { raise_impl(sig) }
}

/// Guest implementation of [`raise`]: delivers `sig` to the calling process through
/// `kill(getpid(), sig)`.
///
/// # Safety
///
/// This function is unsafe because it calls the external `getpid` and `kill` system calls that
/// modify process state.
#[cfg(not(feature = "std"))]
unsafe fn raise_impl(sig: c_int) -> c_int {
    extern "C" {
        fn getpid() -> pid_t;
        fn kill(pid: pid_t, sig: c_int) -> c_int;
    }

    // SAFETY: both calls are FFI to the guest C runtime and have no preconditions here.
    unsafe { kill(getpid(), sig) }
}

/// Host-only stand-in for [`raise_impl`] used by unit tests.
///
/// Signal delivery is unavailable on the host, so this reports failure without referencing any
/// guest-only symbol. The guest never compiles this variant.
///
/// # Safety
///
/// Matches the signature of the guest variant; it has no additional preconditions.
#[cfg(feature = "std")]
unsafe fn raise_impl(_sig: c_int) -> c_int {
    -1
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn raise_reports_failure_on_host() {
        // The host build cannot deliver signals, so raise() must fail rather than reference any
        // guest-only symbol. This guards against reintroducing an unconditional FFI call that
        // would break the host build.
        assert_eq!(unsafe { raise(2) }, -1);
    }
}
