// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Types
//==================================================================================================

///
/// # Description
///
/// A signal set.
///
/// A signal set is a 64-bit bitmask in which bit `n - 1` represents signal `n` (signal numbers run
/// from `1` to [`SIG_MAX`]). This matches the `sigset_t` convention used by the user-space
/// `<signal.h>` shims, so a set crosses the kernel-call boundary unchanged.
///
pub type SigSet = u64;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum supported signal number. Signal numbers are `1..=SIG_MAX`.
pub const SIG_MAX: usize = 64;

/// `sa_handler` sentinel selecting the signal's default action.
pub const SIG_DFL: usize = 0;

/// `sa_handler` sentinel selecting that the signal be ignored.
pub const SIG_IGN: usize = 1;

/// Signal number of `SIGKILL`, which can never be caught, blocked, or ignored.
pub const SIGKILL: usize = 9;

/// Signal number of `SIGSTOP`, which can never be caught, blocked, or ignored.
pub const SIGSTOP: usize = 19;

/// `how` argument to `sigprocmask()`: add the signals in `set` to the blocked mask.
pub const SIG_BLOCK: i32 = 0;

/// `how` argument to `sigprocmask()`: remove the signals in `set` from the blocked mask.
pub const SIG_UNBLOCK: i32 = 1;

/// `how` argument to `sigprocmask()`: replace the blocked mask with `set`.
pub const SIG_SETMASK: i32 = 2;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Signal action structure exchanged with the `sigaction()` kernel call.
///
/// The field layout mirrors the C `struct sigaction` declared in `<signal.h>` and the
/// `sigaction_t` type used by the user-space signal shims, so the structure is copied across the
/// kernel-call boundary without any translation. The handler fields are pointer-sized integers
/// rather than function pointers because `<signal.h>` uses non-function sentinels ([`SIG_DFL`] and
/// [`SIG_IGN`]); the kernel never calls through these values, it only stores and returns them.
///
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SigAction {
    /// Signal handler: [`SIG_DFL`], [`SIG_IGN`], or the address of a user-space handler.
    pub sa_handler: usize,
    /// Additional signals to block while the handler runs.
    pub sa_mask: SigSet,
    /// Handler flags (`SA_SIGINFO`, `SA_RESTART`, `SA_NODEFER`, `SA_RESETHAND`, ...).
    pub sa_flags: i32,
    /// Extended handler slot, used by the C ABI when `SA_SIGINFO` is set. The kernel preserves
    /// this slot but does not interpret it in this phase.
    pub sa_sigaction: usize,
}
