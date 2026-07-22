// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Do not block when no child has changed state.
pub const WNOHANG: c_int = 1;

/// Report children that have stopped (job control).
pub const WUNTRACED: c_int = 2;

/// Report children that have continued (job control).
pub const WCONTINUED: c_int = 8;

/// Status bit indicating that the child produced a core dump.
pub const WCOREFLAG: c_int = 0x80;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Evaluates to `true` if the child terminated normally, that is, by calling `exit()` or by
/// returning from `main()`.
///
/// # Parameters
///
/// - `status`: Status value returned by `wait()`/`waitpid()`.
///
/// # Returns
///
/// `true` if the child terminated normally, `false` otherwise.
///
pub const fn wifexited(status: c_int) -> bool {
    (status & 0x7f) == 0
}

///
/// # Description
///
/// Returns the exit status of a child that terminated normally. This should only be used if
/// [`wifexited()`] returned `true`.
///
/// # Parameters
///
/// - `status`: Status value returned by `wait()`/`waitpid()`.
///
/// # Returns
///
/// The low-order 8 bits of the child's exit status.
///
pub const fn wexitstatus(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

///
/// # Description
///
/// Evaluates to `true` if the child terminated due to the receipt of a signal.
///
/// # Parameters
///
/// - `status`: Status value returned by `wait()`/`waitpid()`.
///
/// # Returns
///
/// `true` if the child terminated due to a signal, `false` otherwise.
///
pub const fn wifsignaled(status: c_int) -> bool {
    ((((status & 0x7f) + 1) as i8) >> 1) > 0
}

///
/// # Description
///
/// Returns the number of the signal that caused a child to terminate. This should only be used if
/// [`wifsignaled()`] returned `true`.
///
/// # Parameters
///
/// - `status`: Status value returned by `wait()`/`waitpid()`.
///
/// # Returns
///
/// The signal number that caused the child to terminate.
///
pub const fn wtermsig(status: c_int) -> c_int {
    status & 0x7f
}
