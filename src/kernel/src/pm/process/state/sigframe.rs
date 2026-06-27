// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Signal Frame
//!
//! Architecture-aware construction and validation of the signal frame that the kernel writes onto
//! a target thread's user stack when an asynchronous signal is delivered to a caught handler.
//!
//! The logic in this module is deliberately self-contained: it operates on plain values and a
//! caller-supplied snapshot of the interrupted CPU context, never on live kernel state. This keeps
//! the frame layout and its security-critical validation independently unit-testable (see the
//! `sigframe_test` sibling module), while the process manager owns the surrounding glue that reads
//! the interrupted context off the kernel stack, copies the frame to user space, and rewrites the
//! return path.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::mem::gdt::SegmentSelector;

pub(crate) use crate::hal::cpu::align_down_residue;
pub use crate::hal::{
    arch::SignalCpuContext,
    cpu::{
        build_frame,
        ctx_offset,
        siginfo_offset,
        FrameLayout,
        SigFrame,
        SigFrameError,
        FPU_AREA_SIZE,
        SIGFRAME_MAGIC,
    },
};

//==================================================================================================
// Types
//==================================================================================================

/// Native machine word: the width of a general-purpose register on the target architecture.
#[cfg(target_arch = "x86")]
type Word = u32;
#[cfg(target_arch = "x86_64")]
type Word = u64;

//==================================================================================================
// Constants
//==================================================================================================

/// Number of bytes occupied by the handler's return address on the user stack.
#[cfg(target_arch = "x86")]
pub const RETADDR_SIZE: usize = 4;
#[cfg(target_arch = "x86_64")]
pub const RETADDR_SIZE: usize = 8;

/// Number of bytes of handler arguments passed on the user stack.
///
/// On 32-bit x86 the cdecl ABI passes `signum`, `*info`, and `*ctx` on the stack (three words). On
/// x86-64 the System V ABI passes them in registers, so no stack space is reserved.
#[cfg(target_arch = "x86")]
pub const ARGS_STACK_SIZE: usize = 12;
#[cfg(target_arch = "x86_64")]
pub const ARGS_STACK_SIZE: usize = 0;

/// Required alignment of the user stack at the point of the simulated call into the handler.
const STACK_ALIGN: usize = 16;

/// Residue of `frame_top` modulo [`STACK_ALIGN`] that satisfies the platform ABI.
///
/// The ABI fixes the stack alignment at the call site; after the simulated call pushes the return
/// address, the handler entry sees `sp` congruent to this value modulo [`STACK_ALIGN`].
#[cfg(target_arch = "x86")]
const FRAME_ALIGN_RESIDUE: usize = 12;
#[cfg(target_arch = "x86_64")]
const FRAME_ALIGN_RESIDUE: usize = 8;

/// `EFLAGS`/`RFLAGS` interrupt-enable bit.
const FLAGS_IF: Word = 1 << 9;
/// `EFLAGS`/`RFLAGS` always-one reserved bit (bit 1).
const FLAGS_RESERVED1: Word = 1 << 1;
/// Bits of `EFLAGS`/`RFLAGS` that a returning frame is allowed to set.
///
/// This permits the arithmetic and direction flags — `CF` (0), `PF` (2), `AF` (4), `ZF` (6),
/// `SF` (7), `DF` (10), `OF` (11) — and the `AC` (18) and `ID` (21) user bits, while masking out
/// the trap flag (8), the interrupt flag (9, forced on separately), the I/O privilege level
/// (12-13), and the nested-task flag (14) so a forged frame cannot single-step the kernel, raise
/// its I/O privilege, or resume in an unexpected mode.
const FLAGS_SAFE_MASK: Word = 0x0024_0CD5;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the placement of a signal frame on a user stack that grows down from `user_sp`.
///
/// # Parameters
///
/// - `user_sp`: The interrupted thread's user stack pointer.
///
/// # Returns
///
/// The [`FrameLayout`], or [`None`] if the stack pointer is too low to hold the frame.
///
pub fn frame_layout(user_sp: usize) -> Option<FrameLayout> {
    let total: usize = RETADDR_SIZE + ARGS_STACK_SIZE + core::mem::size_of::<SigFrame>();
    let raw: usize = user_sp.checked_sub(total)?;
    let frame_top: usize = align_down_residue(raw, STACK_ALIGN, FRAME_ALIGN_RESIDUE);
    Some(FrameLayout {
        frame_top,
        save_area_base: frame_top + RETADDR_SIZE + ARGS_STACK_SIZE,
    })
}

