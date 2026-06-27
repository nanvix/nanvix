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
//! - `SIGKILL`'s unconditional short-circuit and signal masking are validated separately. Caught
//!   (handler) dispositions are exercised by a self-directed delivery test at the end of this
//!   suite, which redirects the caller through a handler and back via `sigreturn()`.
//! - The kernel's transient already-interrupted process state is covered by in-kernel unit tests,
//!   as it is not deterministically observable from user space.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ptr,
    sync::atomic::{
        AtomicBool,
        AtomicUsize,
        Ordering,
    },
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
        ipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
        SA_RESTART,
        SA_SIGINFO,
        SIG_BLOCK,
        SIG_MAX,
        SIG_SETMASK,
        SIGKILL,
        SIGTERM,
        SIGUSR1,
        SIGUSR2,
        SigAction,
        SigSet,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        pid_t,
        time_t,
    },
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

/// Duration, in seconds, of the timed sleeps that park a thread in an interruptible wait. It is
/// comfortably longer than the interrupting signal needs to arrive (so the sleep is always
/// interrupted mid-wait rather than completing on its own), yet bounded so that a regression in
/// signal interruption stalls the suite for only this long instead of an hour.
const INTERRUPTIBLE_SLEEP_SECS: time_t = 10;

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
            // Comfortably longer than the test needs to deliver the signal, so the child is always
            // terminated mid-wait rather than waking on its own, yet bounded so a regression does
            // not stall the suite for an hour.
            let req: timespec = timespec {
                tv_sec: INTERRUPTIBLE_SLEEP_SECS,
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
// Caught-Signal Delivery
//==================================================================================================

/// Records whether the `SIGUSR1` handler ran.
static HANDLER_RAN: AtomicBool = AtomicBool::new(false);
/// Records whether the `SA_SIGINFO` handler ran.
static SIGINFO_HANDLER_RAN: AtomicBool = AtomicBool::new(false);
/// Records the signal number observed by the `SA_SIGINFO` handler.
static SIGINFO_SIGNUM: AtomicUsize = AtomicUsize::new(0);
/// Records the `si_signo` value observed through the `SA_SIGINFO` siginfo pointer.
static SIGINFO_SI_SIGNO: AtomicUsize = AtomicUsize::new(0);
/// Records whether the `SA_SIGINFO` pointers were non-null.
static SIGINFO_POINTERS_VALID: AtomicBool = AtomicBool::new(false);

/// Handler installed for `SIGUSR1`; records its invocation so the test can confirm that
/// asynchronous delivery redirected the thread here and then resumed the interrupted code.
extern "C" fn sigusr1_handler(_signum: c_int) {
    HANDLER_RAN.store(true, Ordering::SeqCst);
}

/// Three-argument handler installed with `SA_SIGINFO`.
extern "C" fn sigusr2_siginfo_handler(signum: c_int, info: *const u32, ctx: *const u32) {
    if let Ok(signum) = usize::try_from(signum) {
        SIGINFO_SIGNUM.store(signum, Ordering::SeqCst);
    }
    SIGINFO_POINTERS_VALID.store(!info.is_null() && !ctx.is_null(), Ordering::SeqCst);
    if !info.is_null() {
        // SAFETY: the kernel supplies `info` as a pointer to the frame's embedded siginfo image
        // for the duration of the handler.
        let si_signo: u32 = unsafe { ::core::ptr::read_volatile(info) };
        if let Ok(si_signo) = usize::try_from(si_signo) {
            SIGINFO_SI_SIGNO.store(si_signo, Ordering::SeqCst);
        }
    }
    SIGINFO_HANDLER_RAN.store(true, Ordering::SeqCst);
}

/// Returns the address of [`sigusr1_handler`] for the `sa_handler` slot. Forming a pointer-sized
/// value from a function item is exactly what the `<signal.h>` handler slot expects.
#[allow(clippy::as_conversions)]
fn sigusr1_handler_addr() -> usize {
    sigusr1_handler as *const () as usize
}

/// Returns the address of [`sigusr2_siginfo_handler`] for the `sa_sigaction` slot.
#[allow(clippy::as_conversions)]
fn sigusr2_siginfo_handler_addr() -> usize {
    sigusr2_siginfo_handler as *const () as usize
}

/// Verifies that a caught signal posted to the calling process runs its registered handler at the
/// kernel-call return boundary and then resumes the interrupted code with the handler's effects
/// visible. This exercises the full asynchronous-delivery path end-to-end: signal-frame build,
/// handler invocation, the restorer trampoline, and `sigreturn()` context restoration.
fn test_caught_signal_runs_handler() -> Result<(), Error> {
    HANDLER_RAN.store(false, Ordering::SeqCst);

    // Install a catching disposition for SIGUSR1.
    let act: SigAction = SigAction {
        sa_handler: sigusr1_handler_addr(),
        sa_mask: 0,
        sa_flags: 0,
        sa_sigaction: 0,
    };
    // SAFETY: `act` is a valid, properly aligned `SigAction`; the old-action pointer is null.
    unsafe { pm::__kcall_sigaction(as_signum(SIGUSR1)?, &raw const act, ptr::null_mut()) }?;

    // Post SIGUSR1 to ourselves. The signal is delivered at the boundary where this `kill()`
    // returns, so the handler has run by the time control returns to user space.
    let caller: ProcessIdentifier = pm::getpid_uncached()?;
    post_signal(caller, as_signum(SIGUSR1)?);

    assert!(
        HANDLER_RAN.load(Ordering::SeqCst),
        "SIGUSR1 handler did not run after a self-directed kill()"
    );
    Ok(())
}

/// Verifies that `SA_SIGINFO` dispatch uses the three-argument handler slot and passes the signal
/// number through both the first argument and the embedded siginfo image.
fn test_siginfo_signal_runs_three_arg_handler() -> Result<(), Error> {
    SIGINFO_HANDLER_RAN.store(false, Ordering::SeqCst);
    SIGINFO_SIGNUM.store(0, Ordering::SeqCst);
    SIGINFO_SI_SIGNO.store(0, Ordering::SeqCst);
    SIGINFO_POINTERS_VALID.store(false, Ordering::SeqCst);

    let act: SigAction = SigAction {
        sa_handler: 0,
        sa_mask: 0,
        sa_flags: SA_SIGINFO,
        sa_sigaction: sigusr2_siginfo_handler_addr(),
    };
    // SAFETY: `act` is a valid, properly aligned `SigAction`; the old-action pointer is null.
    unsafe { pm::__kcall_sigaction(as_signum(SIGUSR2)?, &raw const act, ptr::null_mut()) }?;

    let caller: ProcessIdentifier = pm::getpid_uncached()?;
    post_signal(caller, as_signum(SIGUSR2)?);

    assert!(
        SIGINFO_HANDLER_RAN.load(Ordering::SeqCst),
        "SIGUSR2 SA_SIGINFO handler did not run after a self-directed kill()"
    );
    assert!(
        SIGINFO_POINTERS_VALID.load(Ordering::SeqCst),
        "SIGUSR2 SA_SIGINFO handler did not receive non-null info/context pointers"
    );
    assert_eq!(SIGINFO_SIGNUM.load(Ordering::SeqCst), SIGUSR2);
    assert_eq!(SIGINFO_SI_SIGNO.load(Ordering::SeqCst), SIGUSR2);
    Ok(())
}

//==================================================================================================
// Blocking-Call Interruption
//==================================================================================================

/// Exit status reported by a signaller child that completed its job (posted the signal) successfully.
const SIGNALLER_OK: c_int = 0;

/// Sends an empty notification from the calling process to `to` over IPC.
fn send_empty(to: ProcessIdentifier) -> Result<(), Error> {
    let from: ProcessIdentifier = pm::getpid_uncached()?;
    let message: Message = Message::new(
        MessageSender::new(from, ThreadIdentifier::NONE),
        MessageReceiver::new(to, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );
    ipc::__kcall_send(&message)
}

/// Returns whether a blocking IPC receive reported interruption (`EINTR`).
fn is_eintr_err(result: &Result<Message, Error>) -> bool {
    matches!(result, Err(error) if error.code.get() == ErrorCode::Interrupted.get())
}

/// Installs a catching disposition for `SIGUSR1` with the given `sa_flags`.
fn install_sigusr1_handler(sa_flags: c_int) -> Result<(), Error> {
    let act: SigAction = SigAction {
        sa_handler: sigusr1_handler_addr(),
        sa_mask: 0,
        sa_flags,
        sa_sigaction: 0,
    };
    // SAFETY: `act` is a valid, properly aligned `SigAction`; the old-action pointer is null.
    unsafe { pm::__kcall_sigaction(as_signum(SIGUSR1)?, &raw const act, ptr::null_mut()) }
}

/// Signaller child entry point: wait for the parent's go-ahead, then post `SIGUSR1` to the parent so
/// it interrupts the parent's in-progress blocking call. The cross-process `kill()` is relayed
/// through the process-manager daemon, whose several context switches let the parent reach its
/// blocking point first. This function never returns to the caller.
fn run_signaller_child() -> ! {
    let parent: ProcessIdentifier = match pm::__kcall_getppid() {
        Ok(parent) => parent,
        // SAFETY: the freshly forked child holds no resources requiring cleanup.
        Err(_) => unsafe { bindings::_exit::_exit(CHILD_FAIL) },
    };
    if await_ready().is_err() {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(CHILD_FAIL) };
    }
    let signum: c_int = match as_signum(SIGUSR1) {
        Ok(signum) => signum,
        // SAFETY: as above.
        Err(_) => unsafe { bindings::_exit::_exit(CHILD_FAIL) },
    };
    if kill(i32::from(parent), signum) != 0 {
        // SAFETY: as above.
        unsafe { bindings::_exit::_exit(CHILD_FAIL) };
    }
    // SAFETY: as above.
    unsafe { bindings::_exit::_exit(SIGNALLER_OK) };
}

/// Forks a signaller child. Returns the child's PID in the parent; the child never returns here.
fn spawn_signaller() -> Result<ProcessIdentifier, Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        run_signaller_child();
    }
    assert!(ret > 0, "fork() failed in parent (ret={})", ret);
    Ok(ProcessIdentifier::from(ret))
}

