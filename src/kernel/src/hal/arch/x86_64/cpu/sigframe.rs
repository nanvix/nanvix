// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Signal Trap-Frame Access (x86-64)
//!
//! Placeholder trap-frame access for x86-64. Asynchronous signal delivery is not yet wired on this
//! architecture, so these entry points keep the architecture-neutral delivery logic compiling while
//! making delivery inert: [`returning_to_user`] reports that the interrupted context never resumes
//! in user mode, which short-circuits the delivery checkpoint before the remaining placeholders are
//! reached.
//!

//==================================================================================================
// Imports
//==================================================================================================

use super::SignalCpuContext;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns whether the interrupted trap frame returns to user mode.
///
/// Always `false` until x86-64 signal delivery is implemented, which keeps the delivery checkpoint
/// inert.
///
/// # Safety
///
/// Safe to call; the argument is ignored.
pub unsafe fn returning_to_user(_esp0: usize) -> bool {
    false
}

/// Reads the user stack pointer saved in the interrupted trap frame.
///
/// # Safety
///
/// Unreachable until x86-64 signal delivery is implemented; present so the neutral delivery logic
/// compiles.
pub unsafe fn read_user_sp(_esp0: usize) -> usize {
    0
}

/// Joins a kernel-call return value back into its delivered form.
///
/// On x86-64 the accumulator alone carries the result.
pub fn join_kcall_result(ax: u64, _dx: u64) -> i64 {
    ax as i64
}

/// Reads the interrupted user context off the kernel stack.
///
/// # Safety
///
/// As for [`read_user_sp`].
pub unsafe fn read_trap_context(_esp0: usize, _result: i64) -> SignalCpuContext {
    SignalCpuContext::default()
}

/// Redirects the interrupted thread to a handler entry on its new signal-frame stack.
///
/// # Safety
///
/// As for [`read_user_sp`].
pub unsafe fn redirect_to_handler(_esp0: usize, _handler_ip: usize, _frame_top: usize) {}

/// Restores the interrupted user context from a sanitized frame back onto the kernel stack.
///
/// # Safety
///
/// As for [`read_user_sp`].
pub unsafe fn restore_trap_context(_esp0: usize, _cpu: &SignalCpuContext) {}

/// Rewrites a saved user context to transparently restart an interrupted kernel call.
///
/// Inert on x86-64 until asynchronous signal delivery is implemented; present so the
/// architecture-neutral delivery logic compiles.
pub fn prepare_kcall_restart(_cpu: &mut SignalCpuContext, _number: u32, _args: [u32; 4]) {}
