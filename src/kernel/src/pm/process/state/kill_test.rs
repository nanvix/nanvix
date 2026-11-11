// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    InterruptedProcess,
    ProcessState,
    RunnableProcess,
    RunningProcess,
    SleepingProcess,
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
        process::{
            new_test_process_termination_credit,
            new_test_thread_termination_credit,
        },
        thread::{
            InterruptReason,
            InterruptedThread,
            ReadyThread,
            RunningThread,
            SleepingThread,
        },
        ProcessManager,
    },
};
use ::alloc::{
    boxed::Box,
    vec::Vec,
};
use ::sys::{
    error::ErrorCode,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    ExitStatus,
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

///
/// # Description
///
/// Creates a stub [`ProcessState`] that owns a process termination credit, so that the process can
/// complete its transition into zombie state.
///
/// # Returns
///
/// Upon success, the boxed [`ProcessState`] is returned. Otherwise, [`None`] is returned.
///
fn make_terminable_process_state() -> Option<Box<ProcessState>> {
    let mut state: Box<ProcessState> = make_process_state()?;
    state.install_termination_credit(new_test_process_termination_credit());
    Some(state)
}

///
/// # Description
///
/// Creates a [`RunningThread`] with the given identifier.
///
/// # Parameters
///
/// - `tid`: Raw thread identifier to assign to the fixture.
///
/// # Returns
///
/// A running thread fixture with the specified identifier.
///
fn make_running_thread(tid: i32) -> RunningThread {
    make_ready_thread(tid).run().0
}

///
/// # Description
///
/// Creates an [`InterruptedThread`] with the given identifier and interrupt reason.
///
/// # Parameters
///
/// - `tid`: Raw thread identifier to assign to the fixture.
/// - `reason`: Reason why the thread was interrupted.
///
/// # Returns
///
/// An interrupted thread fixture with the specified identifier and interrupt reason.
///
fn make_interrupted_thread(tid: i32, reason: InterruptReason) -> InterruptedThread {
    make_sleeping_thread(tid).interrupt(reason)
}

///
/// # Description
///
/// Runs a check once for each interrupt reason that, unlike [`InterruptReason::Killed`], lets a
/// resumed thread complete its blocking kernel call and return to user code.
///
/// # Parameters
///
/// - `check`: Check to run for each interrupt reason.
///
/// # Returns
///
/// `true` if the check passes for every interrupt reason, otherwise `false`.
///
fn for_each_resumable_reason(check: fn(InterruptReason) -> bool) -> bool {
    let mut passed: bool = true;
    for reason in [InterruptReason::TimedOut, InterruptReason::Signaled] {
        passed &= check(reason);
    }
    passed
}

///
/// # Description
///
/// Drives a process that is being terminated the way the process manager does, until the process
/// becomes a zombie: runs its next ready thread, checks that the thread resumes with
/// [`InterruptReason::Killed`], and then exits the process on behalf of that thread, as the
/// kernel-call dispatcher does when a killed thread resumes. A thread that resumes with any other
/// reason would instead complete its blocking kernel call and return to user code.
///
/// # Parameters
///
/// - `process`: Process that is being terminated, as the process manager enqueues it.
/// - `carried`: Identifiers of the blocked threads that the process carried into its terminal state.
///
/// # Returns
///
/// `true` if every carried thread resumes exactly once, each of them with
/// [`InterruptReason::Killed`], and the process then becomes a zombie that retains all of them.
/// Otherwise, `false`.
///
fn drive_to_zombie(mut process: RunnableProcess, carried: &[ThreadIdentifier]) -> bool {
    let mut remaining: Vec<ThreadIdentifier> = carried.to_vec();

    // Every iteration retires one carried thread, so the process must become a zombie by the time
    // the last carried thread exits.
    for _ in 0..carried.len() {
        let (running, reason, _ctx, _user_tda) = process.run();
        let tid: ThreadIdentifier = running.get_tid();

        if reason != Some(InterruptReason::Killed) {
            error!("resumed thread was not killed (tid={tid:?}, reason={reason:?})");
            return false;
        }

        match remaining
            .iter()
            .position(|remaining_tid| *remaining_tid == tid)
        {
            Some(index) => {
                remaining.remove(index);
            },
            None => {
                error!("resumed thread was not carried or resumed twice (tid={tid:?})");
                return false;
            },
        }

        match running.exit(ErrorCode::Interrupted.into(), &mut |_pending| {}) {
            Ok((runnable, _ctx)) => process = runnable,
            Err((transition, _ctx)) => {
                if !remaining.is_empty() {
                    error!("process became a zombie with live threads (remaining={remaining:?})");
                    return false;
                }

                let (zombie, _termination) = transition.into_parts();
                if let Some(tid) = carried
                    .iter()
                    .find(|tid| zombie.find_thread(**tid).is_none())
                {
                    error!("zombie process lost a carried thread (tid={tid:?})");
                    return false;
                }

                return true;
            },
        }
    }

    error!("process did not become a zombie after its carried threads exited");
    false
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

///
/// # Description
///
/// Terminating a runnable process kills the interrupted thread that it carries, whatever that
/// thread was interrupted for, so that the thread exits once resumed instead of completing its
/// blocking kernel call and returning to user code after its process was terminated.
///
fn test_terminate_runnable_kills_carried_threads() -> bool {
    for_each_resumable_reason(|reason: InterruptReason| -> bool {
        let state: Box<ProcessState> = match make_terminable_process_state() {
            Some(state) => state,
            None => return false,
        };

        // A ready thread, along with an interrupted sibling and a sleeping sibling.
        let process: RunnableProcess = RunnableProcess::from_state(
            state,
            NonEmptyVecDeque::new(make_ready_thread(1)),
            Some(NonEmptyVecDeque::new(make_interrupted_thread(2, reason))),
            Some(NonEmptyVecDeque::new(make_sleeping_thread(3))),
            None,
        );

        // Terminate the process as the process manager does when the process is ready.
        let process: InterruptedProcess = match process.terminate(&mut |_pending| {}) {
            Ok(process) => process,
            Err(_transition) => {
                error!("terminated process with blocked threads became a zombie");
                return false;
            },
        };

        drive_to_zombie(process.resume(), &[ThreadIdentifier::from(2), ThreadIdentifier::from(3)])
    })
}

///
/// # Description
///
/// Exiting a running process kills the interrupted thread that it carries, whatever that thread
/// was interrupted for, so that the thread exits once resumed instead of completing its blocking
/// kernel call and returning to user code after its process exited.
///
fn test_exit_running_kills_carried_threads() -> bool {
    for_each_resumable_reason(|reason: InterruptReason| -> bool {
        let state: Box<ProcessState> = match make_terminable_process_state() {
            Some(state) => state,
            None => return false,
        };

        // A running thread, along with an interrupted sibling and a sleeping sibling.
        let process: RunningProcess = RunningProcess::new(
            state,
            make_running_thread(1),
            None,
            Some(NonEmptyVecDeque::new(make_interrupted_thread(2, reason))),
            Some(NonEmptyVecDeque::new(make_sleeping_thread(3))),
            None,
        );

        // Exit the process as the process manager does on behalf of the running thread.
        let process: RunnableProcess =
            match process.exit(ExitStatus::from(0u32), &mut |_pending| {}) {
                Ok((process, _ctx)) => process,
                Err((_transition, _ctx)) => {
                    error!("exited process with blocked threads became a zombie");
                    return false;
                },
            };

        drive_to_zombie(process, &[ThreadIdentifier::from(2), ThreadIdentifier::from(3)])
    })
}

///
/// # Description
///
/// Terminating an interrupted process kills the interrupted thread that it carries, whatever that
/// thread was interrupted for, so that the thread exits once resumed instead of completing its
/// blocking kernel call and returning to user code after its process was terminated.
///
fn test_terminate_interrupted_kills_carried_threads() -> bool {
    for_each_resumable_reason(|reason: InterruptReason| -> bool {
        let state: Box<ProcessState> = match make_terminable_process_state() {
            Some(state) => state,
            None => return false,
        };

        // An interrupted thread, along with a sleeping sibling.
        let process: InterruptedProcess = InterruptedProcess::from_sleeping(
            state,
            Some(NonEmptyVecDeque::new(make_sleeping_thread(3))),
            NonEmptyVecDeque::new(make_interrupted_thread(2, reason)),
            None,
        );

        // Terminate the process as the process manager does when the process is interrupted, and
        // resume it as the scheduler then does.
        drive_to_zombie(
            process.terminate().resume(),
            &[ThreadIdentifier::from(2), ThreadIdentifier::from(3)],
        )
    })
}

///
/// # Description
///
/// Terminating a suspended process kills all of its sleeping threads, so that each of them exits
/// once resumed.
///
fn test_terminate_sleeping_kills_all_threads() -> bool {
    let state: Box<ProcessState> = match make_terminable_process_state() {
        Some(state) => state,
        None => return false,
    };

    // Two sleeping threads.
    let mut sleeping: NonEmptyVecDeque<SleepingThread> =
        NonEmptyVecDeque::new(make_sleeping_thread(2));
    sleeping.push_back(make_sleeping_thread(3));
    let process: SleepingProcess = SleepingProcess::new(state, sleeping, None);

    // Terminate the process as the process manager does when the process is suspended, and resume
    // it as the scheduler then does.
    drive_to_zombie(
        process.terminate().resume(),
        &[ThreadIdentifier::from(2), ThreadIdentifier::from(3)],
    )
}

//==================================================================================================
// Test Aggregator
//==================================================================================================

/// Runs all in-kernel unit tests for process termination and exit.
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_terminate_overrides_interrupted_reason);
    passed &= run_test!(test_terminate_folds_sleeping_threads_as_killed);
    passed &= run_test!(test_terminate_runnable_kills_carried_threads);
    passed &= run_test!(test_exit_running_kills_carried_threads);
    passed &= run_test!(test_terminate_interrupted_kills_carried_threads);
    passed &= run_test!(test_terminate_sleeping_kills_all_threads);
    passed
}
