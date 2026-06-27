// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `waitpid()` Standalone Regression Tests
//!
//! Exercises the POSIX `waitpid()`/`wait()` library calls end-to-end in standalone mode, building
//! on `fork()` to create the children that are reaped. The following scenarios from the design
//! testing strategy are covered deterministically:
//!
//! 1. **`ECHILD`** — `waitpid()` with no children returns `-1` and sets `errno` to `ECHILD`.
//! 2. **`EINVAL`** — `waitpid()` with unsupported `options` bits returns `-1` and sets `EINVAL`.
//! 3. **`WNOHANG` poll, blocking reap and exit status** — a non-blocking poll on a live child
//!    returns `0`; once the child exits, a blocking `waitpid()` returns its PID and the encoded
//!    exit status decodes back to the value the child passed to `_exit()`; a subsequent wait on the
//!    already-reaped child returns `ECHILD`.
//! 4. **Wait-for-any drain** — `wait()` reaps an arbitrary child; repeated calls drain every child
//!    and a final call returns `ECHILD`.
//! 5. **Orphan adoption** — a grandchild whose intermediate parent terminates while the grandchild
//!    is still alive is re-parented onto the init process (the standalone test process itself), so
//!    init can subsequently `waitpid()` for it and collect its exit status. Were the orphan not
//!    adopted, that wait would instead fail with `ECHILD`.
//!
//! Each child blocks on an IPC barrier until the parent releases it, so the parent can observe a
//! live child (for the `WNOHANG` poll) before the child terminates, and the orphan stays alive
//! until after init has reaped its intermediate parent. This turns ordering-dependent behavior into
//! deterministic assertions rather than timing-dependent flakes.
//!
//! The following aspects of the `waitpid()` design are intentionally out of scope here:
//!
//! - The non-standalone deployment gate is a compile-time concern.
//! - Job-control reporting (`WUNTRACED`/`WCONTINUED`) and signal deaths are accepted no-ops.
//! - VM-shutdown propagation on init termination is a daemon-level behavior exercised by the
//!   broader system test suite.

//==================================================================================================
// Imports
//==================================================================================================

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
        ipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
    sys_wait::{
        WNOHANG,
        wexitstatus,
        wifexited,
    },
};
use ::syscall::unistd::bindings;

//==================================================================================================
// Constants
//==================================================================================================

/// Selector passed to `waitpid()`/`wait()` to match any child of the caller.
const WAIT_ANY: pid_t = -1;

/// Options value carrying a bit that `waitpid()` does not support, used to provoke `EINVAL`.
const INVALID_OPTIONS: c_int = 0x100;

/// Exit status used by the child in the `WNOHANG`/blocking-reap scenario.
const CHILD_STATUS_A: c_int = 7;

/// Exit status used by the first child in the wait-for-any scenario.
const CHILD_STATUS_B: c_int = 3;

/// Exit status used by the second child in the wait-for-any scenario.
const CHILD_STATUS_C: c_int = 5;

/// Exit status used by a child whose IPC barrier unexpectedly failed.
const CHILD_FAIL: c_int = 111;

/// Exit status used by the intermediate child in the orphan-adoption scenario. It terminates while
/// its own child (the grandchild) is still alive, leaving the grandchild orphaned.
const ORPHANING_CHILD_STATUS: c_int = 9;

/// Exit status used by the orphaned grandchild in the orphan-adoption scenario, collected by the
/// init process once the grandchild has been adopted.
const ORPHAN_STATUS: c_int = 17;

//==================================================================================================
// Helpers
//==================================================================================================

/// Reads the calling thread's `errno`.
fn read_errno() -> c_int {
    // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
    unsafe { *::syscall::errno::__errno_location() }
}

/// Forks a child that blocks on an IPC barrier until the parent releases it, then terminates with
/// `status`. Returns the child's PID in the parent; the child never returns from this function.
fn spawn_blocked_child(status: c_int) -> Result<ProcessIdentifier, Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        // Child: block until the parent releases us, then terminate with the agreed status.
        if ipc::__kcall_recv().is_err() {
            // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
            unsafe { bindings::_exit::_exit(CHILD_FAIL) };
        }
        // SAFETY: as above; the child terminates here and never returns to the test flow.
        unsafe { bindings::_exit::_exit(status) };
    }

    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    Ok(ProcessIdentifier::from(ret))
}

