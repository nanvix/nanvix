// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `duplicate()` Burst Stress Test
//!
//! Stresses process creation by spawning many child processes in rapid bursts through the
//! [`__kcall_duplicate`] kernel call. Every `duplicate()` emits a process-creation scheduling
//! event, and tearing each child down emits a process-termination scheduling event. Both events
//! flow through the kernel's pending-event buffers before being delivered to the owner of the
//! scheduling-event class (the process manager daemon, `procd`).
//!
//! ## Timing and buffering
//!
//! In the standalone deployment `procd` is always spawned first (it is the init process), so by the
//! time this test runs `procd` already owns the scheduling-event class and drains creation and
//! termination events asynchronously. A rapid burst of `duplicate()` calls therefore produces
//! events faster than `procd` can consume them, exercising the kernel-side buffering
//! (`pending_creations`, `pending_terminations`, the single FIFO `pending_scheduling` queue, and
//! zombie harvesting). If that buffering were insufficient, events would be lost or the kernel
//! would become unstable under the burst.
//!
//! ## Verification
//!
//! The burst children perform no inter-process communication: they simply spin until the parent
//! terminates them. This keeps the parent's mailbox empty for the duration of the burst, which is
//! required because `duplicate()` refuses a caller that owns special resources (including a
//! non-empty mailbox). Tearing each child down with [`__kcall_terminate`] confirms that the kernel
//! tracked every burst-created process — a child lost to a buffer overflow would fail with
//! `NoSuchProcess`. Reaping each child with `waitpid()` then confirms that `procd` observed the
//! raw-duplicate child's process-creation event before its process-termination event and recorded
//! the child as reapable under the parent.
//!
//! ## Reaping after termination
//!
//! `terminate()` only marks a child as a zombie; the child's thread slot is not released until the
//! kernel's idle-loop harvester reaps that zombie and publishes a termination scheduling event.
//! Blocking in `wait()` after each termination makes that path deterministic: the parent sleeps,
//! the kernel harvests the child, `procd` receives the creation and termination events, and the
//! wait reply proves that the child was recorded as reapable before the next burst starts.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::proc::{
    wait,
    WaitOutcome,
    WaitTarget,
};
use ::sys::{
    kcall::{
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
/// The child performs no inter-process communication, so the parent's mailbox stays empty for the
/// whole burst (`duplicate()` refuses a caller that owns special resources such as a non-empty
/// mailbox). It simply spins, yielding the processor, until the parent terminates it.
extern "C" fn burst_child_entry(_arg: usize) -> usize {
    // Drop the parent's cached pid inherited through the duplicated address space. Unlike fork(),
    // the raw duplicate() primitive has no in-child choke point, so the child invalidates here.
    pm::invalidate_cached_pid();
    spin()
}

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Spawns many child processes in rapid bursts via [`__kcall_duplicate`] and verifies that every
/// burst-created child is tracked by the kernel and can be torn down.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_duplicate_burst() -> bool {
    let parent_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    // Acquire the memory-management capability for mapping child stacks.
    if pm::__kcall_capctl(Capability::MemoryManagement, true).is_err() {
        return false;
    }

    // Acquire the process-management capability for tearing the spawned children down. Release the
    // memory-management capability again if this fails so that no capability leaks to later tests.
    if pm::__kcall_capctl(Capability::ProcessManagement, true).is_err() {
        let _ = pm::__kcall_capctl(Capability::MemoryManagement, false);
        return false;
    }

    let mut success: bool = true;

    'rounds: for _round in 0..BURST_ROUNDS {
        let mut children: [Option<ProcessIdentifier>; BURST_BATCH] = [None; BURST_BATCH];

        // Phase 1: burst-spawn a batch of spinning children as fast as possible. The children
        // perform no IPC, so the parent's mailbox stays empty for the whole burst and every
        // duplicate() is accepted regardless of how the children interleave with the parent
        // (duplicate() refuses a caller that owns special resources such as a non-empty mailbox).
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

        // Phase 2: tear down the batch by terminating every spawned child. A successful
        // terminate() confirms that the kernel tracked the process across the burst (a child lost
        // to a buffer overflow would fail with `NoSuchProcess`). terminate() only marks the child
        // as a zombie; the kernel reaps it (and releases its thread slot) asynchronously. These
        // children were created by a user process rather than the kernel, so `procd` reaps them
        // without triggering a system shutdown.
        for child in children.iter().flatten() {
            if pm::__kcall_terminate(*child).is_err() {
                success = false;
            }
        }

        // Phase 3: reap every child through procd. Raw duplicate() does not perform the fork-sync
        // handshake, so the parent can terminate children before procd has drained their
        // process-creation scheduling events. waitpid() returning each child PID confirms that the
        // FIFO scheduling-event stream delivered every creation before its matching termination,
        // letting procd record the child's lineage before finalizing its exit status.
        if success {
            for child in children.iter().flatten() {
                match wait(WaitTarget::Pid(*child), 0) {
                    Ok(WaitOutcome::Reaped { child: reaped, .. }) if reaped == *child => {},
                    _ => success = false,
                }
            }
        }

        // Phase 4: reclaim the stack mappings owned by the parent. `mmap()` reserves `STACK_PAGES`
        // pages per stack in a single call, but `munmap()` unmaps a single page at a time, so every
        // page of every stack must be released individually. Otherwise the trailing pages leak and
        // collide with the next round's mappings. A failed unmap means a page leaked, so fail the
        // test. The children run in their own copy-on-write address spaces, so releasing the
        // parent's mappings cannot disturb them.
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

    // Release the process-management capability.
    if pm::__kcall_capctl(Capability::ProcessManagement, false).is_err() {
        success = false;
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
/// Runs the `duplicate()` burst stress test.
///
pub fn test() {
    crate::test!(test_duplicate_burst());
}
