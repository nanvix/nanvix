// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Error,
    kcall::{
        arch,
        pm::{
            __kcall_duplicate,
            __kcall_get_thread_data_area,
        },
    },
    mm::{
        Alignment,
        VirtualAddress,
    },
    pm::{
        ProcessIdentifier,
        ThreadCreateArgs,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Page size, in bytes.
///
/// Sourced from [`Alignment`] (the same canonical definition that the `arch` crate's page
/// alignment resolves to) because the `sys` crate cannot depend on the `arch` crate: `arch`
/// already depends on `sys`, so the reverse dependency would form a cycle.
const PAGE_SIZE: usize = Alignment::Align4096 as usize;

/// Alignment of the fork stack, in bytes, as mandated by the System V ABI.
///
/// Sourced from [`Alignment`] (the same canonical definition that the `arch` crate's
/// `STACK_ALIGNMENT` resolves to) for the same dependency-cycle reason as [`PAGE_SIZE`].
const FORK_STACK_ALIGN: usize = Alignment::Align16 as usize;

/// Size of the fork stack used by the child's main thread before it longjmps back into the
/// parent's duplicated stack. A single page is sufficient for the few trampoline instructions.
const FORK_STACK_SIZE: usize = PAGE_SIZE;

// The kernel uses the top of the fork stack as the child's initial stack pointer, so that top must
// honor the System V stack alignment. Since the storage is page-aligned and its size is a whole
// number of pages, the top is aligned as long as the page size is a multiple of the alignment.
::static_assert::assert_eq!(FORK_STACK_SIZE.is_multiple_of(FORK_STACK_ALIGN));

//==================================================================================================
// Fork Stack Storage
//==================================================================================================

/// Page-aligned BSS storage backing the fork stack.
#[repr(align(4096))]
struct ForkStackStorage {
    bytes: [u8; FORK_STACK_SIZE],
}

::static_assert::assert_eq_align!(ForkStackStorage, PAGE_SIZE);

/// Statically allocated fork stack shared by all `fork()` calls.
///
/// The child's main thread enters the fork trampoline with this region as its stack pointer and
/// immediately switches to the parent's duplicated stack without ever reading from or writing to
/// it. Because the storage is never accessed, a single shared static instance is sufficient and
/// safe even across concurrent `fork()` calls; the region merely has to be a valid, writable,
/// suitably aligned user mapping for the kernel to accept it as an initial stack.
static mut FORK_STACK: ForkStackStorage = ForkStackStorage {
    bytes: [0; FORK_STACK_SIZE],
};

//==================================================================================================
// Fork
//==================================================================================================

///
/// # Description
///
/// Duplicates the calling process following POSIX `fork()` semantics, acting as the high-level
/// wrapper over the [`__kcall_duplicate`] kernel call.
///
/// The implementation snapshots the caller's resumable context through the architecture backend,
/// asks the kernel to duplicate the calling process with the child's main thread starting on the
/// shared static fork stack, and lets the kernel publish the process-creation scheduling event
/// that the process manager daemon consumes to record the parent/child relationship. The child
/// resumes from the snapshot and observes a process identifier of zero, while the parent observes
/// the child's process identifier.
///
/// # Returns
///
/// On success, the child's process identifier is returned in the parent and a process identifier
/// of zero is returned in the child. On failure, the raw error reported by the kernel is returned
/// (only in the parent).
///
#[inline(never)]
#[unsafe(no_mangle)]
pub fn __kcall_fork() -> Result<ProcessIdentifier, Error> {
    // Resumable context snapshot. It lives on the stack so it is captured by the child's
    // copy-on-write clone of the parent's address space.
    let mut ctx: arch::ForkContext = arch::ForkContext::default();

    // Snapshot the calling thread's resumable context. The architecture backend resumes the child
    // at the return site within this very frame, which is why the snapshot must be taken directly
    // here (and why this function is never inlined): the child observes the child sentinel, while
    // the parent observes a different value.
    // SAFETY: `ctx` is a valid, properly aligned `ForkContext`, and this is its direct caller.
    let sentinel: i32 = unsafe { arch::fork_save_context(&mut ctx as *mut arch::ForkContext) };
    if sentinel == arch::FORK_CHILD_SENTINEL {
        // Child path. The fork stack was never touched (the child runs on its private copy of the
        // parent's stack), and the shared static storage requires no cleanup, so simply report a
        // process identifier of zero to the caller.
        return Ok(ProcessIdentifier::from(0));
    }

    // Parent path.

    // Preserve the thread-local storage view for the child's main thread.
    let tda: *mut u8 = __kcall_get_thread_data_area()?;

    // Translate the parent's thread data area into the optional form expected by the kernel. A
    // null pointer means the calling thread has no thread-local storage view (the common case for
    // a process main thread), in which case the child's main thread must also start without one.
    // Passing `Some(null)` would be rejected by the kernel as an out-of-range address.
    let user_tda: Option<VirtualAddress> = if tda.is_null() {
        None
    } else {
        Some(VirtualAddress::from_raw_value(tda as usize))
    };

    // Base address of the shared static fork stack. The kernel only uses it as the child's initial
    // stack pointer; the region itself is never read or written by the trampoline.
    // SAFETY: taking the address of the static storage does not form a reference or access it.
    let stack_base: usize = unsafe { (&raw mut FORK_STACK.bytes) as *mut u8 as usize };

    // Ask the kernel to duplicate the calling process. The child's main thread enters the fork
    // trampoline with the address of `ctx` as its first argument.
    let args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: VirtualAddress::from_raw_value(arch::fork_trampoline_address()),
        user_fn_arg0: &ctx as *const arch::ForkContext as usize,
        user_fn_arg1: 0,
        user_stack_base: VirtualAddress::from_raw_value(stack_base),
        user_stack_size: FORK_STACK_SIZE,
        user_tda,
    };

    // The parent does not need to register the child: the kernel publishes a process-creation
    // scheduling event for the new child, which the process manager daemon consumes to record the
    // parent/child relationship and to drive cloning of the parent's filesystem resources onto the
    // child.
    __kcall_duplicate(&args)
}
