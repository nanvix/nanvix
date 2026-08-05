// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Issues a kernel call with no arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
///
/// # Return
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall0(kcall_nr: u32) -> i64 {
    let low_ret: u32;
    let high_ret: u32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
}

///
/// # Description
///
/// Issues a kernel call with one argument.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall1(kcall_nr: u32, arg0: u32) -> i64 {
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
}

///
/// # Description
///
/// Issues a kernel call with two arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
/// - `arg1` - Second argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall2(kcall_nr: u32, arg0: u32, arg1: u32) -> i64 {
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            in("ecx") arg1,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
}

///
/// # Description
///
/// Issues a kernel call with three arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
/// - `arg1` - Second argument for the kernel call.
/// - `arg2` - Third argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall3(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32) -> i64 {
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            in("ecx") arg1,
            in("edx") arg2,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
}

///
/// # Description
///
/// Issues a kernel call with four arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
/// - `arg1` - Second argument for the kernel call.
/// - `arg2` - Third argument for the kernel call.
/// - `arg3` - Fourth argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall4(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i64 {
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            in("ecx") arg1,
            in("edx") arg2,
            in("edi") arg3,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
}

//==================================================================================================
// Thread Data Area Helpers
//==================================================================================================

///
/// # Description
///
/// Reads a `u32` value from the Thread Data Area (TDA) via the `%gs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
///
/// # Returns
///
/// The `u32` value stored at `gs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%gs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn read_tda_u32(offset: u32) -> u32 {
    let val: u32;
    unsafe {
        arch::asm!(
            "mov {0:e}, gs:[{1:e}]",
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
/// Writes a `u32` value to the Thread Data Area (TDA) via the `%gs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
/// - `val`: The `u32` value to store at `gs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%gs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn write_tda_u32(offset: u32, val: u32) {
    unsafe {
        arch::asm!(
            "mov gs:[{0:e}], {1:e}",
            in(reg) offset,
            in(reg) val,
            options(nostack, preserves_flags),
        );
    }
}

///
/// # Description
///
/// Atomically replaces a `u32` value in the Thread Data Area (TDA) via the `%gs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
/// - `val`: The new `u32` value to store at `gs:[offset]`.
///
/// # Returns
///
/// The previous `u32` value stored at `gs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%gs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn swap_tda_u32(offset: u32, mut val: u32) -> u32 {
    unsafe {
        arch::asm!(
            "xchg gs:[{0:e}], {1:e}",
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
/// compiler already treats [`fork_save_context()`] as an opaque function call.
///
/// The field layout is mirrored by the assembly routines below and must not be reordered.
///
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ForkContext {
    /// Saved `EBX` register (offset 0).
    ebx: u32,
    /// Saved `ESI` register (offset 4).
    esi: u32,
    /// Saved `EDI` register (offset 8).
    edi: u32,
    /// Saved `EBP` register (offset 12).
    ebp: u32,
    /// Saved `ESP` register (offset 16).
    esp: u32,
    /// Saved instruction pointer to resume at (offset 20).
    eip: u32,
}

impl ForkContext {
    /// Byte offset of the `ebx` field.
    const OFFSET_EBX: usize = ::core::mem::offset_of!(Self, ebx);
    /// Byte offset of the `esi` field.
    const OFFSET_ESI: usize = ::core::mem::offset_of!(Self, esi);
    /// Byte offset of the `edi` field.
    const OFFSET_EDI: usize = ::core::mem::offset_of!(Self, edi);
    /// Byte offset of the `ebp` field.
    const OFFSET_EBP: usize = ::core::mem::offset_of!(Self, ebp);
    /// Byte offset of the `esp` field.
    const OFFSET_ESP: usize = ::core::mem::offset_of!(Self, esp);
    /// Byte offset of the `eip` field.
    const OFFSET_EIP: usize = ::core::mem::offset_of!(Self, eip);
}

// The assembly routines below assume a tightly packed, naturally ordered layout. These static
// assertions guard that the Rust layout stays in lock-step with the kernel-facing ABI.
::static_assert::assert_eq_size!(ForkContext, 24);
::static_assert::assert_eq_align!(ForkContext, 4);
::static_assert::assert_eq!(ForkContext::OFFSET_EBX == 0);
::static_assert::assert_eq!(ForkContext::OFFSET_ESI == 4);
::static_assert::assert_eq!(ForkContext::OFFSET_EDI == 8);
::static_assert::assert_eq!(ForkContext::OFFSET_EBP == 12);
::static_assert::assert_eq!(ForkContext::OFFSET_ESP == 16);
::static_assert::assert_eq!(ForkContext::OFFSET_EIP == 20);

::core::arch::global_asm!(
    r#"
    .global fork_save_context
    .global fork_trampoline

    fork_save_context:
        #
        # i32 fork_save_context(ForkContext *ctx)
        #
        # Captures the calling thread's resumable context into *ctx and returns 0. When the saved
        # context is later resumed in the child (see fork_trampoline), execution continues as if
        # this function had returned 1 instead.
        #
        mov eax, [esp + 4]              # EAX = ctx (first argument)
        mov [eax + {OFFSET_EBX}], ebx   # ctx->ebx = EBX
        mov [eax + {OFFSET_ESI}], esi   # ctx->esi = ESI
        mov [eax + {OFFSET_EDI}], edi   # ctx->edi = EDI
        mov [eax + {OFFSET_EBP}], ebp   # ctx->ebp = EBP
        lea ecx, [esp + 4]              # ECX = caller's ESP after this function returns
        mov [eax + {OFFSET_ESP}], ecx   # ctx->esp = ECX
        mov ecx, [esp]                  # ECX = return address into the caller
        mov [eax + {OFFSET_EIP}], ecx   # ctx->eip = ECX
        xor eax, eax                    # return 0 in the parent (PARENT_CONTINUE)
        ret

    fork_trampoline:
        #
        # Entry point of the child's main thread.
        #
        # The kernel enters a freshly forged user thread with its first argument in EDX (see the
        # create-thread ABI). Here that argument is the address of the parent's ForkContext, which
        # lives in the child's copy-on-write copy of the parent's stack. This stub restores that
        # context, so the child resumes exactly where fork_save_context() was called, observing a
        # return value of CHILD_RESUME (1).
        #
        # No return address is on the stack at entry and the stack is abandoned immediately, so no
        # stack-alignment setup is required.
        #
        mov eax, edx                    # EAX = ctx (passed in EDX by the kernel)
        mov ebx, [eax + {OFFSET_EBX}]   # EBX = ctx->ebx
        mov esi, [eax + {OFFSET_ESI}]   # ESI = ctx->esi
        mov edi, [eax + {OFFSET_EDI}]   # EDI = ctx->edi
        mov ebp, [eax + {OFFSET_EBP}]   # EBP = ctx->ebp
        mov ecx, [eax + {OFFSET_EIP}]   # ECX = ctx->eip (resume address)
        mov esp, [eax + {OFFSET_ESP}]   # ESP = ctx->esp (switch to the duplicated parent stack)
        mov eax, {CHILD_RESUME}         # observed return value of fork_save_context() = CHILD_RESUME
        jmp ecx                         # resume in the child just after fork_save_context()
    "#,
    OFFSET_EBX = const ForkContext::OFFSET_EBX,
    OFFSET_ESI = const ForkContext::OFFSET_ESI,
    OFFSET_EDI = const ForkContext::OFFSET_EDI,
    OFFSET_EBP = const ForkContext::OFFSET_EBP,
    OFFSET_ESP = const ForkContext::OFFSET_ESP,
    OFFSET_EIP = const ForkContext::OFFSET_EIP,
    CHILD_RESUME = const FORK_CHILD_SENTINEL,
);

#[cfg(target_os = "none")]
::core::arch::global_asm!(".type fork_save_context, @function", ".type fork_trampoline, @function",);

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