/// Releases a child created by [`spawn_blocked_child`] by sending it an empty IPC message.
fn release_child(parent: ProcessIdentifier, child: ProcessIdentifier) -> Result<(), Error> {
    let go: Message = Message::new(
        MessageSender::new(parent, ThreadIdentifier::NONE),
        MessageReceiver::new(child, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    ipc::__kcall_send(&go)?;
    Ok(())
}

/// Sends `pid` (encoded as a little-endian `i32` in the first four payload bytes) to `to` over IPC.
/// Used by the intermediate child to hand its grandchild's PID to the init process before exiting,
/// so init can later release and reap the adopted orphan. The kernel validates the sender field, so
/// the message is sent from the caller's own PID.
fn report_pid(to: ProcessIdentifier, pid: ProcessIdentifier) -> Result<(), Error> {
    let from: ProcessIdentifier = pm::getpid_uncached()?;
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0..4].copy_from_slice(&i32::from(pid).to_le_bytes());
    let report: Message = Message::new(
        MessageSender::new(from, ThreadIdentifier::NONE),
        MessageReceiver::new(to, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&report)?;
    Ok(())
}

/// Receives a PID previously sent with [`report_pid`] and decodes it from the message payload.
fn recv_reported_pid() -> Result<ProcessIdentifier, Error> {
    let report: Message = ipc::__kcall_recv()?;
    let mut bytes: [u8; 4] = [0u8; 4];
    bytes.copy_from_slice(&report.payload[0..4]);
    Ok(ProcessIdentifier::from(i32::from_le_bytes(bytes)))
}

//==================================================================================================
// Tests
//==================================================================================================

/// Verifies that waiting with no children fails with `ECHILD`.
fn test_echild_without_children() -> Result<(), Error> {
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let ret: pid_t = unsafe { bindings::waitpid::waitpid(WAIT_ANY, &raw mut status, 0) };
    assert!(ret == -1, "waitpid() with no children must fail (ret={})", ret);

    let errno: c_int = read_errno();
    assert!(
        errno == ErrorCode::NoChildProcess.get(),
        "waitpid() with no children must set ECHILD (errno={})",
        errno
    );
    Ok(())
}

/// Verifies that unsupported `options` bits are rejected with `EINVAL`.
fn test_einval_rejects_bad_options() -> Result<(), Error> {
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let ret: pid_t =
        unsafe { bindings::waitpid::waitpid(WAIT_ANY, &raw mut status, INVALID_OPTIONS) };
    assert!(ret == -1, "waitpid() with invalid options must fail (ret={})", ret);

    let errno: c_int = read_errno();
    assert!(
        errno == ErrorCode::InvalidArgument.get(),
        "waitpid() with invalid options must set EINVAL (errno={})",
        errno
    );
    Ok(())
}

/// Verifies the non-blocking poll, blocking reap, exit-status decoding and post-reap `ECHILD`.
fn test_wnohang_then_reap() -> Result<(), Error> {
    let parent: ProcessIdentifier = pm::getpid_uncached()?;
    let child: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS_A)?;

    // The child is alive (blocked on its barrier): a non-blocking poll must report nothing ready.
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let polled: pid_t =
        unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, WNOHANG) };
    assert!(polled == 0, "WNOHANG poll on a live child must return 0 (ret={})", polled);

    // Release the child so it terminates, then reap it with a blocking wait.
    release_child(parent, child)?;
    // SAFETY: `status` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    assert!(
        reaped == i32::from(child),
        "blocking waitpid() must return the child's PID (ret={}, child={})",
        reaped,
        i32::from(child)
    );
    assert!(wifexited(status), "reaped child must have exited normally (status={:#x})", status);
    assert!(
        wexitstatus(status) == CHILD_STATUS_A,
        "reaped child exit status mismatch (got={}, expected={})",
        wexitstatus(status),
        CHILD_STATUS_A
    );

    // The child has been reaped: a second wait on it must report `ECHILD`.
    // SAFETY: `status` is a valid `c_int`.
    let again: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    assert!(again == -1, "waitpid() on a reaped child must fail (ret={})", again);

    let errno: c_int = read_errno();
    assert!(
        errno == ErrorCode::NoChildProcess.get(),
        "waitpid() on a reaped child must set ECHILD (errno={})",
        errno
    );
    Ok(())
}

/// Verifies that `wait()` reaps any child and that draining all children ends with `ECHILD`.
fn test_wait_any_drains_children() -> Result<(), Error> {
    let parent: ProcessIdentifier = pm::getpid_uncached()?;
    let child_a: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS_B)?;
    let child_b: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS_C)?;

    // Release both children so they terminate.
    release_child(parent, child_a)?;
    release_child(parent, child_b)?;

    // Drain both children through the wait-for-any convenience wrapper. The reaping order is not
    // guaranteed, so the returned PIDs are checked as a set.
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let first: pid_t = unsafe { bindings::wait::wait(&raw mut status) };
    // SAFETY: `status` is a valid `c_int`.
    let second: pid_t = unsafe { bindings::wait::wait(&raw mut status) };

    let expected_a: pid_t = i32::from(child_a);
    let expected_b: pid_t = i32::from(child_b);
    assert!(first > 0, "wait() must return a child PID (ret={})", first);
    assert!(second > 0, "wait() must return a child PID (ret={})", second);
    assert!(
        first != second,
        "wait() must report distinct children (first={}, second={})",
        first,
        second
    );
    assert!(
        (first == expected_a || first == expected_b)
            && (second == expected_a || second == expected_b),
        "wait() returned unexpected PIDs (first={}, second={}, children=[{}, {}])",
        first,
        second,
        expected_a,
        expected_b
    );

    // Every child has been reaped: a further wait must report `ECHILD`.
    // SAFETY: `status` is a valid `c_int`.
    let drained: pid_t = unsafe { bindings::wait::wait(&raw mut status) };
    assert!(drained == -1, "wait() after draining must fail (ret={})", drained);

    let errno: c_int = read_errno();
    assert!(
        errno == ErrorCode::NoChildProcess.get(),
        "wait() after draining must set ECHILD (errno={})",
        errno
    );
    Ok(())
}

