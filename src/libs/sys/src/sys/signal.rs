// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Hangup.
pub const SIGHUP: i32 = 1;
/// Interrupt (Ctrl-C).
pub const SIGINT: i32 = 2;
/// Quit (Ctrl-\).
pub const SIGQUIT: i32 = 3;
/// Illegal instruction.
pub const SIGILL: i32 = 4;
/// Trace/breakpoint trap.
pub const SIGTRAP: i32 = 5;
/// Abort.
pub const SIGABRT: i32 = 6;
/// Bus error.
pub const SIGBUS: i32 = 7;
/// Floating-point exception.
pub const SIGFPE: i32 = 8;
/// Kill (cannot be caught or ignored).
pub const SIGKILL: i32 = 9;
/// User-defined signal 1.
pub const SIGUSR1: i32 = 10;
/// Segmentation fault.
pub const SIGSEGV: i32 = 11;
/// User-defined signal 2.
pub const SIGUSR2: i32 = 12;
/// Broken pipe.
pub const SIGPIPE: i32 = 13;
/// Alarm clock.
pub const SIGALRM: i32 = 14;
/// Termination.
pub const SIGTERM: i32 = 15;

/// Maximum number of signals supported.
pub const NSIG: usize = 32;

/// Default signal disposition (terminate process).
pub const SIG_DFL: usize = 0;
/// Ignore signal.
pub const SIG_IGN: usize = 1;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Signal disposition for a single signal number. Stored per-process in the kernel.
///
#[derive(Debug, Clone, Copy)]
pub struct SignalAction {
    /// Handler address: [`SIG_DFL`] (0), [`SIG_IGN`] (1), or a user-space function pointer.
    handler: usize,
    /// Bitmask of signals to block while this handler is executing.
    mask: u64,
    /// Signal action flags.
    flags: u32,
}

impl SignalAction {
    /// Creates a new signal action with the given handler, mask, and flags.
    pub fn new(handler: usize, mask: u64, flags: u32) -> Self {
        Self { handler, mask, flags }
    }

    /// Returns the handler address.
    pub fn handler(&self) -> usize {
        self.handler
    }

    /// Returns the signal mask.
    pub fn mask(&self) -> u64 {
        self.mask
    }

    /// Returns the flags.
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// Returns `true` if this is the default disposition.
    pub fn is_default(&self) -> bool {
        self.handler == SIG_DFL
    }

    /// Returns `true` if this signal is set to be ignored.
    pub fn is_ignored(&self) -> bool {
        self.handler == SIG_IGN
    }
}

impl Default for SignalAction {
    fn default() -> Self {
        Self {
            handler: SIG_DFL,
            mask: 0,
            flags: 0,
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Maps an x86/x86_64 CPU exception number to a POSIX signal number.
///
/// # Parameters
///
/// - `exception_num`: The CPU exception number (0–31).
///
/// # Returns
///
/// The corresponding POSIX signal number, or `None` if no mapping exists.
///
pub fn exception_to_signal(exception_num: u32) -> Option<i32> {
    match exception_num {
        0 => Some(SIGFPE),   // Divide-by-zero.
        1 => Some(SIGTRAP),  // Debug.
        3 => Some(SIGTRAP),  // Breakpoint.
        4 => Some(SIGSEGV),  // Overflow.
        5 => Some(SIGSEGV),  // Bound range exceeded.
        6 => Some(SIGILL),   // Invalid opcode.
        11 => Some(SIGSEGV), // Segment not present.
        12 => Some(SIGSEGV), // Stack-segment fault.
        13 => Some(SIGSEGV), // General protection fault.
        14 => Some(SIGSEGV), // Page fault.
        16 => Some(SIGFPE),  // x87 floating-point exception.
        19 => Some(SIGFPE),  // SIMD floating-point exception.
        _ => None,
    }
}

///
/// # Description
///
/// Checks whether a signal number is valid.
///
/// # Parameters
///
/// - `signum`: Signal number to check.
///
/// # Returns
///
/// `true` if the signal number is in the range `[1, NSIG)`.
///
pub fn is_valid_signal(signum: i32) -> bool {
    signum >= 1 && (signum as usize) < NSIG
}
