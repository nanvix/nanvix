// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    Address,
    VirtualAddress,
};
use ::alloc::boxed::Box;
use ::arch::cpu::excp::Exception;
use ::sys::pm::{
    SigAction,
    SigSet,
    SIGABRT,
    SIGBUS,
    SIGCHLD,
    SIGCONT,
    SIGFPE,
    SIGILL,
    SIGKILL,
    SIGQUIT,
    SIGSEGV,
    SIGSTOP,
    SIGSYS,
    SIGTRAP,
    SIGTSTP,
    SIGTTIN,
    SIGTTOU,
    SIGURG,
    SIGWINCH,
    SIGXCPU,
    SIGXFSZ,
    SIG_BLOCK,
    SIG_DFL,
    SIG_IGN,
    SIG_MAX,
    SIG_SETMASK,
    SIG_UNBLOCK,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Signals that can never be blocked, caught, or ignored: `SIGKILL` and `SIGSTOP`.
///
/// POSIX requires that any attempt to block these signals be silently ignored rather than reported
/// as an error, so they are cleared from every computed blocked-signal mask.
pub const UNBLOCKABLE: SigSet = (1u64 << (SIGKILL - 1)) | (1u64 << (SIGSTOP - 1));

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Parameters of a user-space signal handler installed via `sigaction()`.
///
// Boxed by [`SignalDisposition::Handler`] so that a disposition is only pointer-sized. Storing this
// payload inline in every one of the 64 disposition slots would push the per-process table past the
// kernel heap's maximum slab size (512 bytes); see [`crate::mm::kheap`].
#[derive(Debug, Clone)]
pub struct SignalHandler {
    /// Entry point of the user-space handler.
    pub entry: VirtualAddress,
    /// Additional signals to block while the handler runs.
    pub mask: u64,
    /// Handler flags.
    pub flags: i32,
    /// Extended handler slot used when `SA_SIGINFO` is set.
    pub sigaction: usize,
}

///
/// # Description
///
/// Disposition of a single signal.
///
#[derive(Debug, Clone)]
pub enum SignalDisposition {
    /// Take the default action for the signal.
    Default,
    /// Ignore the signal.
    Ignore,
    /// Run a user-space handler installed via `sigaction()`.
    Handler(Box<SignalHandler>),
}

///
/// # Description
///
/// Per-process signal control block.
///
// The disposition table is read and written by `sigaction()`; the remaining fields are inert
// plumbing read by later phases of the signals effort.
#[derive(Debug)]
pub struct SignalControl {
    /// Disposition for each of the 64 signals, indexed by `signum - 1`.
    ///
    // Split into two 32-entry halves so that each half is a single heap allocation that stays
    // within the kernel heap's maximum slab size (512 bytes) regardless of pointer width: a
    // 64-bit `SignalDisposition` is 16 bytes, so a single 64-entry table would be 1024 bytes and
    // exceed the largest slab. Two 32-entry halves keep `ProcessState` in its original size class.
    dispositions: [Box<[SignalDisposition; 32]>; 2],
    /// Process-directed pending signals not yet claimed by a thread.
    ///
    /// Set by the signal-posting primitive (`kill()`) and read when delivery is evaluated.
    pending: u64,
    /// Address of the user-space return trampoline (restorer).
    ///
    /// Registered by the `SigRestorer` kernel call and read when an asynchronous signal frame is
    /// built, to set the handler's return address.
    restorer: Option<VirtualAddress>,
}

//==================================================================================================
// Compile-Time Assertions
//==================================================================================================

// Enforce, at build time, that each half of the dispositions table fits the kernel heap's largest
// slab size class (512 bytes; see `crate::mm::kheap`). The kernel heap rejects any allocation whose
// size or alignment exceeds that bound, which would turn `SignalControl::default()` into a runtime
// allocation failure. The table is split into two `Box<[SignalDisposition; 32]>` halves so that each
// allocation stays within the bound even when `SignalDisposition` is 16 bytes wide (64-bit).
::static_assert::assert_eq!(::core::mem::size_of::<[SignalDisposition; 32]>() <= 512);
::static_assert::assert_eq!(::core::mem::align_of::<[SignalDisposition; 32]>() <= 512);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Combines the `current` blocked-signal mask with `set` according to `how`.
///
/// # Parameters
///
/// - `how`: One of [`SIG_BLOCK`], [`SIG_UNBLOCK`], or [`SIG_SETMASK`].
/// - `current`: The current blocked-signal mask.
/// - `set`: The signals to apply per `how`.
///
/// # Returns
///
/// The updated mask, or [`None`] if `how` is not a recognized value.
///
pub(super) fn apply_how(how: i32, current: SigSet, set: SigSet) -> Option<SigSet> {
    match how {
        SIG_BLOCK => Some(current | set),
        SIG_UNBLOCK => Some(current & !set),
        SIG_SETMASK => Some(set),
        _ => None,
    }
}

///
/// # Description
///
/// Computes the next blocked-signal mask, applying `how` and then silently clearing the signals
/// that can never be blocked ([`UNBLOCKABLE`]).
///
/// # Parameters
///
/// - `how`: One of [`SIG_BLOCK`], [`SIG_UNBLOCK`], or [`SIG_SETMASK`].
/// - `current`: The current blocked-signal mask.
/// - `set`: The signals to apply per `how`.
///
/// # Returns
///
/// The updated mask with [`SIGKILL`]/[`SIGSTOP`] cleared, or [`None`] if `how` is invalid.
///
pub fn compute_blocked(how: i32, current: SigSet, set: SigSet) -> Option<SigSet> {
    apply_how(how, current, set).map(|next| next & !UNBLOCKABLE)
}

//==================================================================================================
// Default Actions
//==================================================================================================

///
/// # Description
///
/// The action performed for a signal whose disposition is the default ([`SIG_DFL`]).
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultAction {
    /// Abnormally terminate the process.
    Terminate,
    /// Abnormally terminate the process and emit a diagnostic (a core dump is future work).
    Core,
    /// Ignore (discard) the signal.
    Ignore,
    /// Stop (suspend) the process.
    Stop,
    /// Continue the process if it is stopped.
    Continue,
}

///
/// # Description
///
/// Returns the default action for signal `signum`, as defined by POSIX.
///
/// # Parameters
///
/// - `signum`: The signal number (1-based).
///
/// # Returns
///
/// The [`DefaultAction`] taken when `signum` has the default disposition. Signals without an
/// explicitly assigned default action (unassigned slots and the real-time range) terminate the
/// process, matching the POSIX default for catchable signals.
///
pub fn default_action(signum: usize) -> DefaultAction {
    match signum {
        SIGQUIT | SIGILL | SIGTRAP | SIGABRT | SIGBUS | SIGFPE | SIGSEGV | SIGXCPU | SIGXFSZ
        | SIGSYS => DefaultAction::Core,
        SIGCHLD | SIGURG | SIGWINCH => DefaultAction::Ignore,
        SIGSTOP | SIGTSTP | SIGTTIN | SIGTTOU => DefaultAction::Stop,
        SIGCONT => DefaultAction::Continue,
        // All remaining signals — SIGHUP, SIGINT, SIGKILL, SIGUSR1, SIGUSR2, SIGPIPE, SIGALRM,
        // SIGTERM, SIGVTALRM, SIGPROF, SIGIO, and any unassigned or real-time signal — terminate.
        _ => DefaultAction::Terminate,
    }
}

///
/// # Description
///
/// Outcome of evaluating a posted signal in the kernel's `kill()` primitive.
///
/// The cross-process termination path (`terminate()`) operates on a non-running target and can run
/// entirely inside the process manager. Self-termination, however, must unwind through the calling
/// thread's own `exit()`, which performs a context switch and never returns; that step is therefore
/// deferred to the kernel-call handler, which is not holding a borrow of the process manager.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillOutcome {
    /// The signal was fully handled in-kernel: it was posted and a candidate thread woken, it was
    /// discarded, only the existence check was performed, or the target was terminated via the
    /// cross-process path.
    Done,
    /// The signal's default action terminates the caller itself; the kernel-call handler must
    /// complete the termination by invoking `exit()` on the calling process.
    TerminateSelf,
}

