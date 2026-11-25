// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::runtime::{
    KernelThread,
    raw_pointer_address,
};
use ::config::kernel::{
    COND_OPEN_MAX,
    MUTEX_OPEN_MAX,
};
use ::core::sync::atomic::{
    AtomicBool,
    AtomicU32,
    Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        pm,
        sched::__kcall_sched_yield,
    },
    pm::{
        ConditionAddress,
        MutexAddress,
        SigAction,
        SigSet,
        ThreadIdentifier,
    },
    time::SystemTime,
};
use ::sysapi::signal::SIGUSR1;

const ITERATIONS: usize = MUTEX_OPEN_MAX + COND_OPEN_MAX + 1;
const INTERRUPTIONS: usize = 2;

static MUTEX_KEYS: [u8; ITERATIONS + MUTEX_OPEN_MAX] = [0; ITERATIONS + MUTEX_OPEN_MAX];
static COND_KEYS: [u8; ITERATIONS] = [0; ITERATIONS];
static WAITER: AtomicU32 = AtomicU32::new(0);
static RELEASED: AtomicBool = AtomicBool::new(false);
static HANDLED: AtomicBool = AtomicBool::new(false);
static EARLY_HANDLER: AtomicBool = AtomicBool::new(false);

fn mutex(index: usize) -> MutexAddress {
    MutexAddress::from(raw_pointer_address(core::ptr::from_ref(&MUTEX_KEYS[index]).cast_mut()))
}

fn cond(index: usize) -> ConditionAddress {
    ConditionAddress::from(raw_pointer_address(core::ptr::from_ref(&COND_KEYS[index]).cast_mut()))
}

extern "C" fn handler(_signum: i32) {
    HANDLED.store(true, Ordering::SeqCst);
    if !RELEASED.load(Ordering::SeqCst) {
        EARLY_HANDLER.store(true, Ordering::SeqCst);
    }
}

extern "C" fn holder(index: usize) -> usize {
    holder_impl(index).unwrap_or_else(|error| panic!("condition mutex holder failed: {error:?}"));
    0
}

fn holder_impl(index: usize) -> Result<(), Error> {
    pm::__kcall_lock_mutex(mutex(index), None)?;
    let tid: ThreadIdentifier = ThreadIdentifier::try_from(WAITER.load(Ordering::SeqCst))?;
    if index == 1 {
        pm::__kcall_kill_thread(tid, SIGUSR1)?;
    } else {
        assert_eq!(pm::__kcall_signal_cond(cond(index), false)?, 1, "waiter was not asleep");
    }
    // Let the notified waiter block on the mutex before interrupting its reacquisition.
    __kcall_sched_yield()?;
    for _ in 0..INTERRUPTIONS {
        pm::__kcall_kill_thread(tid, SIGUSR1)?;
        if index == 0 {
            // The first case must not return to user space while the holder is still running.
            __kcall_sched_yield()?;
        }
    }
    RELEASED.store(true, Ordering::SeqCst);
    // In the other cases, release before the interrupted waiter resumes. Its in-flight reference
    // must keep the map entry alive until it stores a guard and eventually unlocks.
    pm::__kcall_unlock_mutex(mutex(index))
}

fn test_interrupted_reacquisition() -> Result<(), Error> {
    WAITER.store(u32::try_from(pm::__kcall_gettid()?)?, Ordering::SeqCst);
    for index in 0..ITERATIONS {
        RELEASED.store(false, Ordering::SeqCst);
        HANDLED.store(false, Ordering::SeqCst);
        EARLY_HANDLER.store(false, Ordering::SeqCst);
        pm::__kcall_lock_mutex(mutex(index), None)?;
        let worker: KernelThread = KernelThread::spawn(holder, index)?;
        let result: Result<(), Error> = pm::__kcall_wait_cond(cond(index), mutex(index), None);
        assert!(RELEASED.load(Ordering::SeqCst), "condition wait returned before mutex release");
        assert!(
            !EARLY_HANDLER.load(Ordering::SeqCst),
            "signal handler ran before mutex restoration"
        );
        assert!(HANDLED.load(Ordering::SeqCst), "the interrupting signal was lost");
        if let Err(error) = result {
            assert_eq!(error.code, ErrorCode::Interrupted, "unexpected condition wait error");
        } else {
            assert_ne!(index, 1, "condition wait interruption was lost");
        }
        pm::__kcall_unlock_mutex(mutex(index))?;
        assert_eq!(worker.join()?, 0, "mutex holder failed");
    }

    // Every slot must be available, not just enough slots to keep reusing a single address.
    for index in ITERATIONS..ITERATIONS + MUTEX_OPEN_MAX {
        pm::__kcall_lock_mutex(mutex(index), None)?;
    }
    for index in ITERATIONS..ITERATIONS + MUTEX_OPEN_MAX {
        pm::__kcall_unlock_mutex(mutex(index))?;
    }
    Ok(())
}

fn test_expired_wait() -> Result<(), Error> {
    // Expired waits must also restore ownership and retire both kinds of map entries.
    for index in 0..ITERATIONS {
        pm::__kcall_lock_mutex(mutex(index), None)?;
        match pm::__kcall_wait_cond(cond(index), mutex(index), Some(SystemTime::default())) {
            Err(error) => assert_eq!(error.code, ErrorCode::OperationTimedOut),
            Ok(()) => panic!("an expired condition wait succeeded"),
        }
        pm::__kcall_unlock_mutex(mutex(index))?;
    }
    Ok(())
}

/// Checks mutex restoration and map reclamation after interrupted and expired condition waits.
pub fn run() -> Result<(), Error> {
    #[allow(clippy::as_conversions)]
    let action: SigAction = SigAction {
        sa_handler: raw_pointer_address((handler as *const ()).cast_mut()),
        ..SigAction::default()
    };
    let mut previous: SigAction = SigAction::default();
    // SAFETY: both action buffers remain valid throughout the kernel call.
    unsafe { pm::__kcall_sigaction(SIGUSR1, &action, &mut previous)? };
    let mut old_mask: SigSet = 0;
    pm::__kcall_sigcheckpoint(None, Some(&mut old_mask))?;
    pm::__kcall_sigcheckpoint(Some(&(old_mask & !(1 << (SIGUSR1 - 1)))), None)?;
    test_interrupted_reacquisition()?;
    test_expired_wait()?;
    pm::__kcall_sigcheckpoint(Some(&old_mask), None)?;
    // SAFETY: the saved action is valid and no output buffer is requested.
    unsafe {
        pm::__kcall_sigaction(SIGUSR1, &previous, core::ptr::null_mut())?;
    }
    Ok(())
}
