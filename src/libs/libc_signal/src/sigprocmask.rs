// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(any(feature = "std", test))]
use crate::set_errno;
use crate::signal::sigset_t;
#[cfg(any(feature = "std", test))]
use ::core::sync::atomic::{
    AtomicU64,
    Ordering,
};
#[cfg(not(any(feature = "std", test)))]
use ::sysapi::ffi::c_int;
#[cfg(any(feature = "std", test))]
use ::sysapi::{
    errno::EINVAL,
    ffi::c_int,
    signal::{
        SIGKILL,
        SIGSTOP,
    },
};

//==================================================================================================
// Re-exports
//==================================================================================================

pub use ::sysapi::signal::{
    SIG_BLOCK,
    SIG_SETMASK,
    SIG_UNBLOCK,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Signals that can never be blocked: `SIGKILL` (9) and `SIGSTOP` (19).
///
/// POSIX requires that any attempt to block these signals be silently ignored rather than
/// reported as an error, so they are masked out of every computed blocked-signal set.
#[cfg(any(feature = "std", test))]
const UNBLOCKABLE: sigset_t = (1u64 << (SIGKILL - 1)) | (1u64 << (SIGSTOP - 1));

//==================================================================================================
// Static Data
//==================================================================================================

/// Process-wide blocked-signal mask.
///
/// Nanvix does not yet deliver asynchronous signals, so the mask is pure
/// bookkeeping: it is faithfully maintained so that callers which save and
/// restore the mask (e.g. xz's single-threaded `mythread_sigmask`) observe
/// consistent values, but it has no effect on signal delivery.
#[cfg(any(feature = "std", test))]
static SIGNAL_MASK: AtomicU64 = AtomicU64::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Examines and/or changes the calling process's blocked-signal mask.
///
/// # Parameters
///
/// - `how`: One of [`SIG_BLOCK`], [`SIG_UNBLOCK`], or [`SIG_SETMASK`].
/// - `set`: New signals to apply per `how`, or null to leave the mask unchanged.
/// - `oldset`: Receives the previous mask, or null if not wanted.
///
/// # Returns
///
/// Zero on success, or -1 on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `set` and `oldset`.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sigprocmask.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigprocmask(
    how: c_int,
    set: *const sigset_t,
    oldset: *mut sigset_t,
) -> c_int {
    unsafe { sigprocmask_impl(how, set, oldset) }
}

/// Guest implementation of [`sigprocmask`]: delegates mask state to the Nanvix kernel.
///
/// # Safety
///
/// This function forwards raw pointers to the backend. The caller must ensure `set` and `oldset`
/// are either null or valid pointers to `sigset_t` values.
#[cfg(not(any(feature = "std", test)))]
unsafe fn sigprocmask_impl(how: c_int, set: *const sigset_t, oldset: *mut sigset_t) -> c_int {
    extern "C" {
        fn __nanvix_sigprocmask(how: c_int, set: *const sigset_t, oldset: *mut sigset_t) -> c_int;
    }

    unsafe { __nanvix_sigprocmask(how, set, oldset) }
}

/// Host-only implementation of [`sigprocmask`] used by unit tests.
///
/// # Safety
///
/// This function dereferences `set` and `oldset` when they are non-null. The caller must ensure
/// they are valid pointers to `sigset_t` values.
#[cfg(any(feature = "std", test))]
unsafe fn sigprocmask_impl(how: c_int, set: *const sigset_t, oldset: *mut sigset_t) -> c_int {
    let current: sigset_t = SIGNAL_MASK.load(Ordering::Relaxed);

    if !oldset.is_null() {
        *oldset = current;
    }

    if !set.is_null() {
        let value: sigset_t = *set;
        let next: sigset_t = match apply_how(how, current, value) {
            // SIGKILL and SIGSTOP can never be blocked; drop them silently (not an error).
            Some(next) => next & !UNBLOCKABLE,
            None => {
                // POSIX: sigprocmask() shall fail with EINVAL when `how` is invalid.
                set_errno(EINVAL);
                return -1;
            },
        };
        SIGNAL_MASK.store(next, Ordering::Relaxed);
    }

    0
}

/// Computes the updated blocked-signal mask.
///
/// Combines the `current` mask with `value` according to `how`. Returns [`None`] when `how` is not
/// one of [`SIG_BLOCK`], [`SIG_UNBLOCK`], or [`SIG_SETMASK`].
#[cfg(any(feature = "std", test))]
fn apply_how(how: c_int, current: sigset_t, value: sigset_t) -> Option<sigset_t> {
    match how {
        SIG_BLOCK => Some(current | value),
        SIG_UNBLOCK => Some(current & !value),
        SIG_SETMASK => Some(value),
        _ => None,
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