/// Reaps a signaller child and asserts it completed successfully.
fn reap_signaller(child: ProcessIdentifier) {
    let mut status: c_int = 0;
    // SAFETY: `status` points to a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    assert!(
        reaped == i32::from(child),
        "waitpid() must reap the signaller child (ret={}, child={})",
        reaped,
        i32::from(child)
    );
    assert!(
        wifexited(status),
        "signaller child must surface as a normal exit (status={:#x})",
        status
    );
    assert!(
        wexitstatus(status) == SIGNALLER_OK,
        "signaller child failed to post the signal (status={})",
        wexitstatus(status)
    );
}

/// Verifies that a deliverable caught signal interrupts a thread blocked in `recv()`, returning
/// `EINTR` and running the handler, when the handler is installed without `SA_RESTART`.
fn test_eintr_interrupts_recv() -> Result<(), Error> {
    HANDLER_RAN.store(false, Ordering::SeqCst);
    install_sigusr1_handler(0)?;

    let signaller: ProcessIdentifier = spawn_signaller()?;
    // Release the signaller, then block. The relayed cross-process signal arrives once we are parked
    // in recv().
    send_empty(signaller)?;
    let result: Result<Message, Error> = ipc::__kcall_recv();

    assert!(
        HANDLER_RAN.load(Ordering::SeqCst),
        "SIGUSR1 handler did not run after interrupting a blocked recv()"
    );
    assert!(is_eintr_err(&result), "recv() must report EINTR when interrupted by a caught signal");
    reap_signaller(signaller);
    Ok(())
}

