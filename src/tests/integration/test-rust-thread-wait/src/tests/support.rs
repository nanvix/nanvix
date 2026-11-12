// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Test Support
//!
//! Process, thread, signal, and wait fixtures for the thread-termination wait tests. Every scenario
//! runs in a process forked from the test process, so that the signal state it leaves behind and
//! the threads it abandons cannot leak into another scenario.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ptr,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
    time::Duration,
};
use ::proc::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    WaitResponseMessage,
    WaitTarget,
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
        RequestIdentifier,
        RequestToken,
        SystemMessage,
        SystemMessageHeader,
    },
    kcall::{
        ipc,
        pm,
        sched,
    },
    pm::{
        ProcessIdentifier,
        SIG_BLOCK,
        SIG_UNBLOCK,
        SIGUSR2,
        SigAction,
        SigSet,
        ThreadCreateArgs,
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
use ::syscall::{
    safe::mem::stack::Stack,
    signal::bindings::kill::kill,
    unistd::bindings,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Exit status of a child whose own setup failed, which no scenario expects.
pub const CHILD_FAIL: c_int = 111;

/// Exit status of a scenario process that completed every check.
pub const SCENARIO_PASSED: c_int = 0;

/// Exit status of a scenario process that failed a check.
const SCENARIO_FAILED: c_int = 112;

/// Exit status of a signaller child that was stopped.
pub const SIGNALLER_OK: c_int = 0;

/// Signal that stops a signaller child.
const STOP_SIGNAL: usize = SIGUSR2;

/// Delay between two interruptions that a signaller child posts.
const SIGNAL_PERIOD: Duration = Duration::from_millis(5);

/// Number of scheduling rounds to wait for another thread to make progress.
const POLL_ATTEMPTS: usize = 10_000;

/// Marker carried by the message that checks for stale deliveries.
const SENTINEL: u8 = 0x5e;

//==================================================================================================
// Global State
//==================================================================================================

/// Whether the parent asked this signaller child to stop.
static SIGNALLER_STOPPED: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Process Fixtures
//==================================================================================================

/// Reads the calling thread's `errno`.
pub fn read_errno() -> c_int {
    // SAFETY: `__errno_location()` returns a valid pointer to `errno`.
    unsafe { *::syscall::errno::__errno_location() }
}

/// Builds the error reported by a failed check.
pub fn check_failed(reason: &'static str) -> Error {
    ::syslog::error!("check failed: {reason}");
    Error::new(ErrorCode::OperationNotPermitted, reason)
}

/// Fails with `reason` unless `condition` holds.
pub fn ensure(condition: bool, reason: &'static str) -> Result<(), Error> {
    if condition {
        Ok(())
    } else {
        Err(check_failed(reason))
    }
}

/// Terminates the calling process with `status`.
pub fn exit_process(status: c_int) -> ! {
    // SAFETY: test processes hold no resources that need cleanup before termination.
    unsafe { bindings::_exit::_exit(status) }
}

/// Converts a `fork()` return value in the parent into the identifier of the child.
fn forked_child(ret: pid_t) -> Result<ProcessIdentifier, Error> {
    if ret > 0 {
        Ok(ProcessIdentifier::from(ret))
    } else {
        Err(check_failed("fork() failed"))
    }
}

/// Forks a scenario process that runs `scenario` and exits with [`SCENARIO_PASSED`] when it
/// succeeds. Returns the identifier of the scenario process to the caller.
pub fn fork_scenario(scenario: fn() -> Result<(), Error>) -> Result<ProcessIdentifier, Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        let status: c_int = match scenario() {
            Ok(()) => SCENARIO_PASSED,
            Err(error) => {
                ::syslog::error!("scenario failed (error={error:?})");
                SCENARIO_FAILED
            },
        };
        exit_process(status);
    }
    forked_child(ret)
}

/// Sends a message whose first payload byte is `byte` to `to`.
pub fn send_byte(to: ProcessIdentifier, byte: u8) -> Result<(), Error> {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0] = byte;
    send_payload(to, payload)
}

/// Sends `payload` to `to`. The kernel stamps the real sender into the message.
pub fn send_payload(
    to: ProcessIdentifier,
    payload: [u8; Message::PAYLOAD_SIZE],
) -> Result<(), Error> {
    let from: ProcessIdentifier = pm::getpid_uncached()?;
    let message: Message = Message::new(
        MessageSender::new(from, ThreadIdentifier::NONE),
        MessageReceiver::new(to, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&message)
}

/// Forks a child that blocks until it is released and then exits with `status`.
pub fn spawn_blocked_child(status: c_int) -> Result<ProcessIdentifier, Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        if ipc::__kcall_recv().is_err() {
            exit_process(CHILD_FAIL);
        }
        exit_process(status);
    }
    forked_child(ret)
}

