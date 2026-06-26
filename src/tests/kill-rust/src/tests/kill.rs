// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `kill()` Standalone Regression Tests
//!
//! Exercises the POSIX `kill()` library call end-to-end in standalone mode, verifying that posting
//! a fatal signal whose default action is *terminate* (`SIGTERM`) tears down the target process
//! regardless of the scheduling state it is in when the signal arrives. The three states covered
//! mirror the kernel process-state lists that the in-kernel termination path walks:
//!
//! 1. **Sleeping** — the target is blocked indefinitely in a kernel wait (`recv()` with no pending
//!    alarm). The signal must wake a candidate thread and terminate the process.
//! 2. **Running** — the target is runnable and actively consuming CPU in a tight loop (it never
//!    voluntarily blocks). The signal must terminate it without it ever returning to user space.
//! 3. **Interrupted** — the target is blocked in an *interruptible* timed wait (`nanosleep()` with
//!    a pending alarm). The fatal signal must interrupt the in-progress timed wait and terminate
//!    the process rather than letting it sleep to completion.
//!
//! Each scenario follows the same deterministic protocol:
//!
//! - The parent forks a child, which notifies the parent over IPC the moment before it enters its
//!   wait, then enters that wait. Because the cross-process `kill()` is relayed through the
//!   process-manager daemon (several context switches), the child has reached its target state by
//!   the time the signal is posted. The preemptive scheduler guarantees a CPU-bound child is still
//!   descheduled so the daemon can service the request.
//! - The parent posts `SIGTERM` and then reaps the child with `waitpid()`. A correctly terminated
//!   child is reaped and its status decodes to a normal exit carrying `EINTR` — the status the
//!   kernel assigns to a process killed by a signal's default action. A child that is *not*
//!   terminated would instead block forever (turning the parent's `waitpid()` into a
//!   timeout-detected failure) or wake from its wait and exit with a sentinel status, which the
//!   parent's assertions reject. Either way a missed kill is reported as a failure rather than a
//!   silent pass.
//!
//! The following aspects of the signals design are intentionally out of scope here:
//!
//! - The non-standalone deployment gate is a compile-time concern.
//! - `SIGKILL`'s unconditional short-circuit, signal masking, and caught (handler) dispositions
//!   are validated separately; this suite focuses on the default-termination posting path.
//! - The kernel's transient already-interrupted process state is covered by in-kernel unit tests,
//!   as it is not deterministically observable from user space.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ptr;
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
        SIG_MAX,
        SIGKILL,
        SIGTERM,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
    sys_wait::{
        wexitstatus,
        wifexited,
    },
    time::timespec,
};
use ::syscall::{
    signal::bindings::kill::kill,
    time::bindings::nanosleep::nanosleep,
    unistd::bindings,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Exit status reported by a child whose readiness handshake with the parent failed. A fatal signal
/// never produces this status, so observing it identifies a setup failure rather than a successful
/// kill.
const CHILD_FAIL: c_int = 111;

/// Exit status reported by a child that returned from its blocking point instead of being
/// terminated by the signal. Surfacing a distinct status turns a missed kill into a loud assertion
/// failure rather than a silent pass.
const CHILD_NOT_KILLED: c_int = 112;

//==================================================================================================
// Wait Selection
//==================================================================================================

/// Selects the scheduling state a spawned child parks in before it is signalled.
#[derive(Clone, Copy)]
enum ChildWait {
    /// Block indefinitely in `recv()` — a kernel wait with no pending alarm (the *sleeping* state).
    Recv,
    /// Spin in a CPU-bound loop — runnable and never voluntarily blocking (the *running* state).
    Spin,
    /// Block in a long timed `nanosleep()` — an interruptible wait with a pending alarm (the
    /// *interrupted* state, where the signal interrupts the in-progress timed wait).
    TimedSleep,
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Reads the calling thread's `errno`.
fn read_errno() -> c_int {
    // SAFETY: `__errno_location()` returns a valid pointer to the thread-local `errno`.
    unsafe { *::syscall::errno::__errno_location() }
}

/// Narrows a signal number to the C `int` expected by `kill()`, rejecting values that do not fit.
fn as_signum(signum: usize) -> Result<c_int, Error> {
    c_int::try_from(signum)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "signal number out of range"))
}