/// Verifies that a deliverable caught signal interrupts a thread blocked in a timed sleep, returning
/// `EINTR` and running the handler, when the handler is installed without `SA_RESTART`.
fn test_eintr_interrupts_sleep() -> Result<(), Error> {
    HANDLER_RAN.store(false, Ordering::SeqCst);
    install_sigusr1_handler(0)?;

    let signaller: ProcessIdentifier = spawn_signaller()?;
    send_empty(signaller)?;
    // Comfortably longer than the signal needs to arrive, so the sleep is always interrupted
    // mid-wait rather than completing on its own, yet bounded so a regression does not stall the
    // suite for an hour.
    let req: timespec = timespec {
        tv_sec: INTERRUPTIBLE_SLEEP_SECS,
        tv_nsec: 0,
    };
    // SAFETY: `req` is a valid `timespec`; passing a null `rem` is permitted.
    let ret: c_int = unsafe { nanosleep(&raw const req, ptr::null_mut()) };

    assert!(
        HANDLER_RAN.load(Ordering::SeqCst),
        "SIGUSR1 handler did not run after interrupting a blocked sleep"
    );
    assert!(ret == -1, "nanosleep() must report interruption (ret={})", ret);
    assert!(
        read_errno() == ErrorCode::Interrupted.get(),
        "nanosleep() must fail with EINTR (errno={})",
        read_errno()
    );
    reap_signaller(signaller);
    Ok(())
}