/// Releases a child created by [`spawn_blocked_child`].
pub fn release_child(child: ProcessIdentifier) -> Result<(), Error> {
    send_byte(child, 0)
}

/// Waits for `child` with `waitpid()` and returns its raw wait status.
pub fn reap(child: ProcessIdentifier) -> Result<c_int, Error> {
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let ret: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    if ret != i32::from(child) {
        ::syslog::error!(
            "waitpid() did not reap the child (child={child:?}, ret={ret}, errno={})",
            read_errno()
        );
        return Err(check_failed("waitpid() did not reap the child"));
    }
    Ok(status)
}

/// Reaps `child` and checks that it exited with `expected`.
pub fn expect_exit(child: ProcessIdentifier, expected: c_int) -> Result<(), Error> {
    let status: c_int = reap(child)?;
    if !wifexited(status) || wexitstatus(status) != expected {
        ::syslog::error!(
            "child exited with an unexpected status (child={child:?}, status={status:#x}, \
             expected={expected})"
        );
        return Err(check_failed("child exited with an unexpected status"));
    }
    Ok(())
}

/// Releases `child` and checks that it is still reapable with its own `status`.
pub fn finish_child(child: ProcessIdentifier, status: c_int) -> Result<(), Error> {
    release_child(child)?;
    expect_exit(child, status)
}

/// Checks that the calling thread can still block a wait for `child`, which requires a free
/// blocked-wait slot in the process manager daemon, and that the deferred reply to that wait reaps
/// `child` with exit status `status`. The child is released only once the wait is blocked, so the
/// reply is always deferred.
pub fn finish_child_while_waiting(child: ProcessIdentifier, status: c_int) -> Result<(), Error> {
    let wait: BlockedWait = block_wait(WaitTarget::Pid(child))?;
    release_child(child)?;
    let (reaped, reaped_status): (ProcessIdentifier, c_int) = wait.complete()?;
    ensure(
        reaped == child && reaped_status == status,
        "deferred wait reply reported a wrong child or status",
    )
}

/// Checks that `waitpid()` reports `ECHILD` for `pid`.
pub fn expect_no_child(pid: pid_t) -> Result<(), Error> {
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let ret: pid_t = unsafe { bindings::waitpid::waitpid(pid, &raw mut status, WNOHANG) };
    ensure(
        ret == -1 && read_errno() == ErrorCode::NoChildProcess.get(),
        "waitpid() found a child that should not exist",
    )
}

/// Checks that no stale message is pending for the calling process: a sentinel that the process
/// sends to itself must be the next message it receives.
pub fn check_no_stale_message() -> Result<(), Error> {
    let pid: ProcessIdentifier = pm::getpid_uncached()?;
    send_byte(pid, SENTINEL)?;
    let message: Message = ipc::__kcall_recv()?;
    let source: MessageSender = { message.source };
    ensure(
        message.message_type == MessageType::Ipc
            && source.pid == pid
            && message.payload[0] == SENTINEL,
        "a stale message was delivered to the process",
    )
}

/// Posts `signum` to `pid` with `kill()`.
pub fn post_signal(pid: ProcessIdentifier, signum: usize) -> Result<(), Error> {
    let signum: c_int = c_int::try_from(signum)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "signal number out of range"))?;
    ensure(kill(i32::from(pid), signum) == 0, "kill() failed")
}

//==================================================================================================
// Blocked Waits
//==================================================================================================

/// Wait that the process manager daemon holds blocked for the calling thread.
///
/// A thread that terminates while it holds a blocked wait leaves the wait to its thread-termination
/// event. Thread exit releases the request state of the thread, so the token need not be dropped.
pub struct BlockedWait {
    /// Token that correlates the wait request with its reply.
    token: RequestToken,
}

