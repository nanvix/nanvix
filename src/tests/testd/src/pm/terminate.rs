// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `terminate()` Permission Tests
//!
//! Verifies that the [`__kcall_terminate`] kernel call enforces an authorization check: only a
//! process that holds the [`Capability::ProcessManagement`] capability may terminate another
//! process. Without this check any process could terminate any other process, which is the bug
//! tracked by issue #1434.
//!
//! The test spawns a spinning child process, then attempts to terminate it *without* holding the
//! process-management capability. The kernel must reject this request with
//! [`ErrorCode::PermissionDenied`]. Afterwards the test acquires the capability, terminates the
//! child for real (which both validates the privileged path and reaps the spinning child), and
//! reclaims the resources it allocated.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    kcall::{
        ipc,
        mm,
        pm,
        sched,
    },
    mm::{
        AccessPermission,
        VirtualAddress,
    },
    pm::{
        Capability,
        ProcessIdentifier,
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations.
const ORDER: Ordering = Ordering::SeqCst;

/// Number of pages backing the child's main-thread stack.
const STACK_PAGES: usize = 2;

/// Size, in bytes, of the child's main-thread stack.
const STACK_BYTES: usize = STACK_PAGES * PAGE_SIZE;

/// Base virtual address of the region used to back the child stack.
///
/// This lies in the guard region between the unified mmap region and the user stack, matching the
/// convention used by the other low-level process-management tests. The stack is mapped only for
/// the duration of the test and unmapped before it returns.
const STACK_REGION_BASE: usize = ::config::memory_layout::USER_MMAP_END_RAW;

// Compile-time check: ensure the child stack fits within the guard region and does not overflow
// into the user stack.
::static_assert::assert_eq!(
    STACK_REGION_BASE + STACK_BYTES <= ::config::memory_layout::USER_STACK_TOP_RAW
);

//==================================================================================================
// Global State
//==================================================================================================

/// Parent process identifier, published before the child is spawned so that the child can recover
/// it from its copy-on-write inherited memory image.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Child Entry Point
//==================================================================================================

/// Spins forever, yielding the processor on each iteration.
fn spin() -> ! {
    loop {
        let _ = sched::__kcall_sched_yield();
    }
}

/// Entry point for the child process spawned by the test.
///
/// The child recovers its own and the parent's identifiers, sends a single acknowledgement back to
/// the parent to prove that it was created and actually executed, then spins until the parent
/// terminates it.
extern "C" fn terminate_child_entry(_arg: usize) -> usize {
    // Drop the parent's cached pid inherited through the duplicated address space. Unlike fork(),
    // the raw duplicate() primitive has no in-child choke point, so the child invalidates here.
    pm::invalidate_cached_pid();

    let my_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(_) => spin(),
    };
    let parent_pid: ProcessIdentifier =
        match ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER)) {
            Ok(pid) => pid,
            Err(_) => spin(),
        };

    // Acknowledge creation to the parent.
    let ack: Message = Message::new(
        MessageSender::new(my_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    let _ = ipc::__kcall_send(&ack);

    // Spin until the parent terminates us.
    spin()
}

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Verifies that a process lacking the [`Capability::ProcessManagement`] capability cannot
/// terminate another process, and that a process holding that capability can.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_terminate_requires_process_management_capability() -> bool {
    let parent_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(_) => return false,
    };
    match u32::try_from(parent_pid) {
        Ok(raw) => PARENT_PID_RAW.store(raw, ORDER),
        Err(_) => return false,
    }

    // Acquire the memory-management capability so that the child's stack can be mapped.
    if pm::__kcall_capctl(Capability::MemoryManagement, true).is_err() {
        return false;
    }

    let stack_base: VirtualAddress = VirtualAddress::from_raw_value(STACK_REGION_BASE);
    let mut success: bool = true;
    let mut child: Option<ProcessIdentifier> = None;

    // Map the child's stack.
    if mm::__kcall_mmap(parent_pid, stack_base, STACK_PAGES, AccessPermission::RDWR).is_err() {
        success = false;
    }

    // Spawn a spinning child process.
    if success {
        let args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn: VirtualAddress::from_raw_value(terminate_child_entry as *const () as usize),
            user_fn_arg0: 0,
            user_fn_arg1: 0,
            user_stack_base: stack_base,
            user_stack_size: STACK_BYTES,
            user_tda: None,
        };
        match pm::__kcall_duplicate(&args) {
            Ok(spawned) if spawned != parent_pid => child = Some(spawned),
            _ => success = false,
        }
    }

    // Wait for the child to acknowledge that it was created and is running.
    if success {
        match ipc::__kcall_recv() {
            Ok(message) if message.message_type == MessageType::Ipc => {},
            _ => success = false,
        }
    }

    // Core of the test: a process without the process-management capability must NOT be able to
    // terminate another process. The kernel must reject the request with `PermissionDenied`.
    if let Some(child) = child {
        match pm::__kcall_terminate(child) {
            // Expected behavior: the kernel denies the unprivileged termination request.
            Err(error) => {
                if error.code != ErrorCode::PermissionDenied {
                    success = false;
                }
            },
            // Bug: the kernel allowed an unprivileged process to terminate another process.
            Ok(()) => success = false,
        }
    }

    // Acquire the process-management capability and terminate the child for real. This validates
    // the privileged path and reaps the spinning child created above. `ResourceBusy` means the
    // capability is already held, so termination must still proceed; the capability is released
    // afterwards only when this path acquired it.
    if let Some(child) = child {
        let acquired: Option<bool> = match pm::__kcall_capctl(Capability::ProcessManagement, true) {
            Ok(()) => Some(true),
            Err(error) if error.code == ErrorCode::ResourceBusy => Some(false),
            Err(_) => None,
        };
        match acquired {
            Some(acquired) => {
                if pm::__kcall_terminate(child).is_err() {
                    success = false;
                }
                if acquired && pm::__kcall_capctl(Capability::ProcessManagement, false).is_err() {
                    success = false;
                }
            },
            None => success = false,
        }
    }

    // Reclaim the child's stack. `mmap()` reserves `STACK_PAGES` pages in a single call, but
    // `munmap()` releases a single page at a time, so every page must be released individually.
    for page in 0..STACK_PAGES {
        let page_addr: usize = STACK_REGION_BASE + page * PAGE_SIZE;
        if mm::__kcall_munmap(parent_pid, VirtualAddress::from_raw_value(page_addr)).is_err() {
            success = false;
        }
    }

    // Release the memory-management capability.
    if pm::__kcall_capctl(Capability::MemoryManagement, false).is_err() {
        success = false;
    }

    success
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests permission enforcement of the `terminate()` kernel call.
///
pub fn test() {
    crate::test!(test_terminate_requires_process_management_capability());
}