/// Verifies that an `SA_RESTART` handler transparently restarts a blocked sleep instead of reporting
/// `EINTR`: the sleep is interrupted, the handler runs, and the call re-executes with its original
/// arguments to completion. The sleep duration travels in the kernel call's second argument
/// register, so a faithful restart must also restore that register.
fn test_sa_restart_restarts_sleep() -> Result<(), Error> {
    HANDLER_RAN.store(false, Ordering::SeqCst);
    install_sigusr1_handler(SA_RESTART)?;

    let signaller: ProcessIdentifier = spawn_signaller()?;
    send_empty(signaller)?;
    // A sub-second sleep whose duration is carried entirely in the nanoseconds field (the call's
    // second argument). The signal arrives well within it; the handler runs and the call restarts
    // for the full duration, so a correct restart completes successfully rather than failing.
    let req: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 900_000_000,
    };
    // SAFETY: `req` is a valid `timespec`; passing a null `rem` is permitted.
    let ret: c_int = unsafe { nanosleep(&raw const req, ptr::null_mut()) };

    assert!(
        HANDLER_RAN.load(Ordering::SeqCst),
        "SIGUSR1 handler did not run during an SA_RESTART sleep"
    );
    assert!(
        ret == 0,
        "SA_RESTART nanosleep() must complete after restarting (ret={}, errno={})",
        ret,
        read_errno()
    );
    reap_signaller(signaller);
    Ok(())
}

/// Verifies that `sigsuspend()` blocks with the supplied mask installed, is interrupted by a caught
/// signal whose handler runs, and then reports `EINTR`.
fn test_sigsuspend_returns_after_handler() -> Result<(), Error> {
    HANDLER_RAN.store(false, Ordering::SeqCst);
    install_sigusr1_handler(0)?;

    let signaller: ProcessIdentifier = spawn_signaller()?;
    send_empty(signaller)?;
    // Suspend with an empty mask so SIGUSR1 remains deliverable.
    let mask: SigSet = 0;
    // SAFETY: `mask` points to a valid `SigSet` for the duration of the call.
    let result: Result<(), Error> = unsafe { pm::__kcall_sigsuspend(&raw const mask) };

    assert!(HANDLER_RAN.load(Ordering::SeqCst), "SIGUSR1 handler did not run during sigsuspend()");
    assert!(
        matches!(&result, Err(error) if error.code.get() == ErrorCode::Interrupted.get()),
        "sigsuspend() must report EINTR after a handler runs"
    );
    reap_signaller(signaller);
    Ok(())
}