impl BlockedWait {
    /// Receives the deferred reply that completes the wait, and returns the reaped child and the
    /// exit status that it reported. The calling thread must expect no other message.
    pub fn complete(self) -> Result<(ProcessIdentifier, c_int), Error> {
        let reply: Message = ipc::__kcall_recv()?;
        let source: MessageSender = { reply.source };
        ensure(
            reply.message_type == MessageType::Ipc
                && source.pid == ProcessIdentifier::PROCD
                && RequestIdentifier::read_from(&reply) == self.token.identifier(),
            "received a message that does not complete the wait",
        )?;
        let system: SystemMessage = SystemMessage::from_bytes(reply.payload)?;
        ensure(
            matches!(system.header, SystemMessageHeader::ProcessManagement),
            "wait reply is not a process management message",
        )?;
        let message: ProcessManagementMessage =
            ProcessManagementMessage::from_bytes(system.payload)?;
        ensure(
            matches!(message.header, ProcessManagementMessageHeader::WaitResponse),
            "wait reply is not a wait response",
        )?;

        let response: WaitResponseMessage = WaitResponseMessage::from_bytes(message.payload);
        let (child, status, error): (ProcessIdentifier, c_int, c_int) =
            (response.child, response.status, response.error);
        if error != 0 {
            ::syslog::error!("process manager daemon failed the wait (error={error})");
            return Err(check_failed("process manager daemon failed the wait"));
        }
        Ok((child, status))
    }
}

/// Sends the request that a blocking `waitpid()` sends for `target`, and checks that the process
/// manager daemon blocked it. Unlike `waitpid()`, the calling thread does not block, so it knows
/// that the daemon holds the wait before it parks.
///
/// The daemon serves requests in the order that they were sent. Once it answers a later null-signal
/// probe, it has either blocked the wait or answered it, and a reply to the wait was set aside for
/// this thread while the probe was pending.
pub fn block_wait(target: WaitTarget) -> Result<BlockedWait, Error> {
    let caller: ProcessIdentifier = pm::getpid_uncached()?;
    let token: RequestToken =
        RequestToken::allocate(pm::__kcall_gettid()?, ProcessIdentifier::PROCD)?;
    let mut request: Message = ::proc::wait_request(caller, target, 0)?;
    token.identifier().write_to(&mut request);
    ipc::__kcall_send(&request)?;

    ::proc::kill(caller, 0)?;
    if token.take_stashed()?.is_some() {
        return Err(check_failed("process manager daemon answered a wait that should block"));
    }
    Ok(BlockedWait { token })
}

/// Blocks the calling thread until it receives a message. A thread that holds a blocked wait
/// receives none, so this returns only if the wait ended or the receive failed.
pub fn park() -> Result<Message, Error> {
    ipc::__kcall_recv()
}

//==================================================================================================
// Signaller Child
//==================================================================================================

/// Handler of the signal that stops a signaller child.
extern "C" fn on_stop(_signum: c_int) {
    SIGNALLER_STOPPED.store(true, Ordering::SeqCst);
}

/// Posts `signum` to the parent every [`SIGNAL_PERIOD`] until the parent stops it, and then exits.
///
/// The signaller never receives a message, so no message can reach it while `kill()` waits for its
/// reply and drops every other message. The process manager daemon relays each signal. A signal
/// interrupts a blocked thread only if every thread of the parent is blocked when it lands, and
/// lands at the next return to user mode of a thread that does not block it otherwise, so repeating
/// it eventually interrupts a thread that blocks after an earlier signal landed.
fn run_signaller(signum: usize) -> ! {
    if install_handler(STOP_SIGNAL, on_stop).is_err() {
        exit_process(CHILD_FAIL);
    }
    let parent: ProcessIdentifier = match pm::__kcall_getppid() {
        Ok(parent) => parent,
        Err(_) => exit_process(CHILD_FAIL),
    };
    while !SIGNALLER_STOPPED.load(Ordering::SeqCst) {
        if post_signal(parent, signum).is_err() {
            exit_process(CHILD_FAIL);
        }
        match pm::__kcall_sleep(SIGNAL_PERIOD) {
            Ok(()) => {},
            // The stop signal interrupts the sleep.
            Err(error) if error.code == ErrorCode::Interrupted => {},
            Err(_) => exit_process(CHILD_FAIL),
        }
    }
    exit_process(SIGNALLER_OK)
}

/// Forks a signaller child that posts `signum` to the caller until [`stop_signaller`] stops it. The
/// caller must catch or block `signum` first.
pub fn spawn_signaller(signum: usize) -> Result<ProcessIdentifier, Error> {
    let ret: pid_t = bindings::fork::fork();
    if ret == 0 {
        run_signaller(signum);
    }
    forked_child(ret)
}

