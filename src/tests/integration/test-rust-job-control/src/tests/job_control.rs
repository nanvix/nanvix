// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Job-Control Standalone Regression Tests
//!
//! Exercises the POSIX job-control surface end-to-end in standalone mode, verifying that the process
//! manager daemon establishes and reports session, process-group, and controlling-terminal
//! foreground state consistently, and that a signal addressed to a process group reaches every
//! member of that group and no one else.
//!
//! The suite covers:
//!
//! 1. **Self queries.** The boot process is the leader of its own session and process group, so
//!    `getpgrp()`, `getpgid(0)`, and `getsid(0)` all report its own pid.
//! 2. **Query errors.** `getpgid`/`getsid` of an unknown pid fail with `ESRCH`, and a negative pid
//!    fails with `EINVAL`.
//! 3. **Sessions.** A child that is not a process-group leader can `setsid()` into a brand-new
//!    session (becoming session and group leader), after which a second `setsid()` fails with
//!    `EPERM`.
//! 4. **Process groups and group signalling.** A parent moves two children into one process group
//!    (`setpgid`), makes that group the terminal's foreground group (`tcsetpgrp`/`tcgetpgrp`), then
//!    signals the whole group with `kill(-pgid, SIGTERM)`. Both children are terminated while the
//!    parent — which is in a different group — survives.
//!
//! Terminal-generated signal delivery (`^C`/`^Z` → the foreground group) and background-read
//! arbitration (`SIGTTIN`) are driven by host console input, which the automated harness cannot feed
//! deterministically; those paths are covered by the line-discipline and daemon unit tests instead.

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
        ipc as kipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
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
    unistd::STDIN_FILENO,
};
use ::syscall::{
    signal::bindings::kill::kill,
    unistd::bindings,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Exit status reported by a child whose setup (e.g. its readiness handshake) failed before it could
/// run its checks. A fatal signal never produces this status, so it identifies a setup failure
/// rather than a real result.
const CHILD_FAIL: c_int = 111;

/// Base exit status for the `setsid()` child. The child exits with `SETSID_OK` on success or with
/// `SETSID_FAIL_BASE + n` to identify which check failed, so the parent's assertion pinpoints the
/// failing step.
const SETSID_OK: c_int = 0;
const SETSID_FAIL_BASE: c_int = 50;

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
    kipc::__kcall_send(&notification)
}

/// Blocks until the spawned child's readiness notification arrives. Only the freshly spawned child
/// sends to the parent at this point, so any IPC message received here is that notification.
fn await_ready() -> Result<(), Error> {
    let _notification: Message = kipc::__kcall_recv()?;
    Ok(())
}

/// Child entry point for a process that simply parks until a signal terminates it: notify the parent
/// it is alive, then block on a message that never arrives. This function never returns to the
/// caller — the child always terminates here, by a signal.
fn run_blocked_child() -> ! {
    let parent: ProcessIdentifier = match pm::__kcall_getppid() {
        Ok(parent) => parent,
        // SAFETY: the freshly forked child holds no resources requiring cleanup.
        Err(_) => unsafe { bindings::_exit::_exit(CHILD_FAIL) },
    };
    if notify_ready(parent).is_err() {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(CHILD_FAIL) };
    }

    // Block on a message that never arrives; only a fatal signal ends this wait.
    loop {
        let _ = kipc::__kcall_recv();
    }
}

/// Forks a child that parks until signalled and waits for it to report readiness. Returns the
/// child's PID in the parent; the child never returns from this function.
fn spawn_blocked_child() -> Result<ProcessIdentifier, Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        run_blocked_child();
    }
    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    let child: ProcessIdentifier = ProcessIdentifier::from(ret);
    await_ready()?;
    Ok(child)
}

/// Reaps `child` and asserts it was terminated by a signal's default action, which exits the target
/// with `EINTR`.
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

//==================================================================================================
// Tests
//==================================================================================================

/// Verifies that the boot process is the leader of its own session and process group, so the self
/// queries all report its own pid.
fn test_self_session_and_group() -> Result<(), Error> {
    let me: pid_t = i32::from(pm::getpid_uncached()?);

    assert!(bindings::getpgrp::getpgrp() == me, "getpgrp() must report own pid");
    assert!(bindings::getpgid::getpgid(0) == me, "getpgid(0) must report own pid");
    assert!(bindings::getpgid::getpgid(me) == me, "getpgid(self) must report own pid");
    assert!(bindings::getsid::getsid(0) == me, "getsid(0) must report own pid");
    assert!(bindings::getsid::getsid(me) == me, "getsid(self) must report own pid");

    Ok(())
}

/// Verifies the error reporting of the query calls: an unknown pid is `ESRCH` and a negative pid is
/// `EINVAL`.
fn test_query_errors() -> Result<(), Error> {
    assert!(
        bindings::getpgid::getpgid(i32::MAX) == -1
            && read_errno() == ErrorCode::NoSuchProcess.get(),
        "getpgid() of an unknown pid must fail with ESRCH (errno={})",
        read_errno()
    );
    assert!(
        bindings::getsid::getsid(i32::MAX) == -1 && read_errno() == ErrorCode::NoSuchProcess.get(),
        "getsid() of an unknown pid must fail with ESRCH (errno={})",
        read_errno()
    );
    assert!(
        bindings::getpgid::getpgid(-1) == -1 && read_errno() == ErrorCode::InvalidArgument.get(),
        "getpgid() of a negative pid must fail with EINVAL (errno={})",
        read_errno()
    );
    assert!(
        bindings::getsid::getsid(-1) == -1 && read_errno() == ErrorCode::InvalidArgument.get(),
        "getsid() of a negative pid must fail with EINVAL (errno={})",
        read_errno()
    );

    Ok(())
}