/// Verifies that `sigsuspend()` immediately delivers a signal that was already pending and becomes
/// unblocked under the temporary mask, instead of sleeping until a later signal arrives.
fn test_sigsuspend_delivers_pending_unblocked_signal() -> Result<(), Error> {
    HANDLER_RAN.store(false, Ordering::SeqCst);
    install_sigusr1_handler(0)?;

    let bit: SigSet = 1u64 << (SIGUSR1 - 1);
    let block: SigSet = bit;
    let mut old: SigSet = 0;
    // Block SIGUSR1 so the self-directed signal below becomes pending.
    // SAFETY: `block` and `old` point to valid `SigSet` values.
    unsafe { pm::__kcall_sigprocmask(SIG_BLOCK, &raw const block, &raw mut old) }?;

    let caller: ProcessIdentifier = pm::getpid_uncached()?;
    post_signal(caller, as_signum(SIGUSR1)?);
    assert!(!HANDLER_RAN.load(Ordering::SeqCst), "a blocked signal must stay pending");

    let suspend_mask: SigSet = old & !bit;
    // SAFETY: `suspend_mask` points to a valid `SigSet` for the duration of the call.
    let result: Result<(), Error> = unsafe { pm::__kcall_sigsuspend(&raw const suspend_mask) };

    assert!(
        HANDLER_RAN.load(Ordering::SeqCst),
        "sigsuspend() did not deliver the already-pending SIGUSR1"
    );
    assert!(
        matches!(&result, Err(error) if error.code.get() == ErrorCode::Interrupted.get()),
        "sigsuspend() must report EINTR after delivering a pending signal"
    );

    let mut current: SigSet = 0;
    // SAFETY: `current` points to a valid `SigSet`; a null `set` requests only the old mask.
    unsafe { pm::__kcall_sigprocmask(SIG_SETMASK, ptr::null(), &raw mut current) }?;
    assert!(
        current & bit != 0,
        "sigsuspend() must restore the pre-suspend blocked mask (current={:#x})",
        current
    );

    // Restore the mask that was in effect before this test blocked SIGUSR1.
    // SAFETY: `old` points to a valid `SigSet`; the old-mask output is not requested.
    unsafe { pm::__kcall_sigprocmask(SIG_SETMASK, &raw const old, ptr::null_mut()) }?;
    Ok(())
}

/// Verifies that `sigpending()` reports a signal that is posted while blocked, that the blocked
/// signal is not delivered, and that unblocking it delivers it and clears it from the pending set.
fn test_sigpending_reports_blocked_pending() -> Result<(), Error> {
    HANDLER_RAN.store(false, Ordering::SeqCst);
    install_sigusr1_handler(0)?;

    let bit: SigSet = 1u64 << (SIGUSR1 - 1);
    let block: SigSet = bit;
    let mut old: SigSet = 0;
    // Block SIGUSR1 so a self-directed post stays pending instead of being delivered.
    // SAFETY: `block` and `old` point to valid `SigSet` values.
    unsafe { pm::__kcall_sigprocmask(SIG_BLOCK, &raw const block, &raw mut old) }?;

    // Post SIGUSR1 to ourselves; it is blocked, so it must remain pending and undelivered.
    let caller: ProcessIdentifier = pm::getpid_uncached()?;
    post_signal(caller, as_signum(SIGUSR1)?);
    assert!(!HANDLER_RAN.load(Ordering::SeqCst), "a blocked signal must not be delivered");

    // sigpending() must report the pending-but-blocked signal.
    let mut pending: SigSet = 0;
    // SAFETY: `pending` points to a valid `SigSet`.
    unsafe { pm::__kcall_sigpending(&raw mut pending) }?;
    assert!(
        pending & bit != 0,
        "sigpending() must report the pending-but-blocked SIGUSR1 (pending={:#x})",
        pending
    );

    // Unblocking delivers the pending signal at this call's return-to-user boundary.
    // SAFETY: `old` points to a valid `SigSet`.
    unsafe { pm::__kcall_sigprocmask(SIG_SETMASK, &raw const old, ptr::null_mut()) }?;
    assert!(HANDLER_RAN.load(Ordering::SeqCst), "unblocking a pending signal must deliver it");

    // It must no longer be pending.
    let mut pending_after: SigSet = 0;
    // SAFETY: `pending_after` points to a valid `SigSet`.
    unsafe { pm::__kcall_sigpending(&raw mut pending_after) }?;
    assert!(
        pending_after & bit == 0,
        "a delivered signal must clear from the pending set (pending={:#x})",
        pending_after
    );
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
    test_caught_signal_runs_handler()?;
    test_siginfo_signal_runs_three_arg_handler()?;
    test_eintr_interrupts_recv()?;
    test_eintr_interrupts_sleep()?;
    test_sa_restart_restarts_sleep()?;
    test_sigsuspend_returns_after_handler()?;
    test_sigsuspend_delivers_pending_unblocked_signal()?;
    test_sigpending_reports_blocked_pending()?;
    Ok(())
}
