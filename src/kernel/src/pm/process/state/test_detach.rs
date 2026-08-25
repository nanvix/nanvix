// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    ProcessState,
    RunningProcess,
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
            ZombieThread,
        },
        ProcessManager,
    },
};
use ::alloc::boxed::Box;
use ::sys::{
    error::ErrorCode,
    event::ThreadTerminationInfo,
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
    // SAFETY: pm/init() runs after the process and virtual memory managers are initialized;
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
/// Creates a [`RunningThread`] with the given identifier.
///
fn make_running_thread(tid: i32) -> RunningThread {
    make_ready_thread(tid).run().0
}

///
/// # Description
///
/// Creates a [`ZombieThread`] with the given identifier.
///
/// # Parameters
///
/// - `tid`: Raw thread identifier to assign to the fixture.
///
/// # Returns
///
/// A zombie thread fixture with the specified identifier.
///
fn make_zombie_thread(tid: i32) -> ZombieThread {
    let (transition, _ctx) =
        make_running_thread(tid).exit(ProcessIdentifier::from(1), ExitStatus::from(0u32));
    transition.into_parts().0
}

///
/// # Description
///
/// Creates a detached [`ZombieThread`] with the given identifier by detaching the running thread
/// before it exits.
///
/// # Parameters
///
/// - `tid`: Raw thread identifier to assign to the fixture.
///
/// # Returns
///
/// A detached zombie thread fixture with the specified identifier.
///
fn make_detached_zombie_thread(tid: i32) -> ZombieThread {
    let mut running: RunningThread = make_running_thread(tid);
    running.set_detached();
    let (transition, _ctx) = running.exit(ProcessIdentifier::from(1), ExitStatus::from(0u32));
    transition.into_parts().0
}

///
/// # Description
///
/// Creates a [`SleepingThread`] with the given identifier.
///
fn make_sleeping_thread(tid: i32) -> SleepingThread {
    make_running_thread(tid).sleep(None).0
}

///
/// # Description
///
/// Creates an [`InterruptedThread`] with the given identifier.
///
fn make_interrupted_thread(tid: i32) -> InterruptedThread {
    make_sleeping_thread(tid).interrupt(InterruptReason::Killed)
}

///
/// # Description
///
/// Wraps [`RunningProcess::new`] with a stub [`ProcessState`], assembling a running process from
/// the supplied running thread and optional thread queues.
///
/// # Parameters
///
/// - `running`: Running thread to install in the process.
/// - `ready`: Optional ready-thread queue.
/// - `interrupted`: Optional interrupted-thread queue.
/// - `sleeping`: Optional sleeping-thread queue.
/// - `zombie`: Optional zombie-thread queue.
///
/// # Returns
///
/// Upon success, the new [`RunningProcess`] is returned. Otherwise, [`None`] is returned.
///
fn make_test_process(
    running: RunningThread,
    ready: Option<NonEmptyVecDeque<ReadyThread>>,
    interrupted: Option<NonEmptyVecDeque<InterruptedThread>>,
    sleeping: Option<NonEmptyVecDeque<SleepingThread>>,
    zombie: Option<NonEmptyVecDeque<ZombieThread>>,
) -> Option<RunningProcess> {
    let vmem: Vmem = make_test_vmem()?;
    let state: Box<ProcessState> = Box::new(ProcessState::new(
        ProcessIdentifier::from(1),
        ProcessIdentifier::from(0),
        Some(new_test_process_termination_credit()),
        vmem,
    ));
    Some(RunningProcess::new(state, running, ready, interrupted, sleeping, zombie))
}

//==================================================================================================
// Tests
//==================================================================================================

///
/// # Description
///
/// Detaching the running (calling) thread of a single-thread process marks it detached and
/// reports `Ok(None)`, exercising the `self.running.id() == tid` branch.
///
fn test_detach_running_self() -> bool {
    let running: RunningThread = make_running_thread(1);
    let running_tid: ThreadIdentifier = running.id();
    let mut process: RunningProcess = match make_test_process(running, None, None, None, None) {
        Some(process) => process,
        None => return false,
    };

    match process.detach_thread(running_tid) {
        Ok(None) => {},
        Ok(Some(_)) => {
            error!("detach_thread returned Ok(Some), expected Ok(None)");
            return false;
        },
        Err(e) => {
            error!("detach_thread failed (error={e:?})");
            return false;
        },
    }

    if !process.running_mut().is_detached() {
        error!("running thread was not marked detached");
        return false;
    }

    true
}

///
/// # Description
///
/// Detaching a ready thread marks it detached, reports `Ok(None)`, and leaves the running thread
/// unaffected.
///
fn test_detach_ready_thread() -> bool {
    let running: RunningThread = make_running_thread(1);
    let ready_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let ready: NonEmptyVecDeque<ReadyThread> = NonEmptyVecDeque::new(make_ready_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, Some(ready), None, None, None) {
            Some(process) => process,
            None => return false,
        };

    match process.detach_thread(ready_tid) {
        Ok(None) => {},
        Ok(Some(_)) => {
            error!("detach_thread returned Ok(Some), expected Ok(None)");
            return false;
        },
        Err(e) => {
            error!("detach_thread failed (error={e:?})");
            return false;
        },
    }

    match process.find_thread(ready_tid) {
        Some(thread) => {
            if !thread.is_detached() {
                error!("ready thread was not marked detached");
                return false;
            }
        },
        None => {
            error!("ready thread not found after detach");
            return false;
        },
    }

    if process.running_mut().is_detached() {
        error!("running thread was unexpectedly detached");
        return false;
    }

    true
}

///
/// # Description
///
/// Detaching a zombie thread removes it from the zombie queue and returns it for immediate harvest
/// via `Ok(Some(zombie))`.
///
fn test_detach_zombie_immediate_harvest() -> bool {
    let running: RunningThread = make_running_thread(1);
    let zombie_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let zombie: NonEmptyVecDeque<ZombieThread> = NonEmptyVecDeque::new(make_zombie_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, None, None, None, Some(zombie)) {
            Some(process) => process,
            None => return false,
        };

    match process.detach_thread(zombie_tid) {
        Ok(Some(harvested)) => {
            if harvested.id() != zombie_tid {
                error!("harvested zombie has wrong identifier");
                return false;
            }
        },
        Ok(None) => {
            error!("detach_thread returned Ok(None), expected Ok(Some)");
            return false;
        },
        Err(e) => {
            error!("detach_thread failed (error={e:?})");
            return false;
        },
    }

    if process.find_thread(zombie_tid).is_some() {
        error!("zombie thread still present after immediate harvest");
        return false;
    }

    true
}

///
/// # Description
///
/// Detaching a zombie thread that is already detached still removes it from the zombie queue and
/// returns it for immediate harvest via `Ok(Some(zombie))`, because the zombie path harvests
/// unconditionally.
///
fn test_detach_already_detached_zombie_immediate_harvest() -> bool {
    let running: RunningThread = make_running_thread(1);
    let zombie_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let zombie: NonEmptyVecDeque<ZombieThread> =
        NonEmptyVecDeque::new(make_detached_zombie_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, None, None, None, Some(zombie)) {
            Some(process) => process,
            None => return false,
        };

    match process.detach_thread(zombie_tid) {
        Ok(Some(harvested)) => {
            if harvested.id() != zombie_tid {
                error!("harvested zombie has wrong identifier");
                return false;
            }
            if !harvested.is_detached() {
                error!("harvested zombie was not detached");
                return false;
            }
        },
        Ok(None) => {
            error!("detach_thread returned Ok(None), expected Ok(Some)");
            return false;
        },
        Err(e) => {
            error!("detach_thread failed (error={e:?})");
            return false;
        },
    }

    if process.find_thread(zombie_tid).is_some() {
        error!("zombie thread still present after immediate harvest");
        return false;
    }

    true
}

///
/// # Description
///
/// Detaching a sleeping thread marks it detached and reports `Ok(None)`.
///
fn test_detach_sleeping_thread() -> bool {
    let running: RunningThread = make_running_thread(1);
    let sleeping_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let sleeping: NonEmptyVecDeque<SleepingThread> = NonEmptyVecDeque::new(make_sleeping_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, None, None, Some(sleeping), None) {
            Some(process) => process,
            None => return false,
        };

    match process.detach_thread(sleeping_tid) {
        Ok(None) => {},
        Ok(Some(_)) => {
            error!("detach_thread returned Ok(Some), expected Ok(None)");
            return false;
        },
        Err(e) => {
            error!("detach_thread failed (error={e:?})");
            return false;
        },
    }

    match process.find_thread(sleeping_tid) {
        Some(thread) => {
            if !thread.is_detached() {
                error!("sleeping thread was not marked detached");
                return false;
            }
        },
        None => {
            error!("sleeping thread not found after detach");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Detaching an interrupted thread marks it detached and reports `Ok(None)`.
///
fn test_detach_interrupted_thread() -> bool {
    let running: RunningThread = make_running_thread(1);
    let interrupted_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let interrupted: NonEmptyVecDeque<InterruptedThread> =
        NonEmptyVecDeque::new(make_interrupted_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, None, Some(interrupted), None, None) {
            Some(process) => process,
            None => return false,
        };

    match process.detach_thread(interrupted_tid) {
        Ok(None) => {},
        Ok(Some(_)) => {
            error!("detach_thread returned Ok(Some), expected Ok(None)");
            return false;
        },
        Err(e) => {
            error!("detach_thread failed (error={e:?})");
            return false;
        },
    }

    match process.find_thread(interrupted_tid) {
        Some(thread) => {
            if !thread.is_detached() {
                error!("interrupted thread was not marked detached");
                return false;
            }
        },
        None => {
            error!("interrupted thread not found after detach");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Detaching the running thread twice fails the second time with [`ErrorCode::InvalidArgument`].
///
fn test_detach_already_detached_running() -> bool {
    let running: RunningThread = make_running_thread(1);
    let running_tid: ThreadIdentifier = running.id();
    let mut process: RunningProcess = match make_test_process(running, None, None, None, None) {
        Some(process) => process,
        None => return false,
    };

    if let Err(e) = process.detach_thread(running_tid) {
        error!("first detach_thread failed (error={e:?})");
        return false;
    }

    match process.detach_thread(running_tid) {
        Err(e) => {
            if e.code != ErrorCode::InvalidArgument {
                error!("expected InvalidArgument, got {:?}", e.code);
                return false;
            }
        },
        Ok(_) => {
            error!("second detach_thread unexpectedly succeeded");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Detaching a ready thread twice fails the second time with [`ErrorCode::InvalidArgument`].
///
fn test_detach_already_detached_ready() -> bool {
    let running: RunningThread = make_running_thread(1);
    let ready_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let ready: NonEmptyVecDeque<ReadyThread> = NonEmptyVecDeque::new(make_ready_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, Some(ready), None, None, None) {
            Some(process) => process,
            None => return false,
        };

    if let Err(e) = process.detach_thread(ready_tid) {
        error!("first detach_thread failed (error={e:?})");
        return false;
    }

    match process.detach_thread(ready_tid) {
        Err(e) => {
            if e.code != ErrorCode::InvalidArgument {
                error!("expected InvalidArgument, got {:?}", e.code);
                return false;
            }
        },
        Ok(_) => {
            error!("second detach_thread unexpectedly succeeded");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Detaching a sleeping thread twice fails the second time with [`ErrorCode::InvalidArgument`].
///
fn test_detach_already_detached_sleeping() -> bool {
    let running: RunningThread = make_running_thread(1);
    let sleeping_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let sleeping: NonEmptyVecDeque<SleepingThread> = NonEmptyVecDeque::new(make_sleeping_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, None, None, Some(sleeping), None) {
            Some(process) => process,
            None => return false,
        };

    if let Err(e) = process.detach_thread(sleeping_tid) {
        error!("first detach_thread failed (error={e:?})");
        return false;
    }

    match process.detach_thread(sleeping_tid) {
        Err(e) => {
            if e.code != ErrorCode::InvalidArgument {
                error!("expected InvalidArgument, got {:?}", e.code);
                return false;
            }
        },
        Ok(_) => {
            error!("second detach_thread unexpectedly succeeded");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Detaching an interrupted thread twice fails the second time with [`ErrorCode::InvalidArgument`].
///
fn test_detach_already_detached_interrupted() -> bool {
    let running: RunningThread = make_running_thread(1);
    let interrupted_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let interrupted: NonEmptyVecDeque<InterruptedThread> =
        NonEmptyVecDeque::new(make_interrupted_thread(2));
    let mut process: RunningProcess =
        match make_test_process(running, None, Some(interrupted), None, None) {
            Some(process) => process,
            None => return false,
        };

    if let Err(e) = process.detach_thread(interrupted_tid) {
        error!("first detach_thread failed (error={e:?})");
        return false;
    }

    match process.detach_thread(interrupted_tid) {
        Err(e) => {
            if e.code != ErrorCode::InvalidArgument {
                error!("expected InvalidArgument, got {:?}", e.code);
                return false;
            }
        },
        Ok(_) => {
            error!("second detach_thread unexpectedly succeeded");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Detaching a thread identifier that is not present in any queue fails with
/// [`ErrorCode::NoSuchProcess`].
///
fn test_detach_nonexistent_tid() -> bool {
    let running: RunningThread = make_running_thread(1);
    let mut process: RunningProcess = match make_test_process(running, None, None, None, None) {
        Some(process) => process,
        None => return false,
    };

    match process.detach_thread(ThreadIdentifier::from(99)) {
        Err(e) => {
            if e.code != ErrorCode::NoSuchProcess {
                error!("expected NoSuchProcess, got {:?}", e.code);
                return false;
            }
        },
        Ok(_) => {
            error!("detach_thread unexpectedly succeeded for nonexistent tid");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Exiting a detached running thread while another thread remains drops the detached zombie
/// (returned for deferred reaping) instead of enqueuing it, and the process transitions to a
/// runnable process that does not retain the detached thread.
///
/// # Returns
///
/// `true` if the detached thread produces one termination record and is deferred for reaping,
/// otherwise `false`.
///
fn test_exit_detached_thread_auto_drops() -> bool {
    let mut running: RunningThread = make_running_thread(1);
    running.set_detached();
    let running_tid: ThreadIdentifier = running.id();
    let ready_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let ready: NonEmptyVecDeque<ReadyThread> = NonEmptyVecDeque::new(make_ready_thread(2));
    let process: RunningProcess = match make_test_process(running, Some(ready), None, None, None) {
        Some(process) => process,
        None => return false,
    };

    let status: ExitStatus = ExitStatus::from(0u32);
    let mut termination: Option<ThreadTerminationInfo> = None;
    let (result, deferred) = process.exit_thread(status, &mut |pending| {
        let (info, _credit) = pending.into_parts();
        termination = Some(info);
    });

    if termination
        != Some(ThreadTerminationInfo::new(ProcessIdentifier::from(1), running_tid, status))
    {
        error!("detached thread produced an incorrect termination record");
        return false;
    }
    let deferred: ZombieThread = match deferred {
        Some(zombie) => zombie,
        None => {
            error!("expected detached zombie to be returned for deferred reaping");
            return false;
        },
    };
    let _ = deferred.harvest();
    if termination.is_none() {
        error!("deferred reap changed the committed thread termination record");
        return false;
    }

    match result {
        Ok((_join_cond, runnable, _ctx)) => {
            if runnable.find_thread(running_tid).is_some() {
                error!("detached thread unexpectedly retained in runnable process");
                return false;
            }
            if runnable.find_thread(ready_tid).is_none() {
                error!("ready thread missing from runnable process");
                return false;
            }
        },
        Err(_) => {
            error!("process did not transition to a runnable process");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Exiting a detached running thread while a sleeping thread remains returns the detached zombie
/// for deferred reaping and transitions the process to a sleeping process that does not retain the
/// detached thread.
///
/// # Returns
///
/// `true` if the detached thread is deferred and the process becomes sleeping, otherwise `false`.
///
fn test_exit_detached_thread_auto_drops_with_sleeping() -> bool {
    let mut running: RunningThread = make_running_thread(1);
    running.set_detached();
    let running_tid: ThreadIdentifier = running.id();
    let sleeping_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let sleeping: NonEmptyVecDeque<SleepingThread> = NonEmptyVecDeque::new(make_sleeping_thread(2));
    let process: RunningProcess = match make_test_process(running, None, None, Some(sleeping), None)
    {
        Some(process) => process,
        None => return false,
    };

    let (result, deferred) = process.exit_thread(ExitStatus::from(0u32), &mut |_pending| {});

    if deferred.is_none() {
        error!("expected detached zombie to be returned for deferred reaping");
        return false;
    }

    match result {
        Err(Ok((_join_cond, sleeping_process, _ctx))) => {
            if sleeping_process.find_thread(running_tid).is_some() {
                error!("detached thread unexpectedly retained in sleeping process");
                return false;
            }
            if sleeping_process.find_thread(sleeping_tid).is_none() {
                error!("sleeping thread missing from sleeping process");
                return false;
            }
        },
        _ => {
            error!("process did not transition to a sleeping process");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Exiting a detached running thread while an interrupted thread remains returns the detached
/// zombie for deferred reaping and transitions the process to a runnable process that does not
/// retain the detached thread.
///
/// # Returns
///
/// `true` if the detached thread is deferred and the process becomes runnable, otherwise `false`.
///
fn test_exit_detached_thread_auto_drops_with_interrupted() -> bool {
    let mut running: RunningThread = make_running_thread(1);
    running.set_detached();
    let running_tid: ThreadIdentifier = running.id();
    let interrupted_tid: ThreadIdentifier = ThreadIdentifier::from(2);
    let interrupted: NonEmptyVecDeque<InterruptedThread> =
        NonEmptyVecDeque::new(make_interrupted_thread(2));
    let process: RunningProcess =
        match make_test_process(running, None, Some(interrupted), None, None) {
            Some(process) => process,
            None => return false,
        };

    let (result, deferred) = process.exit_thread(ExitStatus::from(0u32), &mut |_pending| {});

    if deferred.is_none() {
        error!("expected detached zombie to be returned for deferred reaping");
        return false;
    }

    match result {
        Ok((_join_cond, runnable, _ctx)) => {
            if runnable.find_thread(running_tid).is_some() {
                error!("detached thread unexpectedly retained in runnable process");
                return false;
            }
            if runnable.find_thread(interrupted_tid).is_none() {
                error!("interrupted thread missing from runnable process");
                return false;
            }
        },
        Err(_) => {
            error!("process did not transition to a runnable process");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Exiting a detached running thread that is the last thread preserves the zombie (so the zombie
/// process can be constructed) and returns no deferred zombie.
///
/// # Returns
///
/// `true` if the zombie is retained in the zombie-process transition, otherwise `false`.
///
fn test_exit_detached_last_thread_keeps_zombie() -> bool {
    let mut running: RunningThread = make_running_thread(1);
    running.set_detached();
    let running_tid: ThreadIdentifier = running.id();
    let process: RunningProcess = match make_test_process(running, None, None, None, None) {
        Some(process) => process,
        None => return false,
    };

    let (result, deferred) = process.exit_thread(ExitStatus::from(0u32), &mut |_pending| {});

    if deferred.is_some() {
        error!("unexpected deferred zombie for last detached thread");
        return false;
    }

    match result {
        Err(Err((_join_cond, transition, _ctx))) => {
            let (zombie_process, _pending) = transition.into_parts();
            if zombie_process.find_thread(running_tid).is_none() {
                error!("zombie thread not preserved in zombie process");
                return false;
            }
        },
        _ => {
            error!("process did not transition to a zombie process");
            return false;
        },
    }

    true
}

//==================================================================================================
// Test Aggregator
//==================================================================================================

///
/// # Description
///
/// Runs all in-kernel unit tests for thread detach.
///
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_detach_running_self);
    passed &= run_test!(test_detach_ready_thread);
    passed &= run_test!(test_detach_zombie_immediate_harvest);
    passed &= run_test!(test_detach_already_detached_zombie_immediate_harvest);
    passed &= run_test!(test_detach_sleeping_thread);
    passed &= run_test!(test_detach_interrupted_thread);
    passed &= run_test!(test_detach_already_detached_running);
    passed &= run_test!(test_detach_already_detached_ready);
    passed &= run_test!(test_detach_already_detached_sleeping);
    passed &= run_test!(test_detach_already_detached_interrupted);
    passed &= run_test!(test_detach_nonexistent_tid);
    passed &= run_test!(test_exit_detached_thread_auto_drops);
    passed &= run_test!(test_exit_detached_thread_auto_drops_with_sleeping);
    passed &= run_test!(test_exit_detached_thread_auto_drops_with_interrupted);
    passed &= run_test!(test_exit_detached_last_thread_keeps_zombie);
    passed
}