//==================================================================================================
// Synchronous Exception Mapping
//==================================================================================================

///
/// # Description
///
/// Maps a synchronous CPU exception vector to the signal it generates on the faulting thread, as
/// defined by the signals design.
///
/// Only the faults that POSIX surfaces as catchable synchronous signals are mapped; every other
/// vector returns [`None`] so the exception path leaves its handling unchanged (in-kernel
/// resolution, owner forwarding via `evctrl()`, or termination):
///
/// | Vector | Condition | Signal |
/// | --- | --- | --- |
/// | `#DE` | Divide error | [`SIGFPE`] |
/// | `#UD` | Invalid opcode | [`SIGILL`] |
/// | `#GP` | General protection | [`SIGSEGV`] |
/// | `#PF` | Page fault (no valid mapping) | [`SIGSEGV`] |
/// | `#BP`/`#DB` | Breakpoint / debug | [`SIGTRAP`] |
///
/// # Parameters
///
/// - `vector`: The CPU exception vector number.
///
/// # Returns
///
/// The signal number (1-based) generated by the exception, or [`None`] if the vector does not map
/// to a synchronous signal.
///
pub fn exception_to_signal(vector: u32) -> Option<usize> {
    match Exception::try_from_vector(vector as usize)? {
        Exception::DivisionByZero => Some(SIGFPE),
        Exception::InvalidOpcode => Some(SIGILL),
        Exception::GeneralProtectionFault | Exception::PageFault => Some(SIGSEGV),
        Exception::Breakpoint | Exception::Debug => Some(SIGTRAP),
        _ => None,
    }
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SignalDisposition {
    ///
    /// # Description
    ///
    /// Renders this disposition as the [`SigAction`] structure returned through the `oldact`
    /// argument of `sigaction()`.
    ///
    /// # Returns
    ///
    /// The [`SigAction`] that describes this disposition.
    ///
    pub fn to_sigaction(&self) -> SigAction {
        match self {
            SignalDisposition::Default => SigAction {
                sa_handler: SIG_DFL,
                sa_mask: 0,
                sa_flags: 0,
                sa_sigaction: 0,
            },
            SignalDisposition::Ignore => SigAction {
                sa_handler: SIG_IGN,
                sa_mask: 0,
                sa_flags: 0,
                sa_sigaction: 0,
            },
            SignalDisposition::Handler(handler) => SigAction {
                sa_handler: handler.entry.into_raw_value(),
                sa_mask: handler.mask,
                sa_flags: handler.flags,
                sa_sigaction: handler.sigaction,
            },
        }
    }
}

impl SignalControl {
    ///
    /// # Description
    ///
    /// Returns the disposition currently installed for signal `signum`.
    ///
    /// # Parameters
    ///
    /// - `signum`: The signal number (1-based).
    ///
    /// # Returns
    ///
    /// A reference to the disposition for `signum`, or [`None`] if `signum` is out of range.
    ///
    pub fn disposition(&self, signum: usize) -> Option<&SignalDisposition> {
        let idx: usize = signum.wrapping_sub(1);
        self.dispositions.get(idx / 32)?.get(idx % 32)
    }

    ///
    /// # Description
    ///
    /// Installs `disposition` for signal `signum`, returning the disposition it replaced.
    ///
    /// # Parameters
    ///
    /// - `signum`: The signal number (1-based).
    /// - `disposition`: The disposition to install.
    ///
    /// # Returns
    ///
    /// The previous disposition for `signum`, or [`None`] if `signum` is out of range (in which
    /// case nothing is installed).
    ///
    pub fn set_disposition(
        &mut self,
        signum: usize,
        disposition: SignalDisposition,
    ) -> Option<SignalDisposition> {
        let idx: usize = signum.wrapping_sub(1);
        let slot: &mut SignalDisposition =
            self.dispositions.get_mut(idx / 32)?.get_mut(idx % 32)?;
        Some(::core::mem::replace(slot, disposition))
    }

    ///
    /// # Description
    ///
    /// Adds `signum` to the process-directed pending set.
    ///
    /// # Parameters
    ///
    /// - `signum`: The signal number (1-based).
    ///
    /// # Returns
    ///
    /// `true` if the signal was newly posted (it was not already pending), or `false` if `signum`
    /// is out of range or was already pending.
    ///
    pub fn post(&mut self, signum: usize) -> bool {
        if signum == 0 || signum > SIG_MAX {
            return false;
        }
        let bit: u64 = 1u64 << (signum - 1);
        let newly: bool = (self.pending & bit) == 0;
        self.pending |= bit;
        newly
    }

    ///
    /// # Description
    ///
    /// Returns the process-directed pending set.
    ///
    /// # Returns
    ///
    /// The set of signals pending against the process but not yet claimed by a thread.
    ///
    // Exercised by the in-kernel unit tests and read by a later phase of the signals effort
    // (`sigpending()` and asynchronous delivery).
    #[allow(dead_code)]
    pub fn pending(&self) -> u64 {
        self.pending
    }

    ///
    /// # Description
    ///
    /// Removes `signum` from the process-directed pending set.
    ///
    /// # Parameters
    ///
    /// - `signum`: The signal number (1-based).
    ///
    pub fn clear_pending(&mut self, signum: usize) {
        if signum != 0 && signum <= SIG_MAX {
            self.pending &= !(1u64 << (signum - 1));
        }
    }

    ///
    /// # Description
    ///
    /// Returns the address of the user-space signal-return trampoline (restorer), if registered.
    ///
    /// # Returns
    ///
    /// The restorer address, or [`None`] if the process has not registered one.
    ///
    pub fn restorer(&self) -> Option<VirtualAddress> {
        self.restorer
    }

    ///
    /// # Description
    ///
    /// Registers the address of the user-space signal-return trampoline (restorer).
    ///
    /// The restorer is re-resolved from the freshly loaded image after `execv()`, because caught
    /// dispositions are reset to the default on exec.
    ///
    /// # Parameters
    ///
    /// - `restorer`: The restorer address, or [`None`] to clear it.
    ///
    pub fn set_restorer(&mut self, restorer: Option<VirtualAddress>) {
        self.restorer = restorer;
    }

    ///
    /// # Description
    ///
    /// Produces the signal control block a forked child inherits from this (the parent's) one.
    ///
    /// The signal dispositions are copied verbatim, as POSIX requires, and the restorer is inherited
    /// because the child shares the parent's address space (the trampoline lives at the same
    /// address). The pending set is *not* inherited: a freshly forked child starts with no pending
    /// signals.
    ///
    /// # Returns
    ///
    /// A [`SignalControl`] carrying the inherited dispositions and restorer with an empty pending
    /// set.
    ///
    pub fn inherited_for_fork(&self) -> Self {
        Self {
            dispositions: self.dispositions.clone(),
            pending: 0,
            restorer: self.restorer,
        }
    }

    ///
    /// # Description
    ///
    /// Resets the signal control block for an `execv()` image replacement.
    ///
    /// Caught dispositions point at handler code in the outgoing image, so they are reset to the
    /// default; [`SIG_IGN`] and [`SIG_DFL`] dispositions are preserved, as POSIX requires. The
    /// pending set is cleared and the restorer is dropped — the freshly loaded image re-registers
    /// its own restorer at startup.
    ///
    pub fn reset_for_exec(&mut self) {
        for half in self.dispositions.iter_mut() {
            for slot in half.iter_mut() {
                if matches!(slot, SignalDisposition::Handler(_)) {
                    *slot = SignalDisposition::Default;
                }
            }
        }
        self.pending = 0;
        self.restorer = None;
    }
}

impl Default for SignalControl {
    fn default() -> Self {
        Self {
            dispositions: [
                Box::new(::core::array::from_fn(|_idx| SignalDisposition::Default)),
                Box::new(::core::array::from_fn(|_idx| SignalDisposition::Default)),
            ],
            pending: 0,
            restorer: None,
        }
    }
}
