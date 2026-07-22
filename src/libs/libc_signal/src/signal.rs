// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
use ::sysapi::{
    errno::EINVAL,
    ffi::c_int,
};

//==================================================================================================
// Re-exports
//==================================================================================================

pub use ::sysapi::signal::{
    sig_atomic_t,
    sigaction_t,
    siginfo_t,
    sigset_t,
    sigval,
    SignalAction,
    SignalHandler,
    SIG_DFL,
    SIG_ERR,
    SIG_IGN,
    SIG_MAX,
};

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Returns `true` if `signum` is a signal number that `signal()` accepts.
///
/// Valid signal numbers are `1..=SIG_MAX`; the null signal `0` and out-of-range values are
/// rejected.
fn is_valid_signum(signum: c_int) -> bool {
    signum > 0 && signum <= SIG_MAX
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Establishes a simplified signal handler for the given signal number.
///
/// This wraps the `sigaction` system call to provide the traditional `signal()` interface.
///
/// # Parameters
///
/// - `signum`: The signal number to handle.
/// - `handler`: The new signal handler, or `SIG_DFL`/`SIG_IGN`.
///
/// # Returns
///
/// The previous signal handler on success, or `SIG_ERR` on failure.
///
/// # Safety
///
/// This function is unsafe because it calls the external `sigaction` system call and modifies
/// process-wide signal handling state.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn signal(signum: c_int, handler: SignalHandler) -> SignalHandler {
    if !is_valid_signum(signum) {
        // POSIX: signal() shall fail with EINVAL when `signum` is not a valid signal number.
        set_errno(EINVAL);
        return SIG_ERR;
    }

    // SAFETY: `signum` has been validated above; `install_handler` upholds the same contract as
    // this function.
    unsafe { install_handler(signum, handler) }
}

/// Installs `handler` for the already-validated signal `signum` via the `sigaction` kernel call.
///
/// Returns the previous handler, or [`SIG_ERR`] if the underlying call fails.
///
/// # Safety
///
/// This function is unsafe because it calls the external `sigaction` system call and modifies
/// process-wide signal handling state.
#[cfg(not(feature = "std"))]
unsafe fn install_handler(signum: c_int, handler: SignalHandler) -> SignalHandler {
    extern "C" {
        fn sigaction(sig: c_int, act: *const sigaction_t, oact: *mut sigaction_t) -> c_int;
    }

    let act: sigaction_t = sigaction_t {
        sa_handler: handler,
        sa_mask: 0,
        sa_flags: 0,
        sa_sigaction: None,
    };
    let mut old_act: sigaction_t = sigaction_t::new();

    // SAFETY: `act` and `old_act` are valid, properly aligned `sigaction_t` values.
    if unsafe { sigaction(signum, &act, &mut old_act) } != 0 {
        return SIG_ERR;
    }

    old_act.sa_handler
}

/// Host-only stand-in for [`install_handler`] used by unit tests.
///
/// The guest `sigaction` kernel call is unavailable on the host, so this reports failure via
/// [`SIG_ERR`] without referencing any guest-only symbol. The guest never compiles this variant.
///
/// # Safety
///
/// Matches the signature of the guest variant; it has no additional preconditions.
#[cfg(feature = "std")]
unsafe fn install_handler(_signum: c_int, _handler: SignalHandler) -> SignalHandler {
    SIG_ERR
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn sig_err_is_non_null_sentinel() {
        // SIG_ERR must be a non-null value so that C callers can distinguish it from a real
        // handler or from SIG_DFL (null).
        assert_ne!(SIG_ERR, SIG_DFL, "SIG_ERR must be a non-null sentinel");
    }

    #[test]
    fn is_valid_signum_accepts_in_range_signals() {
        assert!(is_valid_signum(1));
        assert!(is_valid_signum(SIG_MAX));
    }

    #[test]
    fn is_valid_signum_rejects_out_of_range_signals() {
        assert!(!is_valid_signum(0));
        assert!(!is_valid_signum(-1));
        assert!(!is_valid_signum(SIG_MAX + 1));
    }

    #[test]
    fn signal_rejects_invalid_signum() {
        // Invalid signal numbers must fail with SIG_ERR (a non-null sentinel) rather than touch
        // kernel state.
        assert_eq!(unsafe { signal(0, SIG_DFL) }, SIG_ERR);
        assert_eq!(unsafe { signal(-1, SIG_DFL) }, SIG_ERR);
        assert_eq!(unsafe { signal(SIG_MAX + 1, SIG_DFL) }, SIG_ERR);
    }
}
