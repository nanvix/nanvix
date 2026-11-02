// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
