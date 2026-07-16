// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `fork()` Regression Tests
//!
//! Exercises the POSIX `fork()` library call end-to-end and verifies that:
//!
//! 1. `fork()` returns `0` in the child and the child's PID (greater than zero) in the parent,
//!    with both processes resuming execution at the call site.
//! 2. Copy-on-write isolation holds in both directions: a write performed by the parent after
//!    `fork()` is invisible to the child, and a write performed by the child is invisible to the
//!    parent.
//! 3. The child's parent linkage is correct: `getppid()` in the child returns the parent's PID.
//! 4. A process owning a mutex and a condition variable — accessed through the userspace pthread
//!    interface — is correctly duplicated, and the child can re-initialize the inherited objects
//!    after `fork()` (as a threaded runtime does, e.g. CPython's `PyOS_AfterFork_Child`). This
//!    doubles as a regression test for the userspace pthread-registry bug fixed by PR #2606: the
//!    address-keyed registry is inherited copy-on-write, so before the fix the child's
//!    `pthread_mutex_init`/`pthread_cond_init` reject the re-initialization.
//!
//! The parent and child rendezvous over IPC so that the parent's post-`fork()` write is
//! guaranteed to happen before the child observes the shared byte. This turns a copy-on-write
//! violation into a deterministic test failure rather than a timing-dependent flake.
//!
//! The following aspects of the `fork()` design are intentionally out of scope here:
//!
//! - Reaping a child via `waitpid()` is covered separately, as `waitpid()` is a follow-up feature.
//! - Failure paths (such as resource exhaustion) are not reliably reproducible at runtime.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ptr,
    sync::atomic::{
        AtomicU8,
        AtomicU32,
        Ordering,
    },
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    kcall::{
        fork,
        ipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
};
use ::sysapi::{
    pthread::{
        PTHREAD_COND_INITIALIZER,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys_types::{
        pthread_cond_t,
        pthread_condattr_t,
        pthread_mutex_t,
        pthread_mutexattr_t,
    },
};
use ::syscall::pthread::{
    pthread_cond_init,
    pthread_cond_signal,
    pthread_cond_timedwait,
    pthread_mutex_init,
    pthread_mutex_lock,
    pthread_mutex_unlock,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations.
const ORDER: Ordering = Ordering::SeqCst;

/// Value written by the parent before `fork()`. Both processes observe this until one writes.
const PATTERN_INIT: u8 = 0x11;

/// Value written by the parent after `fork()`. Must remain invisible to the child.
const PATTERN_PARENT: u8 = 0x22;

/// Value written by the child after `fork()`. Must remain invisible to the parent.
const PATTERN_CHILD: u8 = 0x33;

/// Exit status used by the child when all of its checks succeed.
const CHILD_EXIT_OK: i32 = 0;

/// Exit status used by the child when one of its checks fails.
const CHILD_EXIT_FAIL: i32 = 1;

/// Timeout used for the condition-variable waits performed by [`exercise_mutex_condvar`]. No other
/// thread in the same process signals the condition, so each wait is expected to elapse and report
/// [`ErrorCode::OperationTimedOut`]. Chosen long enough to avoid a premature return on a loaded
/// host, yet short enough to keep the test responsive.
const COND_WAIT_TIMEOUT: Duration = Duration::from_millis(100);

/// Status byte sent by the child when it successfully exercised the inherited mutex and condition
/// variable.
const MC_CHILD_OK: u8 = 1;

/// Status byte sent by the child when it failed to exercise the inherited synchronization
/// primitives.
const MC_CHILD_FAIL: u8 = 0;

//==================================================================================================
// Global State
//==================================================================================================

/// Shared byte living in the program's `.data` segment. After `fork()` the backing page is mapped
/// copy-on-write into both address spaces.
static SHARED_BYTE: AtomicU8 = AtomicU8::new(0);

/// Parent PID, recorded before `fork()` so the child can recover it from copy-on-write memory.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

/// Mutex driven through the userspace pthread interface. The parent initializes and uses it before
/// `fork()`; the child re-initializes it afterwards. It lives in the program's data segment and is
/// inherited copy-on-write, so the userspace pthread registry entry the parent created for it is
/// inherited by the child as well.
static mut MC_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

/// Condition variable handled exactly like [`MC_MUTEX`].
static mut MC_COND: pthread_cond_t = PTHREAD_COND_INITIALIZER;

//==================================================================================================
// Child Path
//==================================================================================================

/// Runs the child's observations and reports them to the parent over IPC.
///
/// The child:
///
/// 1. Blocks on `recv()` to synchronize with the parent's post-`fork()` write.
/// 2. Reads [`SHARED_BYTE`] and confirms it observes the value the parent wrote *before* `fork()`.
/// 3. Writes [`PATTERN_CHILD`] to [`SHARED_BYTE`], taking a private copy of the page.
/// 4. Queries `getppid()` and reports both the observed byte and the parent PID to the parent.
fn run_child(parent_pid: ProcessIdentifier) -> Result<(), Error> {
    let my_pid: ProcessIdentifier = pm::getpid_uncached()?;

    // Barrier: block until the parent signals that its post-fork write has completed.
    ipc::__kcall_recv()?;

    // Observe the shared byte. With copy-on-write intact this must equal PATTERN_INIT, not the
    // PATTERN_PARENT value the parent wrote after fork().
    let observed_before: u8 = SHARED_BYTE.load(ORDER);

    // Take a private copy by writing the child's pattern.
    SHARED_BYTE.store(PATTERN_CHILD, ORDER);

    // Query the parent linkage.
    let ppid: i32 = i32::from(pm::__kcall_getppid()?);

    // Report observations to the parent: the observed byte followed by the parent PID.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0] = observed_before;
    payload[1..5].copy_from_slice(&ppid.to_le_bytes());
    let reply: Message = Message::new(
        MessageSender::new(my_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&reply)?;

    Ok(())
}

//==================================================================================================
// Test
//==================================================================================================

/// Runs the fork copy-on-write and lineage scenario.
fn test_fork_cow_and_lineage() -> Result<(), Error> {
    let parent_pid: ProcessIdentifier = pm::getpid_uncached()?;
    PARENT_PID_RAW.store(u32::try_from(parent_pid)?, ORDER);

    // Prime the shared byte with the pre-fork pattern.
    SHARED_BYTE.store(PATTERN_INIT, ORDER);

    // Fork the calling process. Both processes resume execution at this point.
    let child_pid: ProcessIdentifier = fork::__kcall_fork()?;

    // Child: `fork()` returns a process identifier of zero. The child performs its checks and
    // terminates without returning to the shared test flow, so that it never emits the success
    // marker.
    if child_pid == ProcessIdentifier::from(0) {
        let parent: ProcessIdentifier =
            match ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER)) {
                Ok(pid) => pid,
                // The freshly forked child is at a safe point to terminate; it holds no locks or
                // resources that require explicit cleanup.
                Err(_) => pm::__kcall_exit(CHILD_EXIT_FAIL)?,
            };
        let status: i32 = match run_child(parent) {
            Ok(()) => CHILD_EXIT_OK,
            Err(_) => CHILD_EXIT_FAIL,
        };
        // The child terminates here and never returns.
        pm::__kcall_exit(status)?;
    }

    // Parent: a process identifier of zero would indicate the child path; reaching here means this
    // is the parent. `fork()` failures are surfaced as an error by `__kcall_fork()` above.
    assert!(child_pid != parent_pid, "child PID must differ from parent PID");

    // Write a new pattern. With copy-on-write this gives the parent a private copy while the child
    // continues to observe PATTERN_INIT until it performs its own write.
    SHARED_BYTE.store(PATTERN_PARENT, ORDER);

    // Release the child to perform its observation.
    let go: Message = Message::new(
        MessageSender::new(parent_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(child_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    ipc::__kcall_send(&go)?;

    // Receive the child's report.
    let reply: Message = ipc::__kcall_recv()?;
    let reply_type: MessageType = { reply.message_type };
    assert!(reply_type == MessageType::Ipc, "expected IPC reply from child");

    let child_observed_before: u8 = reply.payload[0];
    let child_ppid_raw: i32 = i32::from_le_bytes([
        reply.payload[1],
        reply.payload[2],
        reply.payload[3],
        reply.payload[4],
    ]);
    let parent_observed: u8 = SHARED_BYTE.load(ORDER);

    // CoW invariant 1: the parent's post-fork write is invisible to the child.
    assert!(
        child_observed_before == PATTERN_INIT,
        "child observed {:#x} before its own write; expected {:#x} (parent->child isolation \
         broken)",
        child_observed_before,
        PATTERN_INIT
    );
    // CoW invariant 2: the child's write is invisible to the parent.
    assert!(
        parent_observed == PATTERN_PARENT,
        "parent observed {:#x} after child's write; expected {:#x} (child->parent isolation \
         broken)",
        parent_observed,
        PATTERN_PARENT
    );
    // Lineage invariant: the child's parent is this process.
    assert!(
        child_ppid_raw == i32::from(parent_pid),
        "child getppid() returned {}; expected {} (lineage broken)",
        child_ppid_raw,
        i32::from(parent_pid)
    );

    Ok(())
}

//==================================================================================================
// Process-Identifier Cache Invalidation
//==================================================================================================

/// Reports the child's cached process identifier to the parent over IPC.
///
/// The child resolves its identifier through [`pm::getpid`] — which, if the cache were not
/// invalidated after `fork()`, would still hold the parent's identifier inherited through the
/// duplicated address space. The message is sent under the child's *real* identifier (so the
/// kernel's source-spoofing check accepts it), carrying the cached value in the payload so the
/// parent can compare it against the child's actual pid.
fn report_cached_pid(parent: ProcessIdentifier) -> Result<(), Error> {
    let real_pid: ProcessIdentifier = pm::getpid_uncached()?;
    let cached_pid: ProcessIdentifier = pm::getpid()?;

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0..4].copy_from_slice(&i32::from(cached_pid).to_le_bytes());
    let reply: Message = Message::new(
        MessageSender::new(real_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&reply)?;

    Ok(())
}

/// Verifies that the cached process identifier is invalidated in the child after `fork()`.
///
/// The parent primes the cache with its own identifier before forking. With the cache correctly
/// invalidated in the child half of `fork()`, the child's [`pm::getpid`] must resolve to the
/// child's own identifier — the very value the parent observed as the `fork()` return — rather than
/// the parent's stale identifier. A regression that drops the invalidation makes the child report
/// the parent's pid, turning this into a deterministic failure.
fn test_fork_pid_cache_invalidation() -> Result<(), Error> {
    // Prime the cache with the parent's identifier so a missing invalidation surfaces as the child
    // reporting the parent's value.
    let parent_pid: ProcessIdentifier = pm::getpid()?;
    PARENT_PID_RAW.store(u32::try_from(parent_pid)?, ORDER);

    // Fork the calling process. Both processes resume execution at this point.
    let child_pid: ProcessIdentifier = fork::__kcall_fork()?;

    if child_pid == ProcessIdentifier::from(0) {
        let parent: ProcessIdentifier =
            match ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER)) {
                Ok(pid) => pid,
                // The freshly forked child is at a safe point to terminate; it holds no locks or
                // resources that require explicit cleanup.
                Err(_) => pm::__kcall_exit(CHILD_EXIT_FAIL)?,
            };
        let status: i32 = match report_cached_pid(parent) {
            Ok(()) => CHILD_EXIT_OK,
            Err(_) => CHILD_EXIT_FAIL,
        };
        // The child terminates here and never returns.
        pm::__kcall_exit(status)?;
    }

    // Parent: a process identifier of zero would indicate the child path; reaching here means this
    // is the parent.
    assert!(child_pid != parent_pid, "child PID must differ from parent PID");

    // Receive the child's report and recover the identifier it resolved through the cache.
    let reply: Message = ipc::__kcall_recv()?;
    assert!(reply.message_type == MessageType::Ipc, "expected IPC reply from child");
    let child_cached_raw: i32 = i32::from_le_bytes([
        reply.payload[0],
        reply.payload[1],
        reply.payload[2],
        reply.payload[3],
    ]);

    // The child's cached identifier must equal its own pid (the fork() return observed here), which
    // is only possible if the cache was invalidated and re-queried in the child.
    assert!(
        child_cached_raw == i32::from(child_pid),
        "child getpid() returned {}; expected child pid {} (cache not invalidated after fork)",
        child_cached_raw,
        i32::from(child_pid)
    );
    // And it must not be the parent's stale identifier.
    assert!(
        child_cached_raw != i32::from(parent_pid),
        "child getpid() returned the parent's pid {} (stale cache inherited across fork)",
        i32::from(parent_pid)
    );

    Ok(())
}

//==================================================================================================
// Mutex / Condition-Variable Inheritance
//==================================================================================================

/// Computes an absolute deadline [`COND_WAIT_TIMEOUT`] in the future from the monotonic clock.
fn cond_wait_deadline() -> Result<SystemTime, Error> {
    let mut now: SystemTime = SystemTime::default();
    pm::__kcall_gettime(&mut now)?;
    now.checked_add_duration(&COND_WAIT_TIMEOUT)
        .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "condition wait deadline overflow"))
}

