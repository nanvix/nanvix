// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    InterruptedProcess,
    ProcessState,
};
use crate::{
    hal::arch::{
        x86::cpu::FpuState,
        ContextInformation,
    },
    mm::{
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        process::new_test_thread_termination_credit,
        thread::{
            InterruptReason,
            InterruptedThread,
            ReadyThread,
            SleepingThread,
        },
        ProcessManager,
    },
};
use ::alloc::boxed::Box;
use ::sys::pm::{
    ProcessIdentifier,
    ThreadIdentifier,
};
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Fixture Helpers
//==================================================================================================

///
/// # Description
///
/// Creates a fresh virtual memory space cloned off the current (kernel) process so that test
/// processes can be constructed without disturbing live kernel mappings.
///
/// # Returns
///
/// Upon success, the new [`Vmem`] is returned. Otherwise, [`None`] is returned.
///
fn make_test_vmem() -> Option<Vmem> {
    // SAFETY: the process and virtual memory managers are initialized before in-kernel tests run;
    // access is synchronized because the kernel is single-threaded with interrupts disabled.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
    match mm.new_vmem(pm.current_vmem()) {
        Ok(vmem) => Some(vmem),
        Err(e) => {
            error!("new_vmem failed (error={e:?})");
            None
        },
    }
}

///
/// # Description
///
/// Creates a stub [`ProcessState`] backed by a fresh virtual memory space.
///
/// # Returns
///
/// Upon success, the boxed [`ProcessState`] is returned. Otherwise, [`None`] is returned.
///
fn make_process_state() -> Option<Box<ProcessState>> {
    let vmem: Vmem = make_test_vmem()?;
    Some(Box::new(ProcessState::new(
        ProcessIdentifier::from(1),
        ProcessIdentifier::from(0),
        None,
        vmem,
    )))
}

///
/// # Description
///
/// Creates a [`ReadyThread`] with the given identifier and an otherwise empty context.
///
/// # Parameters
///
/// - `tid`: Raw thread identifier to assign to the fixture.
///
/// # Returns
///
/// A ready thread fixture with the specified identifier.
///
fn make_ready_thread(tid: i32) -> ReadyThread {
    ReadyThread::new(
        ThreadIdentifier::from(tid),
        Some(new_test_thread_termination_credit()),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized (single-threaded kernel init).
        unsafe { FpuState::new() },
    )
}

///
/// # Description
///
/// Creates a [`SleepingThread`] with the given identifier.
///
fn make_sleeping_thread(tid: i32) -> SleepingThread {
    make_ready_thread(tid).run().0.sleep(None).0
}

//==================================================================================================
// Tests
//==================================================================================================

///
/// # Description
///
/// Terminating an already-interrupted process re-marks an interrupted thread whose original reason
/// was [`InterruptReason::TimedOut`] as [`InterruptReason::Killed`], so that the thread exits rather
/// than resuming its timed-out operation when next scheduled.
///
fn test_terminate_overrides_interrupted_reason() -> bool {
    let state: Box<ProcessState> = match make_process_state() {
        Some(state) => state,
        None => return false,
    };

    // A single already-interrupted thread that timed out rather than being killed.
    let interrupted: NonEmptyVecDeque<InterruptedThread> =
        NonEmptyVecDeque::new(make_sleeping_thread(2).interrupt(InterruptReason::TimedOut));
    let process: InterruptedProcess = InterruptedProcess::new(state, interrupted, None);

    let process: InterruptedProcess = process.terminate();

    match process.thread_reason(ThreadIdentifier::from(2)) {
        Some(InterruptReason::Killed) => true,
        other => {
            error!("interrupted thread was not re-marked as killed (reason={other:?})");
            false
        },
    }
}

///
/// # Description
///
/// Terminating an already-interrupted process that still has sleeping threads folds those sleeping
/// threads into the interrupted set as [`InterruptReason::Killed`] threads, alongside the
/// already-interrupted threads.
///
fn test_terminate_folds_sleeping_threads_as_killed() -> bool {
    let state: Box<ProcessState> = match make_process_state() {
        Some(state) => state,
        None => return false,
    };

    // One already-interrupted thread plus one still-sleeping thread.
    let interrupted: NonEmptyVecDeque<InterruptedThread> =
        NonEmptyVecDeque::new(make_sleeping_thread(2).interrupt(InterruptReason::TimedOut));
    let sleeping: NonEmptyVecDeque<SleepingThread> = NonEmptyVecDeque::new(make_sleeping_thread(3));
    let process: InterruptedProcess =
        InterruptedProcess::from_sleeping(state, Some(sleeping), interrupted, None);

    let process: InterruptedProcess = process.terminate();

    // The previously-interrupted thread is now killed.
    if !matches!(process.thread_reason(ThreadIdentifier::from(2)), Some(InterruptReason::Killed)) {
        error!("interrupted thread was not re-marked as killed");
        return false;
    }

    // The previously-sleeping thread is now an interrupted, killed thread.
    if !matches!(process.thread_reason(ThreadIdentifier::from(3)), Some(InterruptReason::Killed)) {
        error!("sleeping thread was not folded into the interrupted set as killed");
        return false;
    }

    true
}

//==================================================================================================
// Test Aggregator
//==================================================================================================

/// Runs all in-kernel unit tests for interrupted-process termination.
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_terminate_overrides_interrupted_reason);
    passed &= run_test!(test_terminate_folds_sleeping_threads_as_killed);
    passed
}