///
/// # Description
///
/// Offset, from the stack pointer observed at the `sigreturn()` trap, to the start of the
/// [`SigFrame`] save area.
///
/// The restorer trampoline issues `sigreturn()` without adjusting the stack pointer, so the trap is
/// taken with the stack pointer just above the handler's return address (past the popped return
/// address). The save area sits immediately after the on-stack argument words.
///
pub const fn save_area_offset_from_sigreturn_sp() -> usize {
    ARGS_STACK_SIZE
}

///
/// # Description
///
/// Computes the blocked-signal mask in effect while a handler runs.
///
/// The handler's additional mask (`sa_mask`) is unioned into the current mask and, unless
/// `nodefer` is set, the delivered signal is also blocked so the handler is not re-entered by its
/// own signal. The unblockable signals are cleared by the caller's mask arithmetic, not here.
///
/// # Parameters
///
/// - `current`: The thread's current blocked mask.
/// - `sa_mask`: The handler's additional mask.
/// - `signum`: The signal being delivered (1-based).
/// - `nodefer`: Whether `SA_NODEFER` is set.
///
/// # Returns
///
/// The blocked mask to install for the duration of the handler.
///
pub fn next_blocked(current: u64, sa_mask: u64, signum: usize, nodefer: bool) -> u64 {
    let mut next: u64 = current | sa_mask;
    if !nodefer {
        next |= 1u64 << (signum - 1);
    }
    next
}

///
/// # Description
///
/// Sanitizes a flags register value read back from an untrusted frame.
///
/// # Parameters
///
/// - `flags`: The raw flags value from the frame.
///
/// # Returns
///
/// A flags value that keeps only the user-settable bits, forces interrupts enabled, and sets the
/// reserved always-one bit.
///
fn sanitize_flags(flags: Word) -> Word {
    (flags & FLAGS_SAFE_MASK) | FLAGS_IF | FLAGS_RESERVED1
}

///
/// # Description
///
/// Validates and sanitizes a signal frame presented to `sigreturn()`, returning the CPU context to
/// resume.
///
/// The returned context has its segment selectors forced to the user code and data selectors and
/// its flags reduced to safe user values, so a forged frame cannot resume in kernel mode, raise the
/// I/O privilege level, or single-step the kernel.
///
/// # Parameters
///
/// - `frame`: The frame copied in from user space.
///
/// # Returns
///
/// On success, the sanitized [`SignalCpuContext`] to restore. On failure, a [`SigFrameError`].
///
pub fn validate_and_restore(frame: &SigFrame) -> Result<SignalCpuContext, SigFrameError> {
    if frame.magic != SIGFRAME_MAGIC {
        return Err(SigFrameError::BadMagic);
    }

    let mut cpu: SignalCpuContext = frame.cpu;

    // Force the segment selectors to the user selectors regardless of what the frame claims, and
    // reduce the flags to safe user-mode values, so a forged frame cannot resume in kernel mode or
    // single-step the kernel.
    cpu.cs = SegmentSelector::UserCode as Word;
    cpu.ss = SegmentSelector::UserData as Word;
    cpu.flags = sanitize_flags(frame.cpu.flags);

    Ok(cpu)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(feature = "test")]
#[path = "sigframe_test.rs"]
mod sigframe_test;

#[cfg(feature = "test")]
pub(super) use sigframe_test::test;
