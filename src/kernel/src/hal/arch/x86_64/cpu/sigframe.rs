// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Signal Trap-Frame Access (x86-64)
//!
//! Architecture-specific access to the interrupted user context that the kernel-call entry stub
//! (`_do_kcall`) leaves on the top of a thread's kernel stack. The stub pushes a dummy error code
//! and then saves the full register file in the [`ContextInformation`] layout (the same layout the
//! exception hooks build), so the interrupted user state sits at fixed offsets below the top of the
//! kernel stack (`esp0`). These offsets are derived from the [`ContextInformation`] field offsets
//! and therefore track the entry-stub layout automatically.
//!

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    ContextInformation,
    SignalCpuContext,
};
use ::arch::cpu::ring::PrivilegeLevel;

//==================================================================================================
// Constants
//==================================================================================================

/// Flags installed for handler entry: interrupts enabled (bit 9) and the always-one reserved bit
/// (bit 1), everything else clear.
const HANDLER_ENTRY_FLAGS: u64 = (1 << 9) | (1 << 1);

/// Mask for the requested-privilege-level (RPL) field of a segment selector (its low two bits).
const SELECTOR_RPL_MASK: u64 = 0b11;

/// Total size of the on-stack [`ContextInformation`] trap frame; `esp0` points just past its last
/// word, so the frame base sits this many bytes below `esp0`.
const CONTEXT_SIZE: usize = core::mem::size_of::<ContextInformation>();

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Offset *below* `esp0` of a [`ContextInformation`] field, given its offset *within* the structure.
///
/// The entry stub leaves the context with its base `CONTEXT_SIZE` bytes below `esp0`, so a field at
/// in-structure offset `field` sits `CONTEXT_SIZE - field` bytes below `esp0`.
const fn off(field: u32) -> usize {
    CONTEXT_SIZE - field as usize
}

/// Reads a kernel-stack quad-word at `esp0 - off`.
///
/// # Safety
///
/// `esp0` must be the top of the current thread's kernel stack and the thread must have entered the
/// kernel from user mode, so the trap frame is present.
unsafe fn kstack_read(esp0: usize, off: usize) -> u64 {
    unsafe { ::core::ptr::read_volatile((esp0 - off) as *const u64) }
}

/// Writes a kernel-stack quad-word at `esp0 - off`.
///
/// # Safety
///
/// As for [`kstack_read`]; the write modifies the context the kernel-call return path resumes.
unsafe fn kstack_write(esp0: usize, off: usize, value: u64) {
    unsafe { ::core::ptr::write_volatile((esp0 - off) as *mut u64, value) }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns whether the interrupted trap frame returns to user mode.
///
/// The saved code-segment selector's RPL is ring 3 exactly when the interrupted context resumes in
/// user mode.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn returning_to_user(esp0: usize) -> bool {
    let cs: u64 = unsafe { kstack_read(esp0, off(ContextInformation::CONTEXT_CS)) };
    (cs & SELECTOR_RPL_MASK) == PrivilegeLevel::Ring3 as u64
}

/// Reads the user stack pointer saved in the interrupted trap frame.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn read_user_sp(esp0: usize) -> usize {
    unsafe { kstack_read(esp0, off(ContextInformation::CONTEXT_RSP)) as usize }
}

/// Joins a kernel-call return value back into its delivered form.
///
/// On x86-64 the accumulator alone carries the result, so the high word is ignored.
pub fn join_kcall_result(ax: u64, _dx: u64) -> i64 {
    ax as i64
}

/// Reads the interrupted user context off the kernel stack.
///
/// The accumulator is taken from `result` (the kernel-call return value the interrupted call would
/// have delivered) rather than the saved slot, so it survives the handler.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn read_trap_context(esp0: usize, result: i64) -> SignalCpuContext {
    unsafe {
        SignalCpuContext {
            ip: kstack_read(esp0, off(ContextInformation::CONTEXT_RIP)),
            sp: kstack_read(esp0, off(ContextInformation::CONTEXT_RSP)),
            flags: kstack_read(esp0, off(ContextInformation::CONTEXT_RFLAGS)),
            ax: result as u64,
            bx: kstack_read(esp0, off(ContextInformation::CONTEXT_RBX)),
            cx: kstack_read(esp0, off(ContextInformation::CONTEXT_RCX)),
            dx: kstack_read(esp0, off(ContextInformation::CONTEXT_RDX)),
            si: kstack_read(esp0, off(ContextInformation::CONTEXT_RSI)),
            di: kstack_read(esp0, off(ContextInformation::CONTEXT_RDI)),
            bp: kstack_read(esp0, off(ContextInformation::CONTEXT_RBP)),
            r8: kstack_read(esp0, off(ContextInformation::CONTEXT_R8)),
            r9: kstack_read(esp0, off(ContextInformation::CONTEXT_R9)),
            r10: kstack_read(esp0, off(ContextInformation::CONTEXT_R10)),
            r11: kstack_read(esp0, off(ContextInformation::CONTEXT_R11)),
            r12: kstack_read(esp0, off(ContextInformation::CONTEXT_R12)),
            r13: kstack_read(esp0, off(ContextInformation::CONTEXT_R13)),
            r14: kstack_read(esp0, off(ContextInformation::CONTEXT_R14)),
            r15: kstack_read(esp0, off(ContextInformation::CONTEXT_R15)),
            cs: kstack_read(esp0, off(ContextInformation::CONTEXT_CS)),
            ss: kstack_read(esp0, off(ContextInformation::CONTEXT_SS)),
        }
    }
}