/// Stops a signaller child and reaps it.
pub fn stop_signaller(signaller: ProcessIdentifier) -> Result<(), Error> {
    post_signal(signaller, STOP_SIGNAL)?;
    expect_exit(signaller, SIGNALLER_OK)
}

//==================================================================================================
// Signal Fixtures
//==================================================================================================

/// Returns the signal set that contains only `signum`.
fn signal_set(signum: usize) -> SigSet {
    1 << (signum - 1)
}

/// Converts a signal number into the type expected by the signal kernel calls.
fn kcall_signum(signum: usize) -> Result<i32, Error> {
    i32::try_from(signum)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "signal number out of range"))
}

/// Returns the address of a signal handler for the `sa_handler` slot.
#[allow(clippy::as_conversions)]
fn handler_address(handler: extern "C" fn(c_int)) -> usize {
    handler as *const () as usize
}

/// Installs `handler` for `signum`, without `SA_RESTART`, so an interrupted call reports `EINTR`.
pub fn install_handler(signum: usize, handler: extern "C" fn(c_int)) -> Result<(), Error> {
    let action: SigAction = SigAction {
        sa_handler: handler_address(handler),
        sa_mask: 0,
        sa_flags: 0,
        sa_sigaction: 0,
    };
    // SAFETY: `action` is a valid `SigAction` and the old-action pointer is null.
    unsafe { pm::__kcall_sigaction(kcall_signum(signum)?, &raw const action, ptr::null_mut()) }
}

/// Blocks `signum` in the calling thread. Threads created afterwards inherit the blocked mask.
pub fn block_signal(signum: usize) -> Result<(), Error> {
    let set: SigSet = signal_set(signum);
    // SAFETY: `set` is a valid signal set and the old-set pointer is null.
    unsafe { pm::__kcall_sigprocmask(SIG_BLOCK, &raw const set, ptr::null_mut()) }
}

/// Unblocks `signum` in the calling thread.
pub fn unblock_signal(signum: usize) -> Result<(), Error> {
    let set: SigSet = signal_set(signum);
    // SAFETY: `set` is a valid signal set and the old-set pointer is null.
    unsafe { pm::__kcall_sigprocmask(SIG_UNBLOCK, &raw const set, ptr::null_mut()) }
}

//==================================================================================================
// Thread Fixtures
//==================================================================================================

/// Returns the address of a thread entry point for the thread-creation kernel call.
#[allow(clippy::as_conversions)]
fn entry_address(entry: extern "C" fn(usize) -> usize) -> usize {
    entry as *const () as usize
}

/// Creates a thread that runs `entry(arg)` on `stack`. The caller keeps `stack` alive until the
/// thread is joined; a detached thread must never give its stack back.
pub fn spawn_thread(
    stack: &Stack,
    entry: extern "C" fn(usize) -> usize,
    arg: usize,
) -> Result<ThreadIdentifier, Error> {
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: entry_address(entry),
        user_fn_arg1: arg,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    };
    pm::__kcall_create_thread(&mut args)
}

/// Joins `tid` and returns its exit value.
pub fn join_thread(tid: ThreadIdentifier) -> Result<usize, Error> {
    let mut retval: usize = 0;
    pm::__kcall_join_thread(tid, &mut retval)?;
    Ok(retval)
}

/// Waits until the detached thread `tid` has exited and the kernel has reaped it.
pub fn wait_until_reaped(tid: ThreadIdentifier) -> Result<(), Error> {
    for _ in 0..POLL_ATTEMPTS {
        let mut retval: usize = 0;
        match pm::__kcall_join_thread(tid, &mut retval) {
            // A detached thread cannot be joined while it is alive.
            Err(error) if error.code == ErrorCode::InvalidArgument => sched::__kcall_sched_yield()?,
            // A reaped thread no longer exists.
            Err(error) if error.code == ErrorCode::NoSuchProcess => return Ok(()),
            _ => return Err(check_failed("detached thread was unexpectedly joinable")),
        }
    }
    Err(check_failed("detached thread was not reaped"))
}

/// Yields the processor until `condition` holds.
pub fn wait_until(condition: impl Fn() -> bool, reason: &'static str) -> Result<(), Error> {
    for _ in 0..POLL_ATTEMPTS {
        if condition() {
            return Ok(());
        }
        sched::__kcall_sched_yield()?;
    }
    Err(check_failed(reason))
}
