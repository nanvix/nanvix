// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    arch,
    sync::atomic::{
        AtomicU32,
        Ordering,
    },
};

//==================================================================================================
// Kernel Calls
//==================================================================================================

/// Issues a kernel call with no arguments.
///
/// # Safety
///
/// This function is unsafe because it enters the kernel through an `svc` exception.
#[inline(never)]
pub unsafe fn kcall0(kcall_nr: u32) -> i64 {
    let ret: i64;
    arch::asm!(
        "svc {kcall_vector}",
        kcall_vector = const crate::number::KCALL_VECTOR,
        in("x8") kcall_nr as u64,
        lateout("x0") ret,
        options(nostack)
    );
    ret
}

/// Issues a kernel call with one argument.
///
/// # Safety
///
/// This function is unsafe because it enters the kernel through an `svc` exception.
#[inline(never)]
pub unsafe fn kcall1(kcall_nr: u32, arg0: u32) -> i64 {
    let ret: i64;
    arch::asm!(
        "svc {kcall_vector}",
        kcall_vector = const crate::number::KCALL_VECTOR,
        in("x8") kcall_nr as u64,
        inlateout("x0") arg0 as u64 => ret,
        options(nostack)
    );
    ret
}

/// Issues a kernel call with two arguments.
///
/// # Safety
///
/// This function is unsafe because it enters the kernel through an `svc` exception.
#[inline(never)]
pub unsafe fn kcall2(kcall_nr: u32, arg0: u32, arg1: u32) -> i64 {
    let ret: i64;
    arch::asm!(
        "svc {kcall_vector}",
        kcall_vector = const crate::number::KCALL_VECTOR,
        in("x8") kcall_nr as u64,
        inlateout("x0") arg0 as u64 => ret,
        in("x1") arg1 as u64,
        options(nostack)
    );
    ret
}

/// Issues a kernel call with three arguments.
///
/// # Safety
///
/// This function is unsafe because it enters the kernel through an `svc` exception.
#[inline(never)]
pub unsafe fn kcall3(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32) -> i64 {
    let ret: i64;
    arch::asm!(
        "svc {kcall_vector}",
        kcall_vector = const crate::number::KCALL_VECTOR,
        in("x8") kcall_nr as u64,
        inlateout("x0") arg0 as u64 => ret,
        in("x1") arg1 as u64,
        in("x2") arg2 as u64,
        options(nostack)
    );
    ret
}

/// Issues a kernel call with four arguments.
///
/// # Safety
///
/// This function is unsafe because it enters the kernel through an `svc` exception.
#[inline(never)]
pub unsafe fn kcall4(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i64 {
    let ret: i64;
    arch::asm!(
        "svc {kcall_vector}",
        kcall_vector = const crate::number::KCALL_VECTOR,
        in("x8") kcall_nr as u64,
        inlateout("x0") arg0 as u64 => ret,
        in("x1") arg1 as u64,
        in("x2") arg2 as u64,
        in("x3") arg3 as u64,
        options(nostack)
    );
    ret
}

//==================================================================================================
// Thread Data Area Helpers
//==================================================================================================

#[inline(always)]
unsafe fn thread_data_area() -> usize {
    let base: usize;
    arch::asm!(
        "mrs {base}, tpidr_el0",
        base = out(reg) base,
        options(nomem, nostack, preserves_flags)
    );
    base
}

/// Reads a `u32` from the calling thread's Thread Data Area.
///
/// # Safety
///
/// `TPIDR_EL0` must identify a valid TDA and `offset` must address an aligned `u32` within it.
#[inline(always)]
pub unsafe fn read_tda_u32(offset: u32) -> u32 {
    let ptr: *const u32 = (thread_data_area() + offset as usize) as *const u32;
    core::ptr::read_volatile(ptr)
}

/// Writes a `u32` to the calling thread's Thread Data Area.
///
/// # Safety
///
/// `TPIDR_EL0` must identify a valid TDA and `offset` must address an aligned `u32` within it.
#[inline(always)]
pub unsafe fn write_tda_u32(offset: u32, val: u32) {
    let ptr: *mut u32 = (thread_data_area() + offset as usize) as *mut u32;
    core::ptr::write_volatile(ptr, val);
}

/// Atomically replaces a `u32` in the calling thread's Thread Data Area.
///
/// # Safety
///
/// `TPIDR_EL0` must identify a valid TDA and `offset` must address an aligned `u32` within it.
#[inline(always)]
pub unsafe fn swap_tda_u32(offset: u32, val: u32) -> u32 {
    let ptr: *const AtomicU32 = (thread_data_area() + offset as usize) as *const AtomicU32;
    (*ptr).swap(val, Ordering::SeqCst)
}

//==================================================================================================
// Fork Support
//==================================================================================================

/// Value returned by [`fork_save_context()`] when execution resumes in the child.
pub const FORK_CHILD_SENTINEL: i32 = 1;

/// AAPCS64 callee-saved context used to resume the child after `fork()`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ForkContext {
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    x29: u64,
    sp: u64,
    pc: u64,
}

