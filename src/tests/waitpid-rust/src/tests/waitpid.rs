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
//!
//! Each child blocks on an IPC barrier until the parent releases it, so the parent can observe a
//! live child (for the `WNOHANG` poll) before the child terminates. This turns ordering-dependent
//! behavior into deterministic assertions rather than timing-dependent flakes.
//!
//! The following aspects of the `waitpid()` design are intentionally out of scope here:
//!
//! - The non-standalone deployment gate is a compile-time concern.
//! - Job-control reporting (`WUNTRACED`/`WCONTINUED`) and signal deaths are accepted no-ops.
//! - Orphan re-parenting and VM-shutdown propagation are daemon-level behaviors exercised by the
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
    pm::ProcessIdentifier,
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
        MessageSender::from(parent),
        MessageReceiver::from(child),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    ipc::__kcall_send(&go)?;
    Ok(())
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
    let parent: ProcessIdentifier = pm::__kcall_getpid()?;
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
    let parent: ProcessIdentifier = pm::__kcall_getpid()?;
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

    Ok(())
}