/// Initializes the mutex and condition variable through `pthread_mutex_init`/`pthread_cond_init`.
///
/// The parent calls this once to register the objects. The child calls it again after `fork()` to
/// re-initialize the inherited objects — exactly what a threaded runtime does in its post-`fork()`
/// child (for example CPython's `PyOS_AfterFork_Child`, which resets the GIL's lock in place). On a
/// correct implementation both calls succeed.
///
/// The child's call is the operation that reproduces the userspace-registry bug fixed by PR #2606:
/// the address-keyed pthread registry is inherited copy-on-write, so it still lists the addresses
/// the parent registered. Before the fix `pthread_mutex_init` rejects the re-initialization with
/// `InvalidArgument` (EINVAL) and `pthread_cond_init` with `ResourceBusy` (EBUSY), even though the
/// kernel already dropped the underlying objects in the child.
fn init_mutex_condvar() -> Result<(), Error> {
    let mutex_attr: pthread_mutexattr_t = pthread_mutexattr_t::default();
    let cond_attr: pthread_condattr_t = pthread_condattr_t::default();
    // SAFETY: the process is single-threaded here, so each reference is exclusive, does not alias,
    // and is confined to its `pthread_*_init` call.
    unsafe {
        pthread_mutex_init(&mut *ptr::addr_of_mut!(MC_MUTEX), &mutex_attr)?;
        pthread_cond_init(&mut *ptr::addr_of_mut!(MC_COND), &cond_attr)?;
    }
    Ok(())
}