/// Sends an empty readiness notification from the calling child to `parent` over IPC.
fn notify_ready(parent: ProcessIdentifier) -> Result<(), Error> {
    let from: ProcessIdentifier = pm::getpid_uncached()?;
    let notification: Message = Message::new(
        MessageSender::new(from, ThreadIdentifier::NONE),
        MessageReceiver::new(parent, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    ipc::__kcall_send(&notification)
}

/// Blocks until the spawned child's readiness notification arrives. Only the freshly spawned child
/// sends to the parent at this point, so any IPC message received here is that notification.
fn await_ready() -> Result<(), Error> {
    let _notification: Message = ipc::__kcall_recv()?;
    Ok(())
}

/// Posts `signum` to `child` through the POSIX `kill()` C binding, asserting it reports success.
fn post_signal(child: ProcessIdentifier, signum: c_int) {
    let ret: c_int = kill(i32::from(child), signum);
    assert!(
        ret == 0,
        "kill(pid={}, sig={}) must succeed (ret={}, errno={})",
        i32::from(child),
        signum,
        ret,
        read_errno()
    );
}

/// Posts `signum` to raw process identifier `pid`, asserting that `kill()` fails with
/// `expected_errno`.
fn expect_kill_error(pid: pid_t, signum: c_int, expected_errno: ErrorCode) {
    let ret: c_int = kill(pid, signum);
    assert!(
        ret == -1,
        "kill(pid={}, sig={}) must fail (ret={}, errno={})",
        pid,
        signum,
        ret,
        read_errno()
    );
    assert!(
        read_errno() == expected_errno.get(),
        "kill(pid={}, sig={}) failed with errno={} (expected={})",
        pid,
        signum,
        read_errno(),
        expected_errno.get()
    );
}

/// Reaps `child` and asserts it was terminated by the signal's default action, which exits the
/// target with `EINTR`.
fn reap_killed(child: ProcessIdentifier) {
    let mut status: c_int = 0;
    // SAFETY: `status` points to a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    assert!(
        reaped == i32::from(child),
        "waitpid() must reap the killed child (ret={}, child={})",
        reaped,
        i32::from(child)
    );
    assert!(wifexited(status), "killed child must surface as a normal exit (status={:#x})", status);
    assert!(
        wexitstatus(status) == ErrorCode::Interrupted.get(),
        "killed child must exit with EINTR (got={}, expected={})",
        wexitstatus(status),
        ErrorCode::Interrupted.get()
    );
}

/// Child entry point: notify the parent that the wait is about to begin, then enter the requested
/// wait. Only a fatal signal is expected to end the wait; if the wait ever returns on its own, the
/// child exits with a sentinel so the parent's assertions fail loudly. This function never returns
/// to the caller — the child always terminates here.
fn run_child(wait: ChildWait) -> ! {
    let parent: ProcessIdentifier = match pm::__kcall_getppid() {
        Ok(parent) => parent,
        // SAFETY: the freshly forked child holds no resources requiring cleanup.
        Err(_) => unsafe { bindings::_exit::_exit(CHILD_FAIL) },
    };
    if notify_ready(parent).is_err() {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(CHILD_FAIL) };
    }

    match wait {
        ChildWait::Recv => {
            // Block on a message that never arrives; only a fatal signal ends this wait.
            let _ = ipc::__kcall_recv();
        },
        ChildWait::Spin => {
            // Consume CPU until the signal terminates us; the preemptive scheduler still lets the
            // process-manager daemon run and service the kill.
            loop {
                ::core::hint::spin_loop();
            }
        },
        ChildWait::TimedSleep => {
            // One hour — far longer than the test needs to deliver the signal, so the child is
            // always terminated mid-wait rather than waking on its own.
            let req: timespec = timespec {
                tv_sec: 3600,
                tv_nsec: 0,
            };
            // SAFETY: `req` is a valid `timespec`; passing a null `rem` is permitted.
            let _ = unsafe { nanosleep(&raw const req, ptr::null_mut()) };
        },
    }

    // Reached only if an interruptible wait returned without the process being terminated.
    // SAFETY: the child holds no resources requiring cleanup.
    unsafe { bindings::_exit::_exit(CHILD_NOT_KILLED) };
}

