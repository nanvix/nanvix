// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::alloc::{
    alloc,
    dealloc,
};
use ::core::{
    alloc::Layout,
    ptr,
};
use ::sys::{
    error::ErrorCode,
    kcall::pm::{
        __kcall_duplicate,
        __kcall_get_thread_data_area,
    },
    mm::VirtualAddress,
    pm::ThreadCreateArgs,
};
use ::sysapi::sys_types::pid_t;

//==================================================================================================
// Constants
//==================================================================================================

/// Value returned by [`fork_save_context()`] when the saved context is resumed in the child.
const CHILD_RESUME: i32 = 1;

/// Size of the bootstrap stack used by the child's main thread before it longjmps back into the
/// parent's duplicated stack.
const BOOTSTRAP_STACK_SIZE: usize = 4096;

/// Alignment of the bootstrap stack. The i386 SysV ABI requires 16-byte stack alignment.
const BOOTSTRAP_STACK_ALIGN: usize = 16;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Minimal machine context captured by [`fork_save_context()`] and restored by [`fork_trampoline`].
///
/// Only the callee-saved registers, the stack pointer and the resume address are stored. The
/// caller-saved registers do not need to be preserved across the `fork()` boundary because the
/// compiler already treats [`fork_save_context()`] as an opaque function call.
///
/// The field layout is mirrored by the assembly routines below and must not be reordered.
///
#[repr(C)]
#[derive(Clone, Copy)]
struct ForkContext {
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
    /// Creates a zeroed context.
    const fn new() -> Self {
        Self {
            ebx: 0,
            esi: 0,
            edi: 0,
            ebp: 0,
            esp: 0,
            eip: 0,
        }
    }
}

//==================================================================================================
// Assembly Routines
//==================================================================================================

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
        mov eax, [esp + 4]      # EAX = ctx (first argument)
        mov [eax + 0], ebx      # ctx->ebx = EBX
        mov [eax + 4], esi      # ctx->esi = ESI
        mov [eax + 8], edi      # ctx->edi = EDI
        mov [eax + 12], ebp     # ctx->ebp = EBP
        lea ecx, [esp + 4]      # ECX = caller's ESP after this function returns
        mov [eax + 16], ecx     # ctx->esp = ECX
        mov ecx, [esp]          # ECX = return address into the caller
        mov [eax + 20], ecx     # ctx->eip = ECX
        xor eax, eax            # return 0 in the parent (PARENT_CONTINUE)
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
        mov eax, edx            # EAX = ctx (passed in EDX by the kernel)
        mov ebx, [eax + 0]      # EBX = ctx->ebx
        mov esi, [eax + 4]      # ESI = ctx->esi
        mov edi, [eax + 8]      # EDI = ctx->edi
        mov ebp, [eax + 12]     # EBP = ctx->ebp
        mov ecx, [eax + 20]     # ECX = ctx->eip (resume address)
        mov esp, [eax + 16]     # ESP = ctx->esp (switch to the duplicated parent stack)
        mov eax, 1              # observed return value of fork_save_context() = CHILD_RESUME
        jmp ecx                 # resume in the child just after fork_save_context()
    "#
);

