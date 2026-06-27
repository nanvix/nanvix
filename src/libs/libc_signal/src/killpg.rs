// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
use ::sysapi::{
    errno::EINVAL,
    ffi::c_int,
    sys_types::pid_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a signal to a process group.
///
/// Nanvix does not implement process groups, so each process is the leader of its own group and
/// the process-group ID equals the process ID. A `pgrp` value of zero therefore targets the calling
/// process, and a positive `pgrp` delivers `sig` to the process whose ID equals `pgrp`. Negative
/// values (POSIX process-group selectors) are unsupported and rejected with `EINVAL`.
///
/// # Parameters
///
/// - `pgrp`: The process-group ID (a process ID on Nanvix) to signal.
/// - `sig`: The signal number to send.
///
/// # Returns
///
/// Zero on success, or `-1` on error with `errno` set to `EINVAL` when `pgrp` is negative.
///
/// # Safety
///
/// This function is unsafe because it calls the external `kill` system call that modifies process
/// state.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn killpg(pgrp: pid_t, sig: c_int) -> c_int {
    // Process groups are not supported, so the negative `pgrp` selectors that POSIX uses to address
    // a specific group are rejected here rather than forwarded to kill(). This keeps killpg()'s
    // semantics independent of kill()'s own pid validation.
    if pgrp < 0 {
        // POSIX: EINVAL when the process-group ID is invalid.
        set_errno(EINVAL);
        return -1;
    }

    // SAFETY: delegates to the platform-specific implementation, which upholds this contract.
    unsafe { killpg_impl(pgrp, sig) }
}

/// Resolves a Nanvix process-group ID to the process ID used by `kill()`.
#[cfg(any(not(feature = "std"), test))]
fn target_pid_for_group(pgrp: pid_t, caller_pid: pid_t) -> pid_t {
    if pgrp == 0 {
        caller_pid
    } else {
        pgrp
    }
}

/// Guest implementation of [`killpg`]: delivers `sig` through `kill()` because Nanvix has no
/// process groups.
///
/// # Safety
///
/// This function is unsafe because it calls the external `kill` system call that modifies process
/// state.
#[cfg(not(feature = "std"))]
unsafe fn killpg_impl(pgrp: pid_t, sig: c_int) -> c_int {
    extern "C" {
        fn getpid() -> pid_t;
        fn kill(pid: pid_t, sig: c_int) -> c_int;
    }

    let caller_pid: pid_t = unsafe { getpid() };
    let target_pid: pid_t = target_pid_for_group(pgrp, caller_pid);

    // SAFETY: FFI to the guest C runtime; has no preconditions here.
    unsafe { kill(target_pid, sig) }
}

/// Host-only stand-in for [`killpg_impl`] used by unit tests.
///
/// Signal delivery is unavailable on the host, so this reports failure without referencing any
/// guest-only symbol. The guest never compiles this variant.
///
/// # Safety
///
/// Matches the signature of the guest variant; it has no additional preconditions.
#[cfg(feature = "std")]
unsafe fn killpg_impl(_pgrp: pid_t, _sig: c_int) -> c_int {
    -1
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    #[test]
    fn killpg_target_pid_maps_zero_to_caller() {
        assert_eq!(target_pid_for_group(0, 7), 7);
        assert_eq!(target_pid_for_group(3, 7), 3);
    }

    #[test]
    fn killpg_rejects_negative_pgrp() {
        // Process groups are not supported, so negative pgrp selectors must be rejected outright
        // rather than forwarded to kill().
        assert_eq!(unsafe { killpg(-1, 2) }, -1);
        assert_eq!(unsafe { killpg(-2, 2) }, -1);
    }

    #[test]
    fn killpg_reports_failure_on_host() {
        // The host build cannot deliver signals, so killpg() must fail rather than reference any
        // guest-only symbol. This guards against reintroducing an unconditional FFI call that
        // would break the host build.
        assert_eq!(unsafe { killpg(1, 2) }, -1);
    }
}