/// Drives the mutex and condition variable through the userspace pthread interface, proving they
/// are functional in the calling process.
///
/// Performs a canonical lock / wait / unlock / signal cycle:
///
/// 1. Locks the mutex via `pthread_mutex_lock`. A forked process must acquire it without
///    deadlocking.
/// 2. Reads the mutex-guarded byte [`SHARED_BYTE`] under the lock and remembers it.
/// 3. Waits on the condition variable via `pthread_cond_timedwait` with [`COND_WAIT_TIMEOUT`]. With
///    no signaler in this process the wait must elapse and report [`ErrorCode::OperationTimedOut`],
///    which also exercises the mutex release/reacquire performed inside the wait.
/// 4. Writes `write_value` to the guarded byte, still under the reacquired lock, taking a private
///    copy-on-write page.
/// 5. Unlocks the mutex via `pthread_mutex_unlock`.
/// 6. Signals the condition variable via `pthread_cond_signal`. With no waiters the call simply
///    succeeds.
///
/// Returns the value observed in step 2.
fn exercise_mutex_condvar(write_value: u8) -> Result<u8, Error> {
    // Step 1: acquire the mutex.
    // SAFETY: single-threaded; the reference is exclusive and confined to this call.
    unsafe { pthread_mutex_lock(&mut *ptr::addr_of_mut!(MC_MUTEX))? };

    // Step 2: observe the guarded data under the lock.
    let observed: u8 = SHARED_BYTE.load(ORDER);

    // Step 3: wait on the condition variable; with no signaler in this process it must time out.
    let deadline: SystemTime = cond_wait_deadline()?;
    // SAFETY: single-threaded; the shared references are confined to this call and the mutex is
    // held, as `pthread_cond_timedwait` requires.
    let wait: Result<(), Error> = unsafe {
        pthread_cond_timedwait(&*ptr::addr_of!(MC_COND), &*ptr::addr_of!(MC_MUTEX), Some(deadline))
    };
    match wait {
        Err(e) if e.code == ErrorCode::OperationTimedOut => {},
        Err(e) => {
            return Err(Error::new(e.code, "pthread_cond_timedwait returned an unexpected error"));
        },
        Ok(()) => {
            return Err(Error::new(
                ErrorCode::OperationNotPermitted,
                "pthread_cond_timedwait returned without a signal; expected a timeout",
            ));
        },
    }

    // Step 4: take a private copy of the guarded data, still holding the reacquired lock.
    SHARED_BYTE.store(write_value, ORDER);

    // Step 5: release the mutex.
    // SAFETY: single-threaded; the reference is exclusive and confined to this call.
    unsafe { pthread_mutex_unlock(&mut *ptr::addr_of_mut!(MC_MUTEX))? };

    // Step 6: signal the condition variable; with no waiters this simply succeeds.
    // SAFETY: single-threaded; the shared reference is confined to this call.
    unsafe { pthread_cond_signal(&*ptr::addr_of!(MC_COND))? };

    Ok(observed)
}

