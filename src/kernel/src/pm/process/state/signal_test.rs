// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::signal::{
    apply_how,
    compute_blocked,
    default_action,
    exception_to_signal,
    DefaultAction,
    SignalControl,
    SignalDisposition,
    SignalHandler,
    UNBLOCKABLE,
};
use crate::hal::mem::VirtualAddress;
use ::alloc::boxed::Box;
use ::sys::pm::{
    SigAction,
    SIGABRT,
    SIGCHLD,
    SIGCONT,
    SIGFPE,
    SIGHUP,
    SIGILL,
    SIGIO,
    SIGKILL,
    SIGQUIT,
    SIGSEGV,
    SIGSTOP,
    SIGTERM,
    SIGTRAP,
    SIGTSTP,
    SIG_BLOCK,
    SIG_DFL,
    SIG_IGN,
    SIG_SETMASK,
    SIG_UNBLOCK,
};

//==================================================================================================
// Helpers
//==================================================================================================

/// Bit for signal `signum` (1-based) in a signal set.
fn bit(signum: usize) -> u64 {
    1u64 << (signum - 1)
}

/// Builds a catch disposition with the given handler entry, mask, flags, and extended action.
fn handler(entry: usize, mask: u64, flags: i32, sigaction: usize) -> SignalDisposition {
    SignalDisposition::Handler(Box::new(SignalHandler {
        entry: VirtualAddress::new(entry),
        mask,
        flags,
        sigaction,
    }))
}

//==================================================================================================
// Mask Arithmetic Tests
//==================================================================================================

///
/// # Description
///
/// `SIG_BLOCK` unions the requested signals into the current mask.
///
fn test_apply_how_block_computes_union() -> bool {
    if apply_how(SIG_BLOCK, 0b0001, 0b0100) != Some(0b0101) {
        error!("SIG_BLOCK did not compute the union of the masks");
        return false;
    }
    true
}

///
/// # Description
///
/// `SIG_UNBLOCK` clears only the requested signals from the current mask.
///
fn test_apply_how_unblock_clears_requested_bits() -> bool {
    if apply_how(SIG_UNBLOCK, 0b0111, 0b0010) != Some(0b0101) {
        error!("SIG_UNBLOCK did not clear the requested bits");
        return false;
    }
    true
}

///
/// # Description
///
/// `SIG_SETMASK` replaces the current mask with the requested signals.
///
fn test_apply_how_setmask_replaces_mask() -> bool {
    if apply_how(SIG_SETMASK, 0b1111, 0b0010) != Some(0b0010) {
        error!("SIG_SETMASK did not replace the mask");
        return false;
    }
    true
}

///
/// # Description
///
/// An unrecognized `how` value is rejected.
///
fn test_apply_how_rejects_invalid_how() -> bool {
    if apply_how(42, 0, 0).is_some() {
        error!("an invalid `how` value was accepted");
        return false;
    }
    true
}

///
/// # Description
///
/// `SIGKILL` and `SIGSTOP` are silently cleared from any computed mask, even when the caller asks
/// to set a full mask.
///
fn test_compute_blocked_clears_unblockable() -> bool {
    // Setting a full mask must leave SIGKILL/SIGSTOP unblocked.
    match compute_blocked(SIG_SETMASK, 0, u64::MAX) {
        Some(mask) if mask & UNBLOCKABLE == 0 => {},
        other => {
            error!("SIG_SETMASK with a full mask did not clear SIGKILL/SIGSTOP (mask={other:?})");
            return false;
        },
    }

    // Blocking SIGKILL/SIGSTOP explicitly is silently ignored.
    match compute_blocked(SIG_BLOCK, 0, bit(SIGKILL) | bit(SIGSTOP)) {
        Some(0) => {},
        other => {
            error!("blocking SIGKILL/SIGSTOP was not silently ignored (mask={other:?})");
            return false;
        },
    }

    // An invalid `how` is still rejected.
    if compute_blocked(99, 0, 0).is_some() {
        error!("compute_blocked() accepted an invalid `how` value");
        return false;
    }

    true
}

//==================================================================================================
// Disposition Tests
//==================================================================================================

///
/// # Description
///
/// A freshly constructed control block reports the default disposition for every signal.
///
fn test_disposition_defaults_to_default() -> bool {
    let control: SignalControl = SignalControl::default();
    for signum in 1..=64 {
        match control.disposition(signum) {
            Some(SignalDisposition::Default) => {},
            other => {
                error!("signal {signum} was not default-initialized (disposition={other:?})");
                return false;
            },
        }
    }
    true
}

