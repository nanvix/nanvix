// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `duplicate()` Burst Stress Test
//!
//! Stresses process creation by spawning many child processes in rapid bursts through the
//! [`__kcall_duplicate`] kernel call. Every `duplicate()` emits a process-creation scheduling
//! event, and every child exit emits a process-termination scheduling event. Both events flow
//! through the kernel's pending-event buffers before being delivered to the owner of the
//! scheduling-event class (the process manager daemon, `procd`).
//!
//! ## Timing and buffering
//!
//! In the standalone deployment `procd` is always spawned first (it is the init process), so by the
//! time this test runs `procd` already owns the scheduling-event class and drains creation and
//! termination events asynchronously. A rapid burst of `duplicate()` calls therefore produces
//! events faster than `procd` can consume them, exercising the kernel-side buffering
//! (`pending_creations`, `pending_terminations`, the per-event `pending_scheduling` queues, and
//! zombie harvesting). If that buffering were insufficient, events would be lost or the kernel
//! would become unstable under the burst. The test asserts that every child is created and runs,
//! which only holds if the buffers
//! absorb the burst correctly.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::sys::{
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
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations.
const ORDER: Ordering = Ordering::SeqCst;

/// Number of children spawned simultaneously in a single burst.
const BURST_BATCH: usize = 8;

/// Number of consecutive bursts performed by the test.
const BURST_ROUNDS: usize = 4;

/// Number of pages backing each child's main-thread stack.
const STACK_PAGES: usize = 2;

/// Size, in bytes, of each child's main-thread stack.
const STACK_BYTES: usize = STACK_PAGES * PAGE_SIZE;

/// Base virtual address of the region used to back child stacks.
///
/// This lies in the guard region between the unified mmap region and the user stack, matching the
/// convention used by the low-level memory-management tests. The stacks are mapped only for the
/// duration of the test and unmapped before it returns.
const STACK_REGION_BASE: usize = ::config::memory_layout::USER_MMAP_END_RAW;

// Compile-time check: ensure the child stacks fit within the guard region and do not overflow into
// the user stack.
::static_assert::assert_eq!(
    STACK_REGION_BASE + BURST_BATCH * STACK_BYTES <= ::config::memory_layout::USER_STACK_TOP_RAW
);

//==================================================================================================
// Global State
//==================================================================================================

/// Parent process identifier, published before the burst so that each child can recover it from its
/// copy-on-write inherited memory image.
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

/// Entry point for a child process spawned by the burst.
///
/// The child recovers its own and the parent's identifiers, sends a single acknowledgement back to
/// the parent to prove that it was created and actually executed, then exits voluntarily with a
/// success status.
extern "C" fn burst_child_entry(_arg: usize) -> usize {
    let my_pid: ProcessIdentifier = match pm::__kcall_getpid() {
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
        MessageSender::from(my_pid),
        MessageReceiver::from(parent_pid),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    let _ = ipc::__kcall_send(&ack);

    // Exit voluntarily with a success status. Using a clean exit (rather than being force-
    // terminated by the parent) keeps the child's termination status at zero and lets `procd`
    // reap it as an ordinary runtime-spawned child instead of treating it as a shutdown trigger.
    let _ = pm::__kcall_exit(0);

    // `exit()` does not return; spin as a defensive fallback.
    spin()
}

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Spawns many child processes in rapid bursts via [`__kcall_duplicate`] and verifies that every
/// child is created and runs.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_duplicate_burst() -> bool {
    let parent_pid: ProcessIdentifier = match pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };
    let raw_parent_pid: u32 = match u32::try_from(parent_pid) {
        Ok(value) => value,
        Err(_) => return false,
    };
    PARENT_PID_RAW.store(raw_parent_pid, ORDER);

    // Acquire memory management capability for mapping child stacks.
    if ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true).is_err() {
        return false;
    }

    let mut success: bool = true;

    'rounds: for _round in 0..BURST_ROUNDS {
        let mut children: [Option<ProcessIdentifier>; BURST_BATCH] = [None; BURST_BATCH];

        // Phase 1: burst-spawn a batch of children as fast as possible.
        for (i, child_slot) in children.iter_mut().enumerate() {
            let stack_base: VirtualAddress =
                VirtualAddress::from_raw_value(STACK_REGION_BASE + i * STACK_BYTES);

            if mm::__kcall_mmap(parent_pid, stack_base, STACK_PAGES, AccessPermission::RDWR)
                .is_err()
            {
                success = false;
                break;
            }

            let args: ThreadCreateArgs = ThreadCreateArgs {
                user_fn: VirtualAddress::from_raw_value(burst_child_entry as *const () as usize),
                user_fn_arg0: 0,
                user_fn_arg1: 0,
                user_stack_base: stack_base,
                user_stack_size: STACK_BYTES,
                user_tda: None,
            };

            match pm::__kcall_duplicate(&args) {
                Ok(child) if child != parent_pid => *child_slot = Some(child),
                _ => {
                    success = false;
                    break;
                },
            }
        }

        // Phase 2: collect exactly one acknowledgement per spawned child.
        if success {
            let spawned: usize = children.iter().filter(|child| child.is_some()).count();
            for _ in 0..spawned {
                match ipc::__kcall_recv() {
                    Ok(message) => {
                        let message_type: MessageType = { message.message_type };
                        if message_type != MessageType::Ipc {
                            success = false;
                            break;
                        }
                    },
                    Err(_) => {
                        success = false;
                        break;
                    },
                }
            }
        }

        // Phase 3: tear down the batch. Each child has already exited voluntarily after
        // acknowledging its creation, so there is no need to force-terminate it here; simply
        // reclaim the stack mappings owned by the parent. The children run in their own
        // copy-on-write address spaces, so unmapping the parent's mappings cannot disturb a child
        // that is still in the process of exiting.
        //
        // `mmap()` reserves `STACK_PAGES` pages per stack in a single call, but `munmap()` unmaps a
        // single page at a time, so every page of every stack must be released individually.
        // Otherwise the trailing pages leak and collide with the next round's mappings. A failed
        // unmap means a page leaked, so fail the test.
        for i in 0..BURST_BATCH {
            for page in 0..STACK_PAGES {
                let page_addr: usize = STACK_REGION_BASE + i * STACK_BYTES + page * PAGE_SIZE;
                if mm::__kcall_munmap(parent_pid, VirtualAddress::from_raw_value(page_addr))
                    .is_err()
                {
                    success = false;
                }
            }
        }

        if !success {
            break 'rounds;
        }
    }

    // Release memory management capability.
    if ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false).is_err() {
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
/// Runs the `duplicate()` burst stress test.
///
pub fn test() {
    crate::test!(test_duplicate_burst());
}