/// Runs the child's post-`fork()` re-initialization and mutex/condition-variable exercise, then
/// reports the outcome to the parent.
///
/// The payload carries a status byte ([`MC_CHILD_OK`] or [`MC_CHILD_FAIL`]), the bytes observed
/// before and after the child's own write, and the raw error code of the first failing operation
/// (zero on success). This makes the PR #2606 regression deterministic and self-describing: before
/// the fix the child reports [`MC_CHILD_FAIL`] with the `pthread_mutex_init`/`pthread_cond_init`
/// error code, so the parent's assertion names it instead of the test hanging.
fn run_child_mutex_condvar(parent_pid: ProcessIdentifier) -> Result<(), Error> {
    let my_pid: ProcessIdentifier = pm::getpid_uncached()?;

    // Barrier: block until the parent signals that its post-fork write has completed.
    ipc::__kcall_recv()?;

    // Re-initialize the inherited objects (the bug trigger), then drive them, taking a private copy
    // via PATTERN_CHILD.
    let outcome: Result<u8, Error> =
        init_mutex_condvar().and_then(|()| exercise_mutex_condvar(PATTERN_CHILD));
    let (status, observed_before, observed_after, errcode): (u8, u8, u8, i32) = match outcome {
        Ok(observed) => (MC_CHILD_OK, observed, SHARED_BYTE.load(ORDER), 0),
        Err(e) => (MC_CHILD_FAIL, 0, 0, i32::from(e.code)),
    };

    // Report observations to the parent: status, observed bytes, and the failing error code.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0] = status;
    payload[1] = observed_before;
    payload[2] = observed_after;
    payload[4..8].copy_from_slice(&errcode.to_le_bytes());
    let reply: Message = Message::new(
        MessageSender::new(my_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&reply)?;

    Ok(())
}

/// Verifies that a process owning a mutex and a condition variable — accessed through the userspace
/// pthread interface — is correctly duplicated by `fork()`, and that the child can re-initialize the
/// inherited objects, conforming to the POSIX `fork()` contract
/// (<https://pubs.opengroup.org/onlinepubs/9799919799/functions/fork.html>).
///
/// POSIX makes the child a single-threaded replica of the calling thread and its address space, and
/// requires both processes to execute independently afterwards. A threaded runtime typically resets
/// its inherited locks in the child (CPython does this in `PyOS_AfterFork_Child`). This test mirrors
/// that: the parent initializes and exercises the mutex and condition variable, then `fork()`s; the
/// child re-initializes the inherited objects and drives them through a full lock / wait / unlock /
/// signal cycle. The parent does not hold the lock across `fork()`, so the child never operates on a
/// parent-held non-process-shared lock (undefined behavior per POSIX).
///
/// The re-initialization reproduces the userspace-registry bug fixed by PR #2606: the address-keyed
/// pthread registry is inherited copy-on-write, so before the fix `pthread_mutex_init`/
/// `pthread_cond_init` reject the child's reset with `InvalidArgument`/`ResourceBusy` and this test
/// fails; with the fix they reset the entry and the test passes. The mutex-guarded byte additionally
/// validates the POSIX MAP_PRIVATE (copy-on-write) data rules.
fn test_fork_mutex_condvar() -> Result<(), Error> {
    let parent_pid: ProcessIdentifier = pm::getpid_uncached()?;
    PARENT_PID_RAW.store(u32::try_from(parent_pid)?, ORDER);

    // Prime the guarded byte with the pre-fork pattern.
    SHARED_BYTE.store(PATTERN_INIT, ORDER);

    // Initialize and exercise the mutex and condition variable in the parent before forking. The
    // init calls register the objects' addresses in the userspace pthread registry; that registry
    // is what the child inherits copy-on-write. The mutex is locked and unlocked here (not held
    // across `fork()`).
    init_mutex_condvar()?;
    // SAFETY: single-threaded; each reference is exclusive and confined to its call.
    unsafe {
        pthread_mutex_lock(&mut *ptr::addr_of_mut!(MC_MUTEX))?;
        pthread_mutex_unlock(&mut *ptr::addr_of_mut!(MC_MUTEX))?;
    }

    // Fork the calling process. Both processes resume execution at this point.
    let child_pid: ProcessIdentifier = fork::__kcall_fork()?;

    // Child: `fork()` returns a process identifier of zero. It re-initializes and drives the
    // inherited primitives, reports to the parent, and terminates without returning to the shared
    // test flow.
    if child_pid == ProcessIdentifier::from(0) {
        let parent: ProcessIdentifier =
            match ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER)) {
                Ok(pid) => pid,
                // The freshly forked child is at a safe point to terminate; it holds no locks or
                // resources that require explicit cleanup.
                Err(_) => pm::__kcall_exit(CHILD_EXIT_FAIL)?,
            };
        let status: i32 = match run_child_mutex_condvar(parent) {
            Ok(()) => CHILD_EXIT_OK,
            Err(_) => CHILD_EXIT_FAIL,
        };
        // The child terminates here and never returns.
        pm::__kcall_exit(status)?;
    }

    // Parent: reaching here means this is the parent. `fork()` failures are surfaced as an error by
    // `__kcall_fork()` above.
    assert!(child_pid != parent_pid, "child PID must differ from parent PID");

    // The parent drives its own independent mutex and condition variable, taking a private copy via
    // PATTERN_PARENT.
    let parent_observed_before: u8 = exercise_mutex_condvar(PATTERN_PARENT)?;
    assert!(
        parent_observed_before == PATTERN_INIT,
        "parent observed {:#x} before its own write; expected {:#x}",
        parent_observed_before,
        PATTERN_INIT
    );

    // Release the child to perform its observation.
    let go: Message = Message::new(
        MessageSender::new(parent_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(child_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    ipc::__kcall_send(&go)?;

    // Receive the child's report.
    let reply: Message = ipc::__kcall_recv()?;
    assert!(reply.message_type == MessageType::Ipc, "expected IPC reply from child");

    let child_status: u8 = reply.payload[0];
    let child_observed_before: u8 = reply.payload[1];
    let child_observed_after: u8 = reply.payload[2];
    let child_errcode: i32 = i32::from_le_bytes([
        reply.payload[4],
        reply.payload[5],
        reply.payload[6],
        reply.payload[7],
    ]);
    let parent_observed: u8 = SHARED_BYTE.load(ORDER);

    // The child must have re-initialized and driven the inherited mutex and condition variable.
    // Before PR #2606 the child's `pthread_mutex_init`/`pthread_cond_init` fail on the inherited
    // registry entry, so this assertion fires with the reported EINVAL/EBUSY code.
    assert!(
        child_status == MC_CHILD_OK,
        "child failed to re-initialize/exercise the inherited mutex and condition variable after \
         fork() (error code={}); this is the PR #2606 stale userspace-registry bug \
         (pthread_mutex_init -> EINVAL / pthread_cond_init -> EBUSY on the inherited entry)",
        child_errcode
    );
    // CoW invariant 1: the parent's post-fork write is invisible to the child.
    assert!(
        child_observed_before == PATTERN_INIT,
        "child observed {:#x} before its own write; expected {:#x} (parent->child isolation \
         broken)",
        child_observed_before,
        PATTERN_INIT
    );
    // The child's own write under the lock must be visible to itself.
    assert!(
        child_observed_after == PATTERN_CHILD,
        "child observed {:#x} after its own write; expected {:#x}",
        child_observed_after,
        PATTERN_CHILD
    );
    // CoW invariant 2: the child's write is invisible to the parent.
    assert!(
        parent_observed == PATTERN_PARENT,
        "parent observed {:#x} after child's write; expected {:#x} (child->parent isolation \
         broken)",
        parent_observed,
        PATTERN_PARENT
    );

    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all `fork()` regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-fork-kcall: starting fork() regression tests");
    test_fork_cow_and_lineage()?;
    ::syslog::info!("test-fork-kcall: PASS - fork_cow_and_lineage");
    test_fork_pid_cache_invalidation()?;
    ::syslog::info!("test-fork-kcall: PASS - fork_pid_cache_invalidation");
    test_fork_mutex_condvar()?;
    ::syslog::info!("test-fork-kcall: PASS - fork_mutex_condvar");
    Ok(())
}
