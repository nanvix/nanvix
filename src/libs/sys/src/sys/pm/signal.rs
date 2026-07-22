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
pub type SigSet = ::sysapi::signal::sigset_t;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum supported signal number. Signal numbers are `1..=SIG_MAX`.
pub const SIG_MAX: usize = ::sysapi::signal::SIG_MAX as usize;

pub use ::sysapi::signal::{
    SA_NODEFER,
    SA_RESETHAND,
    SA_RESTART,
    SA_SIGINFO,
    SIG_BLOCK,
    SIG_DFL,
    SIG_IGN,
    SIG_SETMASK,
    SIG_UNBLOCK,
};

/// Signal number of `SIGKILL`, which can never be caught, blocked, or ignored.
pub const SIGKILL: usize = ::sysapi::signal::SIGKILL as usize;

/// Signal number of `SIGSTOP`, which can never be caught, blocked, or ignored.
pub const SIGSTOP: usize = ::sysapi::signal::SIGSTOP as usize;

// Standard signal numbers. These mirror the values declared in `<signal.h>` and back the kernel's
// default-action table. `SIGKILL` (9) and `SIGSTOP` (19) are defined above because they are also
// referenced as the uncatchable signals.

/// Hangup.
pub const SIGHUP: usize = ::sysapi::signal::SIGHUP as usize;
/// Terminal interrupt.
pub const SIGINT: usize = ::sysapi::signal::SIGINT as usize;
/// Terminal quit.
pub const SIGQUIT: usize = ::sysapi::signal::SIGQUIT as usize;
/// Illegal instruction.
pub const SIGILL: usize = ::sysapi::signal::SIGILL as usize;
/// Trace/breakpoint trap.
pub const SIGTRAP: usize = ::sysapi::signal::SIGTRAP as usize;
/// Process abort.
pub const SIGABRT: usize = ::sysapi::signal::SIGABRT as usize;
/// Bus error.
pub const SIGBUS: usize = ::sysapi::signal::SIGBUS as usize;
/// Erroneous arithmetic operation.
pub const SIGFPE: usize = ::sysapi::signal::SIGFPE as usize;
/// User-defined signal 1.
pub const SIGUSR1: usize = ::sysapi::signal::SIGUSR1 as usize;
/// Invalid memory reference.
pub const SIGSEGV: usize = ::sysapi::signal::SIGSEGV as usize;
/// User-defined signal 2.
pub const SIGUSR2: usize = ::sysapi::signal::SIGUSR2 as usize;
/// Write on a pipe with no reader.
pub const SIGPIPE: usize = ::sysapi::signal::SIGPIPE as usize;
/// Alarm clock.
pub const SIGALRM: usize = ::sysapi::signal::SIGALRM as usize;
/// Termination request.
pub const SIGTERM: usize = ::sysapi::signal::SIGTERM as usize;
/// Child process stopped or terminated.
pub const SIGCHLD: usize = ::sysapi::signal::SIGCHLD as usize;
/// Continue if stopped.
pub const SIGCONT: usize = ::sysapi::signal::SIGCONT as usize;
/// Terminal stop.
pub const SIGTSTP: usize = ::sysapi::signal::SIGTSTP as usize;
/// Background process attempting read.
pub const SIGTTIN: usize = ::sysapi::signal::SIGTTIN as usize;
/// Background process attempting write.
pub const SIGTTOU: usize = ::sysapi::signal::SIGTTOU as usize;
/// Urgent condition on socket.
pub const SIGURG: usize = ::sysapi::signal::SIGURG as usize;
/// CPU time limit exceeded.
pub const SIGXCPU: usize = ::sysapi::signal::SIGXCPU as usize;
/// File size limit exceeded.
pub const SIGXFSZ: usize = ::sysapi::signal::SIGXFSZ as usize;
/// Virtual timer expired.
pub const SIGVTALRM: usize = ::sysapi::signal::SIGVTALRM as usize;
/// Profiling timer expired.
pub const SIGPROF: usize = ::sysapi::signal::SIGPROF as usize;
/// Window size change.
pub const SIGWINCH: usize = ::sysapi::signal::SIGWINCH as usize;
/// I/O now possible.
pub const SIGIO: usize = ::sysapi::signal::SIGIO as usize;
/// Bad system call.
pub const SIGSYS: usize = ::sysapi::signal::SIGSYS as usize;

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