/// Redirects the interrupted thread to a handler entry on its new signal-frame stack.
///
/// The signal number is placed in `RDI`, the first integer-argument register of the System V ABI,
/// so a handler declared as `fn(int)` receives it. For an `SA_SIGINFO` handler the kernel also
/// places the user addresses of the embedded `siginfo` and context images in `RSI` and `RDX`, the
/// second and third integer-argument registers, so a handler declared as
/// `fn(int, *const siginfo_t, *const c_void)` receives them. For a non-`SA_SIGINFO` handler the
/// caller passes `0` for both, which a one-argument handler simply ignores.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn redirect_to_handler(
    esp0: usize,
    handler_ip: usize,
    frame_top: usize,
    _restorer: usize,
    signum: usize,
    info_ptr: usize,
    ctx_ptr: usize,
) {
    unsafe {
        kstack_write(esp0, off(ContextInformation::CONTEXT_RIP), handler_ip as u64);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RSP), frame_top as u64);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RFLAGS), HANDLER_ENTRY_FLAGS);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RDI), signum as u64);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RSI), info_ptr as u64);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RDX), ctx_ptr as u64);
    }
}

/// Restores the interrupted user context from a sanitized frame back onto the kernel stack.
///
/// The full general-purpose register file is written to the saved-context slots that the
/// kernel-call return path restores, along with the instruction pointer, stack pointer, flags, and
/// segment selectors in the hardware trap frame.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn restore_trap_context(esp0: usize, cpu: &SignalCpuContext) {
    unsafe {
        kstack_write(esp0, off(ContextInformation::CONTEXT_RIP), cpu.ip);
        kstack_write(esp0, off(ContextInformation::CONTEXT_CS), cpu.cs);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RFLAGS), cpu.flags);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RSP), cpu.sp);
        kstack_write(esp0, off(ContextInformation::CONTEXT_SS), cpu.ss);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RAX), cpu.ax);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RBX), cpu.bx);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RCX), cpu.cx);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RDX), cpu.dx);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RSI), cpu.si);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RDI), cpu.di);
        kstack_write(esp0, off(ContextInformation::CONTEXT_RBP), cpu.bp);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R8), cpu.r8);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R9), cpu.r9);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R10), cpu.r10);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R11), cpu.r11);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R12), cpu.r12);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R13), cpu.r13);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R14), cpu.r14);
        kstack_write(esp0, off(ContextInformation::CONTEXT_R15), cpu.r15);
    }
}

/// Size in bytes of the kernel-call trap instruction (`int $KCALL_VECTOR`, encoded as `CD ib`),
/// used to rewind a saved instruction pointer back onto the trap so the call re-executes.
const KCALL_TRAP_INSN_SIZE: u64 = 2;

/// Rewrites a saved user context so that, once restored by `sigreturn()`, the interrupted kernel
/// call is transparently restarted (the kernel's analog of Linux's `ERESTARTSYS`).
///
/// The saved instruction pointer is rewound to the kernel-call trap instruction and the original
/// call number and argument registers are reloaded (`RAX` for the number; `RDI`, `RSI`, `RDX`,
/// `R10` for arguments 0..3), so re-executing the trap repeats the call with its initial arguments.
///
/// # Parameters
///
/// - `cpu`: Saved context to rewrite in place.
/// - `number`: Kernel-call number (restored to the accumulator).
/// - `args`: Original kernel-call arguments, in argument-register order.
pub fn prepare_kcall_restart(cpu: &mut SignalCpuContext, number: u32, args: [u32; 4]) {
    cpu.ip = cpu.ip.wrapping_sub(KCALL_TRAP_INSN_SIZE);
    cpu.ax = u64::from(number);
    cpu.di = u64::from(args[0]);
    cpu.si = u64::from(args[1]);
    cpu.dx = u64::from(args[2]);
    cpu.r10 = u64::from(args[3]);
}