///
/// # Description
///
/// Installing a disposition returns the one it replaced and stores the new one, exercising the
/// atomic swap performed by `sigaction()`.
///
fn test_set_disposition_swaps_and_returns_previous() -> bool {
    let mut control: SignalControl = SignalControl::default();

    // Installing over the initial default returns the default.
    match control.set_disposition(1, handler(0x1000, 0b10, 4, 0x3000)) {
        Some(SignalDisposition::Default) => {},
        other => {
            error!("installing over the default did not return the default (old={other:?})");
            return false;
        },
    }

    // The handler is now the active disposition.
    match control.disposition(1) {
        Some(SignalDisposition::Handler(installed))
            if installed.entry == VirtualAddress::new(0x1000)
                && installed.mask == 0b10
                && installed.flags == 4
                && installed.sigaction == 0x3000 => {},
        other => {
            error!("the installed handler was not stored verbatim (disposition={other:?})");
            return false;
        },
    }

    // Installing again returns the previous handler.
    match control.set_disposition(1, SignalDisposition::Ignore) {
        Some(SignalDisposition::Handler(old)) if old.entry == VirtualAddress::new(0x1000) => {},
        other => {
            error!("installing over a handler did not return the handler (old={other:?})");
            return false;
        },
    }

    // Other signals remain untouched.
    match control.disposition(2) {
        Some(SignalDisposition::Default) => {},
        other => {
            error!("an unrelated signal disposition was modified (disposition={other:?})");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Out-of-range signal numbers neither install nor report a disposition.
///
fn test_set_disposition_rejects_out_of_range() -> bool {
    let mut control: SignalControl = SignalControl::default();

    if control.disposition(0).is_some() {
        error!("signal number 0 was treated as in range");
        return false;
    }
    if control.disposition(65).is_some() {
        error!("signal number 65 was treated as in range");
        return false;
    }
    if control
        .set_disposition(0, SignalDisposition::Ignore)
        .is_some()
    {
        error!("set_disposition() accepted signal number 0");
        return false;
    }
    if control
        .set_disposition(65, SignalDisposition::Ignore)
        .is_some()
    {
        error!("set_disposition() accepted signal number 65");
        return false;
    }

    true
}

///
/// # Description
///
/// Each disposition renders to the matching `oldact` structure returned by `sigaction()`.
///
fn test_to_sigaction_round_trips_dispositions() -> bool {
    let default_action: SigAction = SignalDisposition::Default.to_sigaction();
    if default_action.sa_handler != SIG_DFL {
        error!("default disposition did not render as SIG_DFL");
        return false;
    }

    let ignore_action: SigAction = SignalDisposition::Ignore.to_sigaction();
    if ignore_action.sa_handler != SIG_IGN {
        error!("ignore disposition did not render as SIG_IGN");
        return false;
    }

    let handler_action: SigAction = handler(0x2000, 0b101, 8, 0x4000).to_sigaction();
    if handler_action.sa_handler != 0x2000
        || handler_action.sa_mask != 0b101
        || handler_action.sa_flags != 8
        || handler_action.sa_sigaction != 0x4000
    {
        error!("handler disposition did not render its entry, mask, flags, and extended action");
        return false;
    }

    true
}

//==================================================================================================
// Default-Action Tests
//==================================================================================================

///
/// # Description
///
/// Signals that default to terminating the process report [`DefaultAction::Terminate`], including
/// unassigned and real-time signals.
///
fn test_default_action_terminates() -> bool {
    for signum in [SIGHUP, SIGTERM, SIGKILL, SIGIO] {
        if default_action(signum) != DefaultAction::Terminate {
            error!("signal {signum} did not default to Terminate");
            return false;
        }
    }
    if default_action(40) != DefaultAction::Terminate {
        error!("real-time signal did not default to Terminate");
        return false;
    }
    true
}

///
/// # Description
///
/// Signals that default to a core-dumping termination report [`DefaultAction::Core`].
///
fn test_default_action_cores() -> bool {
    for signum in [SIGQUIT, SIGABRT, SIGSEGV] {
        if default_action(signum) != DefaultAction::Core {
            error!("signal {signum} did not default to Core");
            return false;
        }
    }
    true
}

///
/// # Description
///
/// `SIGCHLD` is ignored by default.
///
fn test_default_action_ignores() -> bool {
    if default_action(SIGCHLD) != DefaultAction::Ignore {
        error!("SIGCHLD did not default to Ignore");
        return false;
    }
    true
}

///
/// # Description
///
/// Job-control signals report [`DefaultAction::Stop`] and [`DefaultAction::Continue`].
///
fn test_default_action_stop_and_continue() -> bool {
    for signum in [SIGSTOP, SIGTSTP] {
        if default_action(signum) != DefaultAction::Stop {
            error!("signal {signum} did not default to Stop");
            return false;
        }
    }
    if default_action(SIGCONT) != DefaultAction::Continue {
        error!("SIGCONT did not default to Continue");
        return false;
    }
    true
}

//==================================================================================================
// Synchronous Exception Mapping Tests
//==================================================================================================

///
/// # Description
///
/// Synchronous CPU exceptions map to the signals defined by the signals design: `#DE`->`SIGFPE`,
/// `#UD`->`SIGILL`, `#GP`/`#PF`->`SIGSEGV`, and `#BP`/`#DB`->`SIGTRAP`.
///
fn test_exception_to_signal_maps_known_vectors() -> bool {
    // (vector, expected signal): #DE=0, #DB=1, #BP=3, #UD=6, #GP=13, #PF=14.
    let cases: [(u32, usize); 6] = [
        (0, SIGFPE),
        (1, SIGTRAP),
        (3, SIGTRAP),
        (6, SIGILL),
        (13, SIGSEGV),
        (14, SIGSEGV),
    ];
    for (vector, expected) in cases {
        if exception_to_signal(vector) != Some(expected) {
            error!("vector {vector} did not map to signal {expected}");
            return false;
        }
    }
    true
}

///
/// # Description
///
/// Exception vectors without a synchronous-signal mapping (and out-of-range vectors) report
/// [`None`], so the exception path leaves their handling unchanged.
///
fn test_exception_to_signal_ignores_unmapped_vectors() -> bool {
    // #NMI=2, #CSO=9, #MF (x87 FP)=16, #AC=17, plus an out-of-range vector.
    for vector in [2u32, 9, 16, 17, 100] {
        if exception_to_signal(vector).is_some() {
            error!("vector {vector} unexpectedly mapped to a signal");
            return false;
        }
    }
    true
}

//==================================================================================================
// Pending-Set Tests
//==================================================================================================

///
/// # Description
///
/// `post()` records a signal in the pending set and reports whether it was newly posted.
///
fn test_post_records_pending_signal() -> bool {
    let mut control: SignalControl = SignalControl::default();
    if control.pending() != 0 {
        error!("a fresh signal control block had pending signals");
        return false;
    }
    if !control.post(SIGTERM) {
        error!("post() did not report SIGTERM as newly posted");
        return false;
    }
    if control.pending() != bit(SIGTERM) {
        error!("post() did not set the SIGTERM pending bit");
        return false;
    }
    // Re-posting an already-pending signal does not report it as newly posted.
    if control.post(SIGTERM) {
        error!("post() reported an already-pending signal as newly posted");
        return false;
    }
    if !control.post(SIGHUP) {
        error!("post() did not report SIGHUP as newly posted");
        return false;
    }
    if control.pending() != bit(SIGTERM) | bit(SIGHUP) {
        error!("post() did not accumulate pending signals");
        return false;
    }
    true
}

///
/// # Description
///
/// `post()` rejects out-of-range signal numbers and leaves the pending set unchanged.
///
fn test_post_rejects_out_of_range() -> bool {
    let mut control: SignalControl = SignalControl::default();
    if control.post(0) {
        error!("post() accepted signal number 0");
        return false;
    }
    if control.post(65) {
        error!("post() accepted signal number 65");
        return false;
    }
    if control.pending() != 0 {
        error!("post() modified the pending set for an out-of-range signal");
        return false;
    }
    true
}

//==================================================================================================
// Test Aggregator
//==================================================================================================

///
/// # Description
///
/// Runs all in-kernel unit tests for the signal control block and signal-mask arithmetic.
///
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_apply_how_block_computes_union);
    passed &= run_test!(test_apply_how_unblock_clears_requested_bits);
    passed &= run_test!(test_apply_how_setmask_replaces_mask);
    passed &= run_test!(test_apply_how_rejects_invalid_how);
    passed &= run_test!(test_compute_blocked_clears_unblockable);
    passed &= run_test!(test_disposition_defaults_to_default);
    passed &= run_test!(test_set_disposition_swaps_and_returns_previous);
    passed &= run_test!(test_set_disposition_rejects_out_of_range);
    passed &= run_test!(test_to_sigaction_round_trips_dispositions);
    passed &= run_test!(test_default_action_terminates);
    passed &= run_test!(test_default_action_cores);
    passed &= run_test!(test_default_action_ignores);
    passed &= run_test!(test_default_action_stop_and_continue);
    passed &= run_test!(test_exception_to_signal_maps_known_vectors);
    passed &= run_test!(test_exception_to_signal_ignores_unmapped_vectors);
    passed &= run_test!(test_post_records_pending_signal);
    passed &= run_test!(test_post_rejects_out_of_range);
    passed
}