/// Forks a child that parks in the requested `wait`. Returns the child's PID in the parent; the
/// child never returns from this function.
fn spawn_child(wait: ChildWait) -> Result<ProcessIdentifier, Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        run_child(wait);
    }
    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    Ok(ProcessIdentifier::from(ret))
}

/// Spawns a child in the requested `wait`, waits for it to reach that state, posts `signum`, and
/// asserts the child is terminated by the signal's default action.
fn kill_child_in_state(wait: ChildWait, signum: c_int) -> Result<(), Error> {
    let child: ProcessIdentifier = spawn_child(wait)?;
    await_ready()?;
    post_signal(child, signum);
    reap_killed(child);
    Ok(())
}

//==================================================================================================
// Tests
//==================================================================================================

/// Verifies that `SIGTERM` terminates a process blocked indefinitely in a kernel wait.
fn test_kill_sleeping_process() -> Result<(), Error> {
    kill_child_in_state(ChildWait::Recv, as_signum(SIGTERM)?)
}

/// Verifies that `SIGTERM` terminates a process that is runnable and actively consuming CPU.
fn test_kill_running_process() -> Result<(), Error> {
    kill_child_in_state(ChildWait::Spin, as_signum(SIGTERM)?)
}

/// Verifies that `SIGTERM` interrupts an in-progress timed wait and terminates the process.
fn test_kill_interrupted_process() -> Result<(), Error> {
    kill_child_in_state(ChildWait::TimedSleep, as_signum(SIGTERM)?)
}

/// Verifies that `SIGKILL` terminates through the unconditional fatal-signal path.
fn test_sigkill_terminates_sleeping_process() -> Result<(), Error> {
    kill_child_in_state(ChildWait::Recv, as_signum(SIGKILL)?)
}

/// Verifies that signal zero probes for permission and existence without killing the target.
fn test_kill_zero_signal_only_probes() -> Result<(), Error> {
    let child: ProcessIdentifier = spawn_child(ChildWait::Recv)?;
    await_ready()?;
    post_signal(child, 0);
    post_signal(child, as_signum(SIGTERM)?);
    reap_killed(child);
    Ok(())
}

/// Verifies that unsupported process-group selectors are rejected.
fn test_kill_rejects_negative_pid() -> Result<(), Error> {
    expect_kill_error(-1, as_signum(SIGTERM)?, ErrorCode::InvalidArgument);
    Ok(())
}

/// Verifies that a positive PID with no live target is rejected.
fn test_kill_rejects_unknown_pid() -> Result<(), Error> {
    expect_kill_error(i32::MAX, 0, ErrorCode::NoSuchProcess);
    Ok(())
}

/// Verifies that signal numbers outside the supported range are rejected.
fn test_kill_rejects_invalid_signal() -> Result<(), Error> {
    let caller: ProcessIdentifier = pm::getpid_uncached()?;
    expect_kill_error(i32::from(caller), as_signum(SIG_MAX + 1)?, ErrorCode::InvalidArgument);
    Ok(())
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs every `kill()` test.
pub fn run() -> Result<(), Error> {
    test_kill_sleeping_process()?;
    test_kill_running_process()?;
    test_kill_interrupted_process()?;
    test_sigkill_terminates_sleeping_process()?;
    test_kill_zero_signal_only_probes()?;
    test_kill_rejects_negative_pid()?;
    test_kill_rejects_unknown_pid()?;
    test_kill_rejects_invalid_signal()?;
    Ok(())
}
