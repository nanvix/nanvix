// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Issues a kernel call with no arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall0(kcall_nr: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with one argument.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall1(kcall_nr: u32, arg0: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with two arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall2(kcall_nr: u32, arg0: u32, arg1: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            in("rsi") arg1,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with three arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall3(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with four arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall4(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            in("r10") arg3,
            options(nostack, preserves_flags)
        );
    }
    ret
}

//==================================================================================================
// Thread Data Area Helpers
//==================================================================================================

///
/// # Description
///
/// Reads a `u32` value from the Thread Data Area (TDA) via the `%fs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
///
/// # Returns
///
/// The `u32` value stored at `fs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%fs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn read_tda_u32(offset: u32) -> u32 {
    let val: u32;
    unsafe {
        arch::asm!(
            "mov {0:e}, fs:[{1:e}]",
            out(reg) val,
            in(reg) offset,
            options(nostack, readonly, preserves_flags),
        );
    }
    val
}

///
/// # Description
///
/// Writes a `u32` value to the Thread Data Area (TDA) via the `%fs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
/// - `val`: The `u32` value to store at `fs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%fs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn write_tda_u32(offset: u32, val: u32) {
    unsafe {
        arch::asm!(
            "mov fs:[{0:e}], {1:e}",
            in(reg) offset,
            in(reg) val,
            options(nostack, preserves_flags),
        );
    }
}

///
/// # Description
///
/// Atomically replaces a `u32` value in the Thread Data Area (TDA) via the `%fs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
/// - `val`: The new `u32` value to store at `fs:[offset]`.
///
/// # Returns
///
/// The previous `u32` value stored at `fs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%fs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn swap_tda_u32(offset: u32, mut val: u32) -> u32 {
    unsafe {
        arch::asm!(
            "xchg fs:[{0:e}], {1:e}",
            in(reg) offset,
            inout(reg) val,
            options(nostack, preserves_flags),
        );
    }
    val
}

//==================================================================================================
// Fork Support
//==================================================================================================

///
/// # Description
///
/// Sentinel value returned by [`fork_save_context()`] when the saved context is resumed in the
/// child.
///
pub const FORK_CHILD_SENTINEL: i32 = 1;

///
/// # Description
///
/// Minimal machine context captured by [`fork_save_context()`] and restored by the fork trampoline
/// (see [`fork_trampoline_address()`]).
///
/// Only the callee-saved registers, the stack pointer and the resume address are stored. The
/// caller-saved registers do not need to be preserved across the `fork()` boundary because the
/// compiler already treats [`fork_save_context()`] as an opaque function call. Per the System V
/// AMD64 ABI the callee-saved general-purpose registers are `RBX`, `RBP`, and `R12`–`R15` (note
/// that, unlike the i386 ABI, `RSI`/`RDI` are caller-saved on x86-64).
///
/// The field layout is mirrored by the assembly routines below and must not be reordered.
///
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ForkContext {
    /// Saved `RBX` register (offset 0).
    rbx: u64,
    /// Saved `RBP` register (offset 8).
    rbp: u64,
    /// Saved `R12` register (offset 16).
    r12: u64,
    /// Saved `R13` register (offset 24).
    r13: u64,
    /// Saved `R14` register (offset 32).
    r14: u64,
    /// Saved `R15` register (offset 40).
    r15: u64,
    /// Saved `RSP` register (offset 48).
    rsp: u64,
    /// Saved instruction pointer to resume at (offset 56).
    rip: u64,
}

impl ForkContext {
    /// Byte offset of the `rbx` field.
    const OFFSET_RBX: usize = ::core::mem::offset_of!(Self, rbx);
    /// Byte offset of the `rbp` field.
    const OFFSET_RBP: usize = ::core::mem::offset_of!(Self, rbp);
    /// Byte offset of the `r12` field.
    const OFFSET_R12: usize = ::core::mem::offset_of!(Self, r12);
    /// Byte offset of the `r13` field.
    const OFFSET_R13: usize = ::core::mem::offset_of!(Self, r13);
    /// Byte offset of the `r14` field.
    const OFFSET_R14: usize = ::core::mem::offset_of!(Self, r14);
    /// Byte offset of the `r15` field.
    const OFFSET_R15: usize = ::core::mem::offset_of!(Self, r15);
    /// Byte offset of the `rsp` field.
    const OFFSET_RSP: usize = ::core::mem::offset_of!(Self, rsp);
    /// Byte offset of the `rip` field.
    const OFFSET_RIP: usize = ::core::mem::offset_of!(Self, rip);
}

// The assembly routines below assume a tightly packed, naturally ordered layout. These static
// assertions guard that the Rust layout stays in lock-step with the kernel-facing ABI.
::static_assert::assert_eq_size!(ForkContext, 64);
::static_assert::assert_eq_align!(ForkContext, 8);
::static_assert::assert_eq!(ForkContext::OFFSET_RBX == 0);
::static_assert::assert_eq!(ForkContext::OFFSET_RBP == 8);
::static_assert::assert_eq!(ForkContext::OFFSET_R12 == 16);
::static_assert::assert_eq!(ForkContext::OFFSET_R13 == 24);
::static_assert::assert_eq!(ForkContext::OFFSET_R14 == 32);
::static_assert::assert_eq!(ForkContext::OFFSET_R15 == 40);
::static_assert::assert_eq!(ForkContext::OFFSET_RSP == 48);
::static_assert::assert_eq!(ForkContext::OFFSET_RIP == 56);

