// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    set_errno,
    signal::sigset_t,
};
use ::sysapi::{
    errno::EINVAL,
    ffi::c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// `how` argument to [`sigprocmask`]: add the signals in `set` to the mask.
pub const SIG_BLOCK: c_int = 0;
/// `how` argument to [`sigprocmask`]: remove the signals in `set` from the mask.
pub const SIG_UNBLOCK: c_int = 1;
/// `how` argument to [`sigprocmask`]: replace the mask with `set`.
pub const SIG_SETMASK: c_int = 2;

/// Signals that can never be blocked: `SIGKILL` (9) and `SIGSTOP` (19).
///
/// POSIX requires that any attempt to block these signals be silently ignored rather than
/// reported as an error, so they are masked out of every computed blocked-signal set.
const UNBLOCKABLE: sigset_t = (1u64 << (9 - 1)) | (1u64 << (19 - 1));

//==================================================================================================
// Static Data
//==================================================================================================

/// Process-wide blocked-signal mask.
///
/// Nanvix does not yet deliver asynchronous signals, so the mask is pure
/// bookkeeping: it is faithfully maintained so that callers which save and
/// restore the mask (e.g. xz's single-threaded `mythread_sigmask`) observe
/// consistent values, but it has no effect on signal delivery.
static mut SIGNAL_MASK: sigset_t = 0;

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
    // SAFETY: SIGNAL_MASK is only accessed from this single-threaded library.
    let current: sigset_t = unsafe { SIGNAL_MASK };

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
        // SAFETY: single-threaded access to the process mask.
        unsafe { SIGNAL_MASK = next };
    }

    0
}

/// Computes the updated blocked-signal mask.
///
/// Combines the `current` mask with `value` according to `how`. Returns [`None`] when `how` is not
/// one of [`SIG_BLOCK`], [`SIG_UNBLOCK`], or [`SIG_SETMASK`].
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
mod test {
    use super::*;

    #[test]
    fn apply_how_block_computes_union() {
        assert_eq!(apply_how(SIG_BLOCK, 0b0001, 0b0100), Some(0b0101));
    }

    #[test]
    fn apply_how_unblock_clears_requested_bits() {
        assert_eq!(apply_how(SIG_UNBLOCK, 0b0111, 0b0010), Some(0b0101));
    }

    #[test]
    fn apply_how_setmask_replaces_mask() {
        assert_eq!(apply_how(SIG_SETMASK, 0b1111, 0b0010), Some(0b0010));
    }

    #[test]
    fn apply_how_rejects_invalid_how() {
        assert_eq!(apply_how(42, 0, 0), None);
        assert_eq!(apply_how(-1, 0, 0), None);
    }

    #[test]
    fn sigprocmask_round_trips_through_the_process_mask() {
        // This is the only test that touches the process-wide SIGNAL_MASK; keeping the whole
        // sequence in a single test keeps it deterministic regardless of test execution order.
        unsafe {
            // Start from a known, empty mask.
            let empty: sigset_t = 0;
            assert_eq!(sigprocmask(SIG_SETMASK, &empty, core::ptr::null_mut()), 0);

            // Blocking reports the previous (empty) mask through `oldset`.
            let to_block: sigset_t = 0b1010;
            let mut old: sigset_t = u64::MAX;
            assert_eq!(sigprocmask(SIG_BLOCK, &to_block, &mut old), 0);
            assert_eq!(old, 0);

            // A null `set` leaves the mask unchanged and returns it through `oldset`.
            let mut current: sigset_t = 0;
            assert_eq!(sigprocmask(SIG_BLOCK, core::ptr::null(), &mut current), 0);
            assert_eq!(current, 0b1010);

            // Unblocking clears only the requested bits.
            let to_unblock: sigset_t = 0b0010;
            assert_eq!(sigprocmask(SIG_UNBLOCK, &to_unblock, core::ptr::null_mut()), 0);
            let mut after: sigset_t = u64::MAX;
            assert_eq!(sigprocmask(SIG_BLOCK, &empty, &mut after), 0);
            assert_eq!(after, 0b1000);

            // An invalid `how` combined with a non-null `set` is rejected.
            let mut ignored: sigset_t = 0;
            assert_eq!(sigprocmask(999, &to_block, &mut ignored), -1);

            // SIGKILL and SIGSTOP can never be blocked (POSIX); attempts are silently ignored,
            // whether requested through SIG_SETMASK or SIG_BLOCK.
            let kill_stop: sigset_t = (1 << (9 - 1)) | (1 << (19 - 1));
            assert_eq!(sigprocmask(SIG_SETMASK, &kill_stop, core::ptr::null_mut()), 0);
            let mut after_setmask: sigset_t = u64::MAX;
            assert_eq!(sigprocmask(SIG_BLOCK, &empty, &mut after_setmask), 0);
            assert_eq!(after_setmask & kill_stop, 0);
            assert_eq!(sigprocmask(SIG_BLOCK, &kill_stop, core::ptr::null_mut()), 0);
            let mut after_block: sigset_t = u64::MAX;
            assert_eq!(sigprocmask(SIG_BLOCK, &empty, &mut after_block), 0);
            assert_eq!(after_block & kill_stop, 0);

            // Leave the global mask clean for any other consumers.
            assert_eq!(sigprocmask(SIG_SETMASK, &empty, core::ptr::null_mut()), 0);
        }
    }
}
