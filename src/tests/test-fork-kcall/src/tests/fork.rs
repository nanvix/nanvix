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
//!
//! The parent and child rendezvous over IPC so that the parent's post-`fork()` write is
//! guaranteed to happen before the child observes the shared byte. This turns a copy-on-write
//! violation into a deterministic test failure rather than a timing-dependent flake.
//!
//! The following aspects of the `fork()` design are intentionally out of scope here:
//!
//! - Reaping a child via `waitpid()` is covered separately, as `waitpid()` is a follow-up feature.
//! - The non-standalone deployment gate is a compile-time concern.
//! - Failure paths (such as resource exhaustion) are not reliably reproducible at runtime.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicU8,
    AtomicU32,
    Ordering,
};
use ::sys::{
    error::Error,
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
    pm::ProcessIdentifier,
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

//==================================================================================================
// Global State
//==================================================================================================

/// Shared byte living in the program's `.data` segment. After `fork()` the backing page is mapped
/// copy-on-write into both address spaces.
static SHARED_BYTE: AtomicU8 = AtomicU8::new(0);

/// Parent PID, recorded before `fork()` so the child can recover it from copy-on-write memory.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

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
        MessageSender::from(my_pid),
        MessageReceiver::from(parent_pid),
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
        MessageSender::from(parent_pid),
        MessageReceiver::from(child_pid),
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
        MessageSender::from(real_pid),
        MessageReceiver::from(parent),
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
// Public Entry Point
//==================================================================================================

/// Runs all `fork()` regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-fork-kcall: starting fork() regression tests");
    test_fork_cow_and_lineage()?;
    ::syslog::info!("test-fork-kcall: PASS - fork_cow_and_lineage");
    test_fork_pid_cache_invalidation()?;
    ::syslog::info!("test-fork-kcall: PASS - fork_pid_cache_invalidation");
    Ok(())
}