::core::arch::global_asm!(
    r#"
    .global fork_save_context
    .global fork_trampoline
    .type fork_save_context, @function
    .type fork_trampoline, @function

    fork_save_context:
        #
        # i32 fork_save_context(ForkContext *ctx)
        #
        # Captures the calling thread's resumable context into *ctx and returns 0. When the saved
        # context is later resumed in the child (see fork_trampoline), execution continues as if
        # this function had returned 1 instead.
        #
        # The System V AMD64 ABI passes the first argument (ctx) in RDI.
        #
        mov [rdi + {OFFSET_RBX}], rbx   # ctx->rbx = RBX
        mov [rdi + {OFFSET_RBP}], rbp   # ctx->rbp = RBP
        mov [rdi + {OFFSET_R12}], r12   # ctx->r12 = R12
        mov [rdi + {OFFSET_R13}], r13   # ctx->r13 = R13
        mov [rdi + {OFFSET_R14}], r14   # ctx->r14 = R14
        mov [rdi + {OFFSET_R15}], r15   # ctx->r15 = R15
        lea rcx, [rsp + 8]              # RCX = caller's RSP after this function returns
        mov [rdi + {OFFSET_RSP}], rcx   # ctx->rsp = RCX
        mov rcx, [rsp]                  # RCX = return address into the caller
        mov [rdi + {OFFSET_RIP}], rcx   # ctx->rip = RCX
        xor eax, eax                    # return 0 in the parent (PARENT_CONTINUE)
        ret

    fork_trampoline:
        #
        # Entry point of the child's main thread.
        #
        # The kernel enters a freshly forged user thread with its first argument (arg0) in RDI (see
        # the create-thread ABI: forge_user_stack() pushes arg0 below the iretq frame and
        # __leave_kernel_to_user_mode pops it into RDI). Here that argument is the address of the
        # parent's ForkContext, which lives in the child's copy-on-write copy of the parent's stack.
        # This stub restores that context, so the child resumes exactly where fork_save_context()
        # was called, observing a return value of CHILD_RESUME (1).
        #
        # No return address is on the stack at entry and the stack is abandoned immediately, so no
        # stack-alignment setup is required.
        #
        mov rbx, [rdi + {OFFSET_RBX}]   # RBX = ctx->rbx
        mov rbp, [rdi + {OFFSET_RBP}]   # RBP = ctx->rbp
        mov r12, [rdi + {OFFSET_R12}]   # R12 = ctx->r12
        mov r13, [rdi + {OFFSET_R13}]   # R13 = ctx->r13
        mov r14, [rdi + {OFFSET_R14}]   # R14 = ctx->r14
        mov r15, [rdi + {OFFSET_R15}]   # R15 = ctx->r15
        mov rcx, [rdi + {OFFSET_RIP}]   # RCX = ctx->rip (resume address)
        mov rsp, [rdi + {OFFSET_RSP}]   # RSP = ctx->rsp (switch to the duplicated parent stack)
        mov eax, {CHILD_RESUME}         # observed return value of fork_save_context() = CHILD_RESUME
        jmp rcx                         # resume in the child just after fork_save_context()
    "#,
    OFFSET_RBX = const ForkContext::OFFSET_RBX,
    OFFSET_RBP = const ForkContext::OFFSET_RBP,
    OFFSET_R12 = const ForkContext::OFFSET_R12,
    OFFSET_R13 = const ForkContext::OFFSET_R13,
    OFFSET_R14 = const ForkContext::OFFSET_R14,
    OFFSET_R15 = const ForkContext::OFFSET_R15,
    OFFSET_RSP = const ForkContext::OFFSET_RSP,
    OFFSET_RIP = const ForkContext::OFFSET_RIP,
    CHILD_RESUME = const FORK_CHILD_SENTINEL,
);

unsafe extern "C" {
    /// Captures the calling thread's resumable context. See the assembly routine above.
    ///
    /// Returns `0` in the parent when the snapshot is taken, and [`FORK_CHILD_SENTINEL`] in the
    /// child when execution later resumes from the snapshot.
    ///
    /// # Safety
    ///
    /// `ctx` must point to a valid, properly aligned [`ForkContext`]. This function must be called
    /// directly from the frame that is meant to resume in the child, because the child resumes at
    /// the return site within that caller; wrapping it in an intermediate frame that is later
    /// popped would leave the child resuming into reclaimed stack.
    pub fn fork_save_context(ctx: *mut ForkContext) -> i32;

    /// Entry point of the child's main thread. See the assembly routine above.
    fn fork_trampoline();
}

///
/// # Description
///
/// Returns the address of the child's main-thread entry trampoline, suitable for use as the
/// `user_fn` of a duplicate kernel call. The trampoline expects the address of the parent's
/// [`ForkContext`] as its first argument.
///
/// # Returns
///
/// The address of the fork trampoline.
///
pub fn fork_trampoline_address() -> usize {
    fork_trampoline as *const () as usize
}
