// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::core::{
    ptr,
    sync::atomic::{
        AtomicBool,
        AtomicI32,
        AtomicUsize,
        Ordering,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::sched::__kcall_sched_yield,
};
use ::sysapi::{
    ffi::c_int,
    pthread::PTHREAD_ONCE_INIT,
    sys_types::pthread_once_t,
};

//==================================================================================================
// External Bindings
//==================================================================================================

unsafe extern "C" {
    /// `pthread_once()` as exported by libposix. These integration tests drive it through its C
    /// ABI, exactly as a C caller would.
    fn pthread_once(
        once_control: *mut pthread_once_t,
        init_routine: Option<unsafe extern "C" fn()>,
    ) -> c_int;
}

//==================================================================================================
// Constants
//==================================================================================================

/// Number of worker threads that race on the shared control word.
const MULTI_WORKERS: usize = 8;

/// Number of times the multi-threaded initializer yields, widening the window during which a
/// concurrent caller can observe the in-progress state (and, if `pthread_once()` were buggy,
/// wrongly return before initialization completes).
const MULTI_INIT_YIELDS: usize = 256;

/// Value published by the multi-threaded initializer as its very last step.  Every worker must
/// observe it after `pthread_once()` returns.
const MULTI_VALUE_SENTINEL: usize = 0x00c0_ffee;

/// Sentinel stored in [`RECURSIVE_INNER_RET`] until the recursive initializer records the return
/// value of its nested `pthread_once()` call.
const RECURSIVE_RET_SENTINEL: c_int = c_int::MIN;

//==================================================================================================
// Globals
//==================================================================================================

// State for the single-threaded test.
static mut ONCE_SINGLE: pthread_once_t = PTHREAD_ONCE_INIT;
static SINGLE_COUNT: AtomicUsize = AtomicUsize::new(0);

// State for the single-threaded recursive test.
static mut ONCE_RECURSIVE: pthread_once_t = PTHREAD_ONCE_INIT;
static RECURSIVE_COUNT: AtomicUsize = AtomicUsize::new(0);
static RECURSIVE_INNER_RET: AtomicI32 = AtomicI32::new(RECURSIVE_RET_SENTINEL);

// State for the multi-threaded test.
static mut ONCE_MULTI: pthread_once_t = PTHREAD_ONCE_INIT;
/// Number of times the initializer body ran (must end at exactly 1).
static MULTI_COUNT: AtomicUsize = AtomicUsize::new(0);
/// Number of threads currently executing the initializer (must never exceed 1).
static MULTI_CONCURRENT: AtomicUsize = AtomicUsize::new(0);
/// Set to `true` as the initializer's final step; read by every worker after `pthread_once()`
/// returns to confirm the initializer's effects are visible.
static MULTI_INIT_DONE: AtomicBool = AtomicBool::new(false);
/// Value published by the initializer; every worker must observe the sentinel on return.
static MULTI_VALUE: AtomicUsize = AtomicUsize::new(0);
/// Release gate so all workers begin contending at roughly the same time.
static MULTI_START_GATE: AtomicBool = AtomicBool::new(false);
/// Count of detected races (overlapping init, or initialization not visible on return).
static MULTI_ANOMALIES: AtomicUsize = AtomicUsize::new(0);
/// Set if `sched_yield()` ever failed inside the initializer.  Surfaced as a test failure rather
/// than being silently ignored: a failed yield stops widening the race window this test relies on.
static MULTI_INIT_YIELD_FAILED: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Exercises `pthread_once()` on a single thread, recursively from within the initializer, and
/// across multiple threads sharing one control word.
pub fn run() -> Result<(), Error> {
    test_single_thread()?;
    test_single_thread_recursive()?;
    test_multi_thread()?;
    Ok(())
}

//==================================================================================================
// Single-threaded test
//==================================================================================================