/// Verifies that an orphaned process is adopted by the init process.
///
/// Builds a three-generation lineage rooted at the standalone test process, which runs as the init
/// process: the test forks an intermediate child, the intermediate child forks a grandchild that
/// blocks on an IPC barrier, and the intermediate child then terminates while the grandchild is
/// still alive. With its parent gone, the grandchild is an orphan that procd must re-parent onto
/// init. The adoption is observable because init can subsequently release the grandchild and reap
/// it with `waitpid()`, collecting its exit status — an operation only a parent may perform. Were
/// the orphan not adopted, that final `waitpid()` would instead fail with `ECHILD`.
///
/// The IPC barrier makes the ordering deterministic: the grandchild cannot exit until init releases
/// it, and init does not release it until after it has reaped the intermediate child. Reaping the
/// intermediate child guarantees procd has already processed its termination — and therefore the
/// re-parenting — because the grandchild is recorded as the intermediate child's child before the
/// intervening `fork()` returns (the fork-sync handshake), so it is always present in the
/// intermediate child's child list when that child terminates.
fn test_orphan_adopted_by_init() -> Result<(), Error> {
    let init: ProcessIdentifier = pm::getpid_uncached()?;

    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        // Intermediate child: spawn a grandchild that blocks on its barrier, hand its PID to init,
        // then terminate immediately. The grandchild is left alive and blocked, so it becomes an
        // orphan that must be re-parented onto init.
        let grandchild: ProcessIdentifier = match spawn_blocked_child(ORPHAN_STATUS) {
            Ok(grandchild) => grandchild,
            // SAFETY: the child holds no resources requiring cleanup; terminate immediately.
            Err(_) => unsafe { bindings::_exit::_exit(CHILD_FAIL) },
        };
        // Report the grandchild's PID before exiting, so init never blocks awaiting a PID that was
        // never sent.
        if report_pid(init, grandchild).is_err() {
            // SAFETY: as above.
            unsafe { bindings::_exit::_exit(CHILD_FAIL) };
        }
        // SAFETY: as above; exit now to orphan the still-blocked grandchild.
        unsafe { bindings::_exit::_exit(ORPHANING_CHILD_STATUS) };
    }

    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    let child: ProcessIdentifier = ProcessIdentifier::from(ret);

    // Receive the grandchild's PID, reported by the intermediate child before it exits.
    let grandchild: ProcessIdentifier = recv_reported_pid()?;
    assert!(
        grandchild != child && grandchild != init,
        "grandchild PID must be distinct (grandchild={}, child={}, init={})",
        i32::from(grandchild),
        i32::from(child),
        i32::from(init)
    );

    // Reap the intermediate child. By the time this returns, procd has processed its termination
    // and re-parented the still-blocked grandchild onto init.
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    assert!(
        reaped == i32::from(child),
        "waitpid() must reap the intermediate child (ret={}, child={})",
        reaped,
        i32::from(child)
    );
    assert!(
        wifexited(status) && wexitstatus(status) == ORPHANING_CHILD_STATUS,
        "intermediate child exit status mismatch (status={:#x}, expected={})",
        status,
        ORPHANING_CHILD_STATUS
    );

    // Release the now-orphaned grandchild so that it terminates.
    release_child(init, grandchild)?;

    // Reap the adopted orphan. This succeeds only because the orphan was re-parented onto init: the
    // test process never forked it directly. A return of `ECHILD` would mean adoption failed.
    let mut orphan_status: c_int = 0;
    // SAFETY: `orphan_status` is a valid `c_int`.
    let reaped_orphan: pid_t =
        unsafe { bindings::waitpid::waitpid(i32::from(grandchild), &raw mut orphan_status, 0) };
    assert!(
        reaped_orphan == i32::from(grandchild),
        "init must reap the adopted orphan (ret={}, orphan={})",
        reaped_orphan,
        i32::from(grandchild)
    );
    assert!(
        wifexited(orphan_status) && wexitstatus(orphan_status) == ORPHAN_STATUS,
        "adopted orphan exit status mismatch (status={:#x}, expected={})",
        orphan_status,
        ORPHAN_STATUS
    );

    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all `waitpid()` regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("waitpid-rust: starting waitpid() regression tests");

    test_echild_without_children()?;
    ::syslog::info!("waitpid-rust: PASS - echild_without_children");

    test_einval_rejects_bad_options()?;
    ::syslog::info!("waitpid-rust: PASS - einval_rejects_bad_options");

    test_wnohang_then_reap()?;
    ::syslog::info!("waitpid-rust: PASS - wnohang_then_reap");

    test_wait_any_drains_children()?;
    ::syslog::info!("waitpid-rust: PASS - wait_any_drains_children");

    test_orphan_adopted_by_init()?;
    ::syslog::info!("waitpid-rust: PASS - orphan_adopted_by_init");

    Ok(())
}
