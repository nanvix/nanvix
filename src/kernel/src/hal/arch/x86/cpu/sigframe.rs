// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Signal Trap-Frame Access
//!
//! Architecture-specific access to the interrupted user context that the kernel-call entry stub
//! (`_do_kcall`) leaves on the top of a thread's kernel stack. The stub runs before any Rust code,
//! so the interrupted user state sits at fixed offsets below the top of the kernel stack: the
//! hardware trap frame (`EIP`, `CS`, `EFLAGS`, user `ESP`, `SS`) followed by the callee-saved
//! registers the stub pushes (`EBP`, `ESI`, `EDI`, `EBX`). These offsets mirror that stub and must
//! change with it.
//!

//==================================================================================================
// Imports
//==================================================================================================

use super::SignalCpuContext;
use ::arch::cpu::ring::PrivilegeLevel;

//==================================================================================================
// Constants
//==================================================================================================

/// Flags installed for handler entry: interrupts enabled, everything else clear.
const HANDLER_ENTRY_FLAGS: u32 = (1 << 9) | (1 << 1);

/// Mask for the requested-privilege-level (RPL) field of a segment selector (its low two bits).
const SELECTOR_RPL_MASK: u32 = 0b11;

//==================================================================================================
// Structures
//==================================================================================================

/// On-stack image of the interrupted user context, as the kernel-call entry stub (`_do_kcall`)
/// leaves it just below the top of the kernel stack (`esp0`).
///
/// Fields are ordered by ascending address: the stub pushes the callee-saved registers last (so
/// `EBX` sits lowest), beneath the hardware trap frame the CPU pushes on the ring 3 -> ring 0
/// transition. Each field therefore sits `size_of - offset_of` bytes *below* `esp0`, which the
/// `OFF_*` constants encode so the stub layout has a single source of truth.
#[repr(C)]
struct TrapFrame {
    /// `EBX` (pushed last by the entry stub; lowest address).
    ebx: u32,
    /// `EDI` (pushed by the entry stub).
    edi: u32,
    /// `ESI` (pushed by the entry stub).
    esi: u32,
    /// `EBP` (pushed by the entry stub).
    ebp: u32,
    /// `EIP` (pushed by the CPU).
    eip: u32,
    /// `CS` (pushed by the CPU).
    cs: u32,
    /// `EFLAGS` (pushed by the CPU).
    eflags: u32,
    /// User `ESP` (pushed by the CPU).
    esp: u32,
    /// `SS` (pushed by the CPU; highest address, just below `esp0`).
    ss: u32,
}

// `TrapFrame` must be 36 bytes long (9 words). This must match the `_do_kcall` entry stub.
::static_assert::assert_eq_size!(TrapFrame, 36);

impl TrapFrame {
    /// Size of the saved frame; `esp0` points just past its last word (`SS`).
    const SIZE: usize = core::mem::size_of::<Self>();

    /// Offset of the saved `EIP` below `esp0`.
    const OFF_EIP: usize = Self::SIZE - core::mem::offset_of!(Self, eip);
    /// Offset of the saved `CS` below `esp0`.
    const OFF_CS: usize = Self::SIZE - core::mem::offset_of!(Self, cs);
    /// Offset of the saved `EFLAGS` below `esp0`.
    const OFF_EFLAGS: usize = Self::SIZE - core::mem::offset_of!(Self, eflags);
    /// Offset of the saved user `ESP` below `esp0`.
    const OFF_ESP: usize = Self::SIZE - core::mem::offset_of!(Self, esp);
    /// Offset of the saved `SS` below `esp0`.
    const OFF_SS: usize = Self::SIZE - core::mem::offset_of!(Self, ss);
    /// Offset of the saved `EBP` below `esp0`.
    const OFF_EBP: usize = Self::SIZE - core::mem::offset_of!(Self, ebp);
    /// Offset of the saved `ESI` below `esp0`.
    const OFF_ESI: usize = Self::SIZE - core::mem::offset_of!(Self, esi);
    /// Offset of the saved `EDI` below `esp0`.
    const OFF_EDI: usize = Self::SIZE - core::mem::offset_of!(Self, edi);
    /// Offset of the saved `EBX` below `esp0`.
    const OFF_EBX: usize = Self::SIZE - core::mem::offset_of!(Self, ebx);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Reads a kernel-stack word at `esp0 - off`.
///
/// # Safety
///
/// `esp0` must be the top of the current thread's kernel stack and the thread must have entered the
/// kernel from user mode, so the trap frame is present.
unsafe fn kstack_read(esp0: usize, off: usize) -> u32 {
    unsafe { ::core::ptr::read_volatile((esp0 - off) as *const u32) }
}

/// Writes a kernel-stack word at `esp0 - off`.
///
/// # Safety
///
/// As for [`kstack_read`]; the write modifies the context the kernel-call return path resumes.
unsafe fn kstack_write(esp0: usize, off: usize, value: u32) {
    unsafe { ::core::ptr::write_volatile((esp0 - off) as *mut u32, value) }
}

/// Returns whether the interrupted trap frame returns to user mode.
///
/// The saved code-segment selector's RPL is ring 3 exactly when the interrupted context resumes in
/// user mode.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn returning_to_user(esp0: usize) -> bool {
    let cs: u32 = unsafe { kstack_read(esp0, TrapFrame::OFF_CS) };
    (cs & SELECTOR_RPL_MASK) == PrivilegeLevel::Ring3 as u32
}