/// Child entry point for the session test: a child that is not a process-group leader creates a new
/// session and verifies the resulting state, then confirms a second `setsid()` is rejected. Reports
/// the outcome through its exit status so the parent can assert on it. Never returns.
fn run_setsid_child() -> ! {
    let self_pid: pid_t = match pm::getpid_uncached() {
        Ok(pid) => i32::from(pid),
        // SAFETY: the freshly forked child holds no resources requiring cleanup.
        Err(_) => unsafe { bindings::_exit::_exit(SETSID_FAIL_BASE) },
    };

    // A fresh session is created and led by the caller, so setsid() returns the caller's pid.
    if bindings::setsid::setsid() != self_pid {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(SETSID_FAIL_BASE + 1) };
    }

    // The caller is now the leader of its own session and process group.
    if bindings::getsid::getsid(0) != self_pid {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(SETSID_FAIL_BASE + 2) };
    }
    if bindings::getpgrp::getpgrp() != self_pid {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(SETSID_FAIL_BASE + 3) };
    }

    // A process-group leader cannot start a second new session.
    if bindings::setsid::setsid() != -1 {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(SETSID_FAIL_BASE + 4) };
    }
    if read_errno() != ErrorCode::OperationNotPermitted.get() {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(SETSID_FAIL_BASE + 5) };
    }

    // SAFETY: as above.
    unsafe { bindings::_exit::_exit(SETSID_OK) };
}

/// Verifies that `setsid()` establishes a new session in a child and that a second `setsid()` by the
/// now-leader is rejected. The checks run in the child (which inherits the parent's session and is
/// therefore not a group leader); the parent asserts on the child's exit status.
fn test_setsid_establishes_session() -> Result<(), Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        run_setsid_child();
    }
    assert!(ret > 0, "fork() failed in parent (ret={})", ret);

    let mut status: c_int = 0;
    // SAFETY: `status` points to a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(ret, &raw mut status, 0) };
    assert!(reaped == ret, "waitpid() must reap the setsid child (ret={}, child={})", reaped, ret);
    assert!(wifexited(status), "setsid child must exit normally (status={:#x})", status);
    assert!(
        wexitstatus(status) == SETSID_OK,
        "setsid child reported failure at step {} (0 = success)",
        wexitstatus(status).wrapping_sub(SETSID_FAIL_BASE)
    );

    Ok(())
}

/// Verifies that `setpgid()` builds a process group, that `tcsetpgrp()`/`tcgetpgrp()` track the
/// foreground group, and that `kill(-pgid, SIGTERM)` reaches every member of the group while leaving
/// the caller (in a different group) untouched.
fn test_process_groups_and_group_signal() -> Result<(), Error> {
    let my_pgrp: pid_t = bindings::getpgrp::getpgrp();
    let me: pid_t = i32::from(pm::getpid_uncached()?);

    // Two children, both initially in the caller's process group.
    let c1: ProcessIdentifier = spawn_blocked_child()?;
    let c2: ProcessIdentifier = spawn_blocked_child()?;
    let c1_raw: pid_t = i32::from(c1);
    let c2_raw: pid_t = i32::from(c2);

    // Move C1 into a new process group led by itself.
    assert!(
        bindings::setpgid::setpgid(c1_raw, c1_raw) == 0,
        "setpgid(C1, C1) must succeed (errno={})",
        read_errno()
    );
    assert!(bindings::getpgid::getpgid(c1_raw) == c1_raw, "C1 must lead its own process group");
    // Changing the process group does not change the session.
    assert!(bindings::getsid::getsid(c1_raw) == me, "C1 must remain in the caller's session");

    // Move C2 into C1's process group.
    assert!(
        bindings::setpgid::setpgid(c2_raw, c1_raw) == 0,
        "setpgid(C2, C1) must succeed (errno={})",
        read_errno()
    );
    assert!(bindings::getpgid::getpgid(c2_raw) == c1_raw, "C2 must join C1's process group");

    // Make C1's group the foreground group of the controlling terminal, when one is present.
    let has_tty: bool = bindings::tcgetpgrp::tcgetpgrp(STDIN_FILENO) >= 0;
    if has_tty {
        assert!(
            bindings::tcsetpgrp::tcsetpgrp(STDIN_FILENO, c1_raw) == 0,
            "tcsetpgrp(C1) must succeed (errno={})",
            read_errno()
        );
        assert!(
            bindings::tcgetpgrp::tcgetpgrp(STDIN_FILENO) == c1_raw,
            "the foreground group must be C1"
        );
    }

    // Signal the entire group: both members terminate; the caller (in its own group) survives and
    // proceeds to reap them.
    assert!(
        kill(-c1_raw, as_signum(SIGTERM)?) == 0,
        "kill(-C1, SIGTERM) must succeed (errno={})",
        read_errno()
    );
    reap_killed(c1);
    reap_killed(c2);

    // Restore the foreground group to the caller's group so a now-defunct group is not left in
    // charge of the terminal.
    if has_tty {
        assert!(
            bindings::tcsetpgrp::tcsetpgrp(STDIN_FILENO, my_pgrp) == 0,
            "restoring the foreground group must succeed (errno={})",
            read_errno()
        );
        assert!(
            bindings::tcgetpgrp::tcgetpgrp(STDIN_FILENO) == my_pgrp,
            "the foreground group must be restored to the caller's group"
        );
    }

    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

/// Runs every job-control test.
pub fn run() -> Result<(), Error> {
    test_self_session_and_group()?;
    test_query_errors()?;
    test_setsid_establishes_session()?;
    test_process_groups_and_group_signal()?;
    Ok(())
}
