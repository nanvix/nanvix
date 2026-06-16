// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Detached-Thread Use-After-Free Regression Test
//!
//! Stress-tests the detached thread exit path to verify that the kernel does
//! not use-after-free the `ContextInformation` owned by a detached zombie
//! thread. In debug builds, the kernel's `exit_thread` code path contains a
//! `debug_assert!` (paired with slab poison-on-free) that fires if the
//! `ContextInformation` is freed before the context switch. This test
//! exercises the code path by spawning many detached threads that exit
//! immediately, triggering a deterministic kernel panic if the bug is present.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::alloc::Layout;
use ::arch::mem::PAGE_SIZE;
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        pm,
        sched,
    },
    mm::VirtualAddress,
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of detached threads to attempt spawning in the stress test.
///
/// The system-wide thread limit (`MAX_THREADS`) includes the caller's own thread, so the actual
/// number of workers that can be created is at most `MAX_THREADS - 1`. The spawn loop tolerates
/// hitting the limit early and uses the actual spawned count for the completion wait condition.
const MAX_DETACHED_THREADS: usize = ::config::kernel::MAX_THREADS - 1;

//==================================================================================================
// Global State
//==================================================================================================

static WORKERS_FINISHED: AtomicUsize = AtomicUsize::new(0);

//==================================================================================================
// Helper Functions
//==================================================================================================

fn alloc_thread_stack() -> Result<(*mut u8, Layout, VirtualAddress), Error> {
    let layout: Layout = Layout::from_size_align(USER_THREAD_STACK_SIZE, PAGE_SIZE)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "bad stack layout"))?;
    let stack_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if stack_ptr.is_null() {
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate thread stack"));
    }
    let stack_base: VirtualAddress = VirtualAddress::from_raw_value(stack_ptr as usize);
    Ok((stack_ptr, layout, stack_base))
}

fn spawn_detached_thread(
    entry: extern "C" fn(usize) -> usize,
    stack_base: VirtualAddress,
) -> Result<ThreadIdentifier, Error> {
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: entry as *const () as usize,
        user_fn_arg1: 0,
        user_stack_base: stack_base,
        user_stack_size: USER_THREAD_STACK_SIZE,
        user_tda: None,
    };
    let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;
    pm::__kcall_detach_thread(tid)?;
    Ok(tid)
}

//==================================================================================================
// Test
//==================================================================================================

/// Executes detached-thread regression tests.
pub fn run() -> Result<(), Error> {
    test_detach_and_exit_stress()
}

/// Stress-tests detached thread exit to trigger use-after-free.
///
/// Spawns multiple detached threads that exit immediately. Each detached-thread
/// exit exercises the code path where `ContextInformation` is freed. In debug
/// builds, the kernel's `exit_thread` path contains a `debug_assert!` that
/// verifies the `ContextInformation` has not been freed (slab-poisoned) before
/// the context switch saves registers through the `from` pointer. If the UAF
/// bug is present, the assertion fires and the kernel panics.
fn test_detach_and_exit_stress() -> Result<(), Error> {
    WORKERS_FINISHED.store(0, Ordering::Relaxed);

    let mut spawned: usize = 0;
    for _ in 0..MAX_DETACHED_THREADS {
        let (stack_ptr, layout, stack_base) = match alloc_thread_stack() {
            Ok(v) => v,
            Err(_) => break,
        };
        match spawn_detached_thread(worker_fast, stack_base) {
            Ok(_tid) => spawned += 1,
            Err(_) => {
                // Thread limit reached — free the unused stack and stop spawning.
                unsafe { ::alloc::alloc::dealloc(stack_ptr, layout) };
                break;
            },
        }

        // Yield to let the detached worker run and exit. The context switch
        // back to us is where the UAF would corrupt memory.
        sched::__kcall_sched_yield()?;
        sched::__kcall_sched_yield()?;

        // The stack is intentionally leaked: the detached thread may still be
        // using it if the yields above did not schedule it. The kernel will
        // reclaim the memory when the process exits.
        let _ = (stack_ptr, layout);
    }

    assert!(spawned > 0, "failed to spawn any detached worker threads");

    // Wait for all workers to finish. The test harness (nanvix-test) enforces
    // an external timeout, so an unbounded loop is safe here.
    while WORKERS_FINISHED.load(Ordering::Acquire) < spawned {
        sched::__kcall_sched_yield()?;
    }

    Ok(())
}

extern "C" fn worker_fast(_arg: usize) -> usize {
    WORKERS_FINISHED.fetch_add(1, Ordering::Release);
    0
}