/// Reads the user stack pointer saved in the interrupted trap frame.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn read_user_sp(esp0: usize) -> usize {
    unsafe { kstack_read(esp0, TrapFrame::OFF_ESP) as usize }
}

/// Splits a kernel-call return value into the `EDX:EAX` pair used by the x86 ABI.
pub fn split_kcall_result(result: i64) -> (u32, u32) {
    let bits: u64 = result as u64;
    (bits as u32, (bits >> 32) as u32)
}

/// Joins an `EDX:EAX` return-value pair back into a kernel-call return value.
pub fn join_kcall_result(ax: u32, dx: u32) -> i64 {
    let bits: u64 = (u64::from(dx) << 32) | u64::from(ax);
    bits as i64
}

/// Reads the interrupted user context off the kernel stack.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn read_trap_context(esp0: usize, result: i64) -> SignalCpuContext {
    let (ax, dx): (u32, u32) = split_kcall_result(result);
    unsafe {
        SignalCpuContext {
            ip: kstack_read(esp0, TrapFrame::OFF_EIP),
            sp: kstack_read(esp0, TrapFrame::OFF_ESP),
            flags: kstack_read(esp0, TrapFrame::OFF_EFLAGS),
            ax,
            bx: kstack_read(esp0, TrapFrame::OFF_EBX),
            cx: 0,
            dx,
            si: kstack_read(esp0, TrapFrame::OFF_ESI),
            di: kstack_read(esp0, TrapFrame::OFF_EDI),
            bp: kstack_read(esp0, TrapFrame::OFF_EBP),
            cs: kstack_read(esp0, TrapFrame::OFF_CS),
            ss: kstack_read(esp0, TrapFrame::OFF_SS),
        }
    }
}

/// Redirects the interrupted thread to a handler entry on its new signal-frame stack.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn redirect_to_handler(esp0: usize, handler_ip: usize, frame_top: usize) {
    unsafe {
        kstack_write(esp0, TrapFrame::OFF_EIP, handler_ip as u32);
        kstack_write(esp0, TrapFrame::OFF_ESP, frame_top as u32);
        kstack_write(esp0, TrapFrame::OFF_EFLAGS, HANDLER_ENTRY_FLAGS);
    }
}

/// Restores the interrupted user context from a sanitized frame back onto the kernel stack.
///
/// The general-purpose registers are written to the slots the kernel-call return stub pops, and the
/// instruction pointer, stack pointer, flags, and segment selectors to the hardware trap frame.
///
/// # Safety
///
/// As for [`kstack_read`].
pub unsafe fn restore_trap_context(esp0: usize, cpu: &SignalCpuContext) {
    unsafe {
        kstack_write(esp0, TrapFrame::OFF_EIP, cpu.ip);
        kstack_write(esp0, TrapFrame::OFF_CS, cpu.cs);
        kstack_write(esp0, TrapFrame::OFF_EFLAGS, cpu.flags);
        kstack_write(esp0, TrapFrame::OFF_ESP, cpu.sp);
        kstack_write(esp0, TrapFrame::OFF_SS, cpu.ss);
        kstack_write(esp0, TrapFrame::OFF_EBX, cpu.bx);
        kstack_write(esp0, TrapFrame::OFF_ESI, cpu.si);
        kstack_write(esp0, TrapFrame::OFF_EDI, cpu.di);
        kstack_write(esp0, TrapFrame::OFF_EBP, cpu.bp);
    }
}