impl ForkContext {
    const OFFSET_X19: usize = ::core::mem::offset_of!(Self, x19);
    const OFFSET_X21: usize = ::core::mem::offset_of!(Self, x21);
    const OFFSET_X23: usize = ::core::mem::offset_of!(Self, x23);
    const OFFSET_X25: usize = ::core::mem::offset_of!(Self, x25);
    const OFFSET_X27: usize = ::core::mem::offset_of!(Self, x27);
    const OFFSET_X29: usize = ::core::mem::offset_of!(Self, x29);
    const OFFSET_SP: usize = ::core::mem::offset_of!(Self, sp);
    const OFFSET_PC: usize = ::core::mem::offset_of!(Self, pc);
}

::static_assert::assert_eq_size!(ForkContext, 104);
::static_assert::assert_eq_align!(ForkContext, 8);
::static_assert::assert_eq!(ForkContext::OFFSET_X19 == 0);
::static_assert::assert_eq!(ForkContext::OFFSET_X21 == 16);
::static_assert::assert_eq!(ForkContext::OFFSET_X23 == 32);
::static_assert::assert_eq!(ForkContext::OFFSET_X25 == 48);
::static_assert::assert_eq!(ForkContext::OFFSET_X27 == 64);
::static_assert::assert_eq!(ForkContext::OFFSET_X29 == 80);
::static_assert::assert_eq!(ForkContext::OFFSET_SP == 88);
::static_assert::assert_eq!(ForkContext::OFFSET_PC == 96);

::core::arch::global_asm!(
    r#"
    .global fork_save_context
    .global fork_trampoline
    .type fork_save_context, %function
    .type fork_trampoline, %function

fork_save_context:
    stp x19, x20, [x0, #{OFFSET_X19}]
    stp x21, x22, [x0, #{OFFSET_X21}]
    stp x23, x24, [x0, #{OFFSET_X23}]
    stp x25, x26, [x0, #{OFFSET_X25}]
    stp x27, x28, [x0, #{OFFSET_X27}]
    str x29, [x0, #{OFFSET_X29}]
    mov x9, sp
    str x9, [x0, #{OFFSET_SP}]
    str x30, [x0, #{OFFSET_PC}]
    mov w0, wzr
    ret

fork_trampoline:
    mov x9, x0
    ldp x19, x20, [x9, #{OFFSET_X19}]
    ldp x21, x22, [x9, #{OFFSET_X21}]
    ldp x23, x24, [x9, #{OFFSET_X23}]
    ldp x25, x26, [x9, #{OFFSET_X25}]
    ldp x27, x28, [x9, #{OFFSET_X27}]
    ldr x29, [x9, #{OFFSET_X29}]
    ldr x10, [x9, #{OFFSET_SP}]
    ldr x11, [x9, #{OFFSET_PC}]
    mov sp, x10
    mov w0, #{CHILD_RESUME}
    br x11
    "#,
    OFFSET_X19 = const ForkContext::OFFSET_X19,
    OFFSET_X21 = const ForkContext::OFFSET_X21,
    OFFSET_X23 = const ForkContext::OFFSET_X23,
    OFFSET_X25 = const ForkContext::OFFSET_X25,
    OFFSET_X27 = const ForkContext::OFFSET_X27,
    OFFSET_X29 = const ForkContext::OFFSET_X29,
    OFFSET_SP = const ForkContext::OFFSET_SP,
    OFFSET_PC = const ForkContext::OFFSET_PC,
    CHILD_RESUME = const FORK_CHILD_SENTINEL,
);

unsafe extern "C" {
    /// Captures the calling thread's resumable context.
    pub fn fork_save_context(ctx: *mut ForkContext) -> i32;

    /// Entry point of the child's main thread.
    fn fork_trampoline();
}

/// Returns the entry address of the child-side fork trampoline.
pub fn fork_trampoline_address() -> usize {
    fork_trampoline as *const () as usize
}