/// Initializer for the single-threaded test.
unsafe extern "C" fn single_init() {
    SINGLE_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// Verifies that repeated calls on one thread run the initializer exactly once.
fn test_single_thread() -> Result<(), Error> {
    // Reset shared state so the test is re-runnable and order-independent.
    // SAFETY: this single-threaded test owns `ONCE_SINGLE`; no other thread accesses it here.
    unsafe { ONCE_SINGLE = PTHREAD_ONCE_INIT };
    SINGLE_COUNT.store(0, Ordering::SeqCst);

    let control: *mut pthread_once_t = ptr::addr_of_mut!(ONCE_SINGLE);

    // Repeated calls must be idempotent: only the first one runs the initializer.
    for _ in 0..3 {
        // SAFETY: `control` points to a valid, statically-initialized `pthread_once_t`, and
        // `single_init` is a valid initialization routine.
        let ret: c_int = unsafe { pthread_once(control, Some(single_init)) };
        assert_eq!(ret, 0, "pthread_once() must return 0 on success");
    }

    assert_eq!(
        SINGLE_COUNT.load(Ordering::SeqCst),
        1,
        "initializer must run exactly once on a single thread"
    );

    Ok(())
}

//==================================================================================================
// Single-threaded recursive test
//==================================================================================================

/// Initializer for the recursive test. It re-enters `pthread_once()` on the same control word.
unsafe extern "C" fn recursive_init() {
    RECURSIVE_COUNT.fetch_add(1, Ordering::SeqCst);

    // The control word is in the IN_PROGRESS state while this runs, so a nested call on the same
    // control must return without re-running the initializer, rather than deadlocking or recursing
    // forever.
    let control: *mut pthread_once_t = ptr::addr_of_mut!(ONCE_RECURSIVE);
    // SAFETY: `control` points to a valid `pthread_once_t` that is currently being initialized;
    // re-entrancy on the same control is explicitly supported by `pthread_once()`.
    let inner_ret: c_int = unsafe { pthread_once(control, Some(recursive_init)) };
    RECURSIVE_INNER_RET.store(inner_ret, Ordering::SeqCst);
}

/// Verifies that a recursive call from within the initializer neither deadlocks nor re-runs it.
fn test_single_thread_recursive() -> Result<(), Error> {
    // Reset shared state so the test is re-runnable and order-independent.
    // SAFETY: this single-threaded test owns `ONCE_RECURSIVE`; no other thread accesses it here.
    unsafe { ONCE_RECURSIVE = PTHREAD_ONCE_INIT };
    RECURSIVE_COUNT.store(0, Ordering::SeqCst);
    RECURSIVE_INNER_RET.store(RECURSIVE_RET_SENTINEL, Ordering::SeqCst);

    let control: *mut pthread_once_t = ptr::addr_of_mut!(ONCE_RECURSIVE);
    // SAFETY: see `single_init`'s call site.
    let ret: c_int = unsafe { pthread_once(control, Some(recursive_init)) };
    assert_eq!(ret, 0, "outer pthread_once() must return 0 on success");

    assert_eq!(
        RECURSIVE_COUNT.load(Ordering::SeqCst),
        1,
        "initializer must run exactly once despite recursive re-entry"
    );
    assert_eq!(
        RECURSIVE_INNER_RET.load(Ordering::SeqCst),
        0,
        "recursive pthread_once() must return 0 without re-running the initializer"
    );

    Ok(())
}

//==================================================================================================
// Multi-threaded test
//==================================================================================================

/// Initializer for the multi-threaded test.
///
/// It deliberately yields many times so that, while it runs, the other workers are scheduled and
/// call `pthread_once()` on the same (in-progress) control word.  A correct implementation makes
/// them wait until this returns; a racy one would let them proceed before initialization is done.
unsafe extern "C" fn multi_init() {
    // Detect overlapping execution: if more than one thread is ever inside the initializer at
    // once, the exactly-once contract has been violated.
    if MULTI_CONCURRENT.fetch_add(1, Ordering::SeqCst) != 0 {
        MULTI_ANOMALIES.fetch_add(1, Ordering::SeqCst);
    }
    MULTI_COUNT.fetch_add(1, Ordering::SeqCst);

    // Widen the initialization window so concurrent callers have time to observe the in-progress
    // state and contend.
    for _ in 0..MULTI_INIT_YIELDS {
        if __kcall_sched_yield().is_err() {
            // Record the failure so the test reports it; a broken yield narrows the race window
            // this test depends on, so stop spinning on it.
            MULTI_INIT_YIELD_FAILED.store(true, Ordering::SeqCst);
            break;
        }
    }

    // Publish the initialized value, then mark completion as the very last step.
    MULTI_VALUE.store(MULTI_VALUE_SENTINEL, Ordering::SeqCst);
    MULTI_INIT_DONE.store(true, Ordering::SeqCst);
    MULTI_CONCURRENT.fetch_sub(1, Ordering::SeqCst);
}

/// Entry point for a worker thread in the multi-threaded test.
extern "C" fn multi_worker(_arg: usize) -> usize {
    multi_worker_impl().unwrap_or_else(|err| panic!("multi_worker: {err:?}"))
}

fn multi_worker_impl() -> Result<usize, Error> {
    // Wait at the gate so all workers start contending together, maximizing the chance of a race.
    while !MULTI_START_GATE.load(Ordering::Acquire) {
        __kcall_sched_yield()?;
    }

    let control: *mut pthread_once_t = ptr::addr_of_mut!(ONCE_MULTI);
    // SAFETY: `control` points to a valid, statically-initialized `pthread_once_t` shared by every
    // worker, and `multi_init` is a valid initialization routine.
    let ret: c_int = unsafe { pthread_once(control, Some(multi_init)) };
    if ret != 0 {
        // Preserve the actual error number so test failures report the real cause.
        let code: ErrorCode = ErrorCode::try_from(ret).unwrap_or(ErrorCode::TryAgain);
        return Err(Error::new(code, "pthread_once() failed in worker"));
    }

    // POSIX requires that on return from `pthread_once()` the initializer's effects are visible.
    // If a worker observes initialization as incomplete here, the implementation returned before
    // the initializer finished -- a multi-threading race.
    if !MULTI_INIT_DONE.load(Ordering::SeqCst)
        || MULTI_VALUE.load(Ordering::SeqCst) != MULTI_VALUE_SENTINEL
    {
        MULTI_ANOMALIES.fetch_add(1, Ordering::SeqCst);
    }

    Ok(0)
}

/// Verifies that the initializer runs exactly once, never overlaps itself, and is fully visible
/// to every thread on return when several threads race on one control word.
fn test_multi_thread() -> Result<(), Error> {
    // Reset shared state so the test is re-runnable and order-independent.
    // SAFETY: workers are spawned only after this and the start gate is still closed, so no other
    // thread accesses `ONCE_MULTI` concurrently here.
    unsafe { ONCE_MULTI = PTHREAD_ONCE_INIT };
    MULTI_COUNT.store(0, Ordering::SeqCst);
    MULTI_CONCURRENT.store(0, Ordering::SeqCst);
    MULTI_INIT_DONE.store(false, Ordering::SeqCst);
    MULTI_VALUE.store(0, Ordering::SeqCst);
    MULTI_ANOMALIES.store(0, Ordering::SeqCst);
    MULTI_START_GATE.store(false, Ordering::SeqCst);
    MULTI_INIT_YIELD_FAILED.store(false, Ordering::SeqCst);

    let mut workers: [Option<KernelThread>; MULTI_WORKERS] = [const { None }; MULTI_WORKERS];

    // Spawn every worker before releasing the gate so they coexist and contend on the shared
    // control word as simultaneously as possible.
    for slot in workers.iter_mut() {
        *slot = Some(KernelThread::spawn(multi_worker, 0)?);
    }
    MULTI_START_GATE.store(true, Ordering::Release);

    for slot in workers.iter_mut() {
        if let Some(handle) = slot.take() {
            let retval: usize = handle.join()?;
            assert_eq!(retval, 0, "worker reported a pthread_once() failure");
        }
    }

    assert_eq!(
        MULTI_COUNT.load(Ordering::SeqCst),
        1,
        "initializer must run exactly once across all threads"
    );
    assert!(MULTI_INIT_DONE.load(Ordering::SeqCst), "initializer must have completed");
    assert_eq!(
        MULTI_VALUE.load(Ordering::SeqCst),
        MULTI_VALUE_SENTINEL,
        "initializer must have published its value"
    );
    assert_eq!(
        MULTI_ANOMALIES.load(Ordering::SeqCst),
        0,
        "detected a pthread_once() race: overlapping init or init not visible on return"
    );
    assert!(
        !MULTI_INIT_YIELD_FAILED.load(Ordering::SeqCst),
        "sched_yield() failed inside the initializer"
    );

    Ok(())
}
