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

// Standard signal numbers. These mirror the values declared in `<signal.h>` and back the kernel's
// default-action table. `SIGKILL` (9) and `SIGSTOP` (19) are defined above because they are also
// referenced as the uncatchable signals.

/// Hangup.
pub const SIGHUP: usize = 1;
/// Terminal interrupt.
pub const SIGINT: usize = 2;
/// Terminal quit.
pub const SIGQUIT: usize = 3;
/// Illegal instruction.
pub const SIGILL: usize = 4;
/// Trace/breakpoint trap.
pub const SIGTRAP: usize = 5;
/// Process abort.
pub const SIGABRT: usize = 6;
/// Bus error.
pub const SIGBUS: usize = 7;
/// Erroneous arithmetic operation.
pub const SIGFPE: usize = 8;
/// User-defined signal 1.
pub const SIGUSR1: usize = 10;
/// Invalid memory reference.
pub const SIGSEGV: usize = 11;
/// User-defined signal 2.
pub const SIGUSR2: usize = 12;
/// Write on a pipe with no reader.
pub const SIGPIPE: usize = 13;
/// Alarm clock.
pub const SIGALRM: usize = 14;
/// Termination request.
pub const SIGTERM: usize = 15;
/// Child process stopped or terminated.
pub const SIGCHLD: usize = 17;
/// Continue if stopped.
pub const SIGCONT: usize = 18;
/// Terminal stop.
pub const SIGTSTP: usize = 20;
/// Background process attempting read.
pub const SIGTTIN: usize = 21;
/// Background process attempting write.
pub const SIGTTOU: usize = 22;
/// Urgent condition on socket.
pub const SIGURG: usize = 23;
/// CPU time limit exceeded.
pub const SIGXCPU: usize = 24;
/// File size limit exceeded.
pub const SIGXFSZ: usize = 25;
/// Virtual timer expired.
pub const SIGVTALRM: usize = 26;
/// Profiling timer expired.
pub const SIGPROF: usize = 27;
/// Window size change.
pub const SIGWINCH: usize = 28;
/// I/O now possible.
pub const SIGIO: usize = 29;
/// Bad system call.
pub const SIGSYS: usize = 31;

/// `how` argument to `sigprocmask()`: add the signals in `set` to the blocked mask.
pub const SIG_BLOCK: i32 = 0;

/// `how` argument to `sigprocmask()`: remove the signals in `set` from the blocked mask.
pub const SIG_UNBLOCK: i32 = 1;

/// `how` argument to `sigprocmask()`: replace the blocked mask with `set`.
pub const SIG_SETMASK: i32 = 2;

// `sa_flags` bits interpreted by the signal-delivery machinery. The values mirror those declared in
// `<signal.h>` so a `struct sigaction` crosses the kernel-call boundary unchanged.

/// `sa_flags` bit selecting the extended, three-argument handler (`sa_sigaction`).
pub const SA_SIGINFO: i32 = 0x0000_0004;

/// `sa_flags` bit requesting that interrupted, restartable kernel calls be resumed.
pub const SA_RESTART: i32 = 0x1000_0000;

/// `sa_flags` bit requesting that the delivered signal not be blocked while its handler runs.
pub const SA_NODEFER: i32 = 0x4000_0000;

/// `sa_flags` bit requesting that the disposition be reset to [`SIG_DFL`] on delivery.
///
/// This is the `0x8000_0000` bit of `<signal.h>`; because `sa_flags` is a signed 32-bit field it
/// is the sign bit, i.e. [`i32::MIN`]. Flag tests use a bitwise `AND`, which is sign-agnostic.
pub const SA_RESETHAND: i32 = i32::MIN;

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
    /// Extended handler slot, used by the C ABI when `SA_SIGINFO` is set. The kernel uses this
    /// slot as the handler entry when [`SA_SIGINFO`] is present.
    pub sa_sigaction: usize,
}