unsafe extern "C" {
    /// Captures the calling thread's resumable context. See the assembly routine above.
    fn fork_save_context(ctx: *mut ForkContext) -> i32;

    /// Entry point of the child's main thread. See the assembly routine above.
    fn fork_trampoline();
}

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Allocates a bootstrap stack for the child's main thread.
///
/// # Returns
///
/// On success, a non-null pointer to the base of the allocated stack is returned. On failure, a
/// null pointer is returned.
///
fn alloc_bootstrap_stack() -> *mut u8 {
    // The size and alignment are compile-time constants known to be valid, so this never fails.
    let layout: Layout = match Layout::from_size_align(BOOTSTRAP_STACK_SIZE, BOOTSTRAP_STACK_ALIGN)
    {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    // SAFETY: The layout has a non-zero size.
    unsafe { alloc(layout) }
}

///
/// # Description
///
/// Frees a bootstrap stack previously allocated by [`alloc_bootstrap_stack()`].
///
/// # Parameters
///
/// - `base`: Base address returned by [`alloc_bootstrap_stack()`]. A null pointer is ignored.
///
/// # Safety
///
/// The caller must ensure that `base` was returned by [`alloc_bootstrap_stack()`] and that the
/// stack is no longer in use.
///
unsafe fn free_bootstrap_stack(base: *mut u8) {
    if base.is_null() {
        return;
    }

    // The layout matches the one used during allocation.
    let layout: Layout = match Layout::from_size_align(BOOTSTRAP_STACK_SIZE, BOOTSTRAP_STACK_ALIGN)
    {
        Ok(layout) => layout,
        Err(_) => return,
    };

    // SAFETY: `base` was allocated with the same layout and is no longer in use.
    unsafe { dealloc(base, layout) }
}

///
/// # Description
///
/// Maps an error reported by the `duplicate()` kernel call onto the error code that `fork()` must
/// surface to user space.
///
/// # Parameters
///
/// - `code`: Error code reported by the kernel.
///
/// # Returns
///
/// The error code to surface to user space.
///
fn map_duplicate_error(code: ErrorCode) -> ErrorCode {
    match code {
        // Resource exhaustion is reported to user space as a transient failure (EAGAIN).
        ErrorCode::OutOfMemory => ErrorCode::TryAgain,
        ErrorCode::OperationNotPermitted => ErrorCode::TryAgain,
        // Anything else is reported as insufficient memory (ENOMEM).
        _ => ErrorCode::OutOfMemory,
    }
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new process by duplicating the calling process, following POSIX `fork()` semantics.
///
/// The implementation snapshots the caller's resumable context, asks the kernel to duplicate the
/// calling process with the child's main thread starting on a fresh bootstrap stack, and records
/// the parent/child relationship with the process daemon. The child resumes from the snapshot and
/// returns `0`, while the parent returns the child's process identifier.
///
/// # Returns
///
/// On success, the child's process identifier is returned in the parent and `0` is returned in the
/// child. On failure, an error code is returned (only in the parent).
///
#[inline(never)]
pub fn do_fork() -> Result<pid_t, ErrorCode> {
    // Resumable context snapshot. It lives on the stack so it is captured by the child's
    // copy-on-write clone of the parent's address space.
    let mut ctx: ForkContext = ForkContext::new();

    // Bootstrap-stack pointer. It is accessed through volatile operations so that it is forced to
    // memory before the clone and re-read from memory after a resume, instead of being cached in a
    // register that the resume would restore to a stale value.
    let mut boot_stack: *mut u8 = ptr::null_mut();

    // Snapshot the calling thread's resumable context. This returns CHILD_RESUME when the child
    // later resumes from the snapshot.
    // SAFETY: `ctx` is a valid, properly aligned `ForkContext`.
    let sentinel: i32 = unsafe { fork_save_context(&mut ctx as *mut ForkContext) };
    if sentinel == CHILD_RESUME {
        // Child path. The bootstrap stack has been abandoned (the child now runs on its private
        // copy of the parent's stack), so release the child's copy-on-write copy of it and report
        // a process identifier of zero to the caller.
        // SAFETY: `boot_stack` was written by the parent before the clone and is read back here.
        let stack: *mut u8 = unsafe { ptr::read_volatile(&boot_stack) };
        // SAFETY: `stack` was allocated by `alloc_bootstrap_stack()` and is no longer in use.
        unsafe { free_bootstrap_stack(stack) };
        return Ok(0);
    }

    // Parent path.

    // Allocate the bootstrap stack for the child's main thread.
    let stack: *mut u8 = alloc_bootstrap_stack();
    if stack.is_null() {
        return Err(ErrorCode::TryAgain);
    }
    // SAFETY: `boot_stack` is a valid local. The volatile write forces it to memory so the clone
    // captures it and the resumed child observes the parent-written value.
    unsafe { ptr::write_volatile(&mut boot_stack, stack) };

    // Preserve the thread-local storage view for the child's main thread.
    let tda: *mut u8 = match __kcall_get_thread_data_area() {
        Ok(tda) => tda,
        Err(_) => {
            // SAFETY: `stack` was just allocated and is not in use.
            unsafe { free_bootstrap_stack(stack) };
            return Err(ErrorCode::OutOfMemory);
        },
    };

    // Translate the parent's thread data area into the optional form expected by the kernel. A
    // null pointer means the calling thread has no thread-local storage view (the common case for
    // a process main thread), in which case the child's main thread must also start without one.
    // Passing `Some(null)` would be rejected by the kernel as an out-of-range address.
    let user_tda: Option<VirtualAddress> = if tda.is_null() {
        None
    } else {
        Some(VirtualAddress::from_raw_value(tda as usize))
    };

    // Ask the kernel to duplicate the calling process. The child's main thread enters
    // `fork_trampoline` with the address of `ctx` as its first argument.
    let args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: VirtualAddress::from_raw_value(fork_trampoline as *const () as usize),
        user_fn_arg0: &ctx as *const ForkContext as usize,
        user_fn_arg1: 0,
        user_stack_base: VirtualAddress::from_raw_value(stack as usize),
        user_stack_size: BOOTSTRAP_STACK_SIZE,
        user_tda,
    };

    match __kcall_duplicate(&args) {
        Ok(child) => {
            // Parent path continues. Release the parent's copy-on-write copy of the bootstrap
            // stack. The child owns and releases its own independent copy when it resumes, so this
            // is neither a leak nor a double free.
            // SAFETY: `stack` was allocated by `alloc_bootstrap_stack()` and the parent no longer
            // uses it.
            unsafe { free_bootstrap_stack(stack) };

            // Record the parent/child relationship with the process daemon. A failure here leaves
            // the child running but untracked, which only degrades `waitpid()`; it is not fatal.
            if let Err(e) = ::proc::register_child(child) {
                ::syslog::warn!("fork(): failed to register child with procd (error={:?})", e);
            }

            Ok(i32::from(child))
        },
        Err(e) => {
            // SAFETY: `stack` was allocated by `alloc_bootstrap_stack()` and is not in use.
            unsafe { free_bootstrap_stack(stack) };
            Err(map_duplicate_error(e.code))
        },
    }
}
