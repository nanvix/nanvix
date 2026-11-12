// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Thread-Termination Wait Races
//!
//! Exercises the races between a thread blocked in `waitpid()`, the termination of that thread,
//! the termination of the child it waits for, and the deferred reply of the process manager daemon.
//! The kernel publishes one thread-termination event at every live-to-zombie thread transition,
//! ordered after every request that the thread sent before it terminated, and the daemon drops the
//! waits of a terminated thread when the event arrives. The scenarios cover:
//!
//! 1. A waiter interrupted in its wait that exits from its signal handler before its child
//!    terminates, so that only its thread-termination event releases its wait.
//! 2. A child that terminates before the thread that waits for it exits.
//! 3. A wait cancellation that completes before the interrupted waiter exits.
//! 4. A detached waiter that exits from its signal handler and is reaped without a join.
//! 5. A whole-process exit while several threads wait.
//! 6. A fatal signal that terminates a process whose threads all wait, without running user code.
//! 7. More interrupted wait-and-exit cycles than the daemon can hold blocked waits.
//!
//! After each scenario, the children of terminated waiters must remain reapable with their own
//! statuses, and no stale message may be pending for the process.
//!
//! Waiters send the request of a blocking `waitpid()` themselves and check that the daemon blocked
//! the wait before they park or release the child that they wait for, so that no scenario passes
//! without a blocked wait or a deferred reply. A signaller child interrupts a waiter with a signal
//! that it posts every few milliseconds until it is stopped. A signal interrupts a blocked thread
//! only if every thread of the process is blocked when it lands, so the main thread blocks the
//! signal and stays blocked while the waiter waits, which leaves the waiter as the only thread that
//! can take it. A signal that lands before the waiter blocks runs the handler early, which then
//! returns, and a later signal interrupts the waiter once it blocks.

//==================================================================================================
// Imports
//==================================================================================================

use super::support::{
    BlockedWait,
    SCENARIO_PASSED,
    block_signal,
    block_wait,
    check_failed,
    check_no_stale_message,
    ensure,
    exit_process,
    expect_exit,
    expect_no_child,
    finish_child,
    finish_child_while_waiting,
    fork_scenario,
    install_handler,
    join_thread,
    park,
    post_signal,
    read_errno,
    release_child,
    send_byte,
    send_payload,
    spawn_blocked_child,
    spawn_signaller,
    spawn_thread,
    stop_signaller,
    unblock_signal,
    wait_until,
    wait_until_reaped,
};
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::{
    mem::{
        self,
        ManuallyDrop,
    },
    sync::atomic::{
        AtomicBool,
        AtomicI32,
        AtomicU8,
        AtomicUsize,
        Ordering,
    },
};
use ::proc::WaitTarget;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageSender,
    },
    kcall::{
        ipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
        SIGKILL,
        SIGUSR1,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
};
use ::syscall::{
    safe::mem::stack::Stack,
    unistd::bindings,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Signal that interrupts waiting threads.
const INTERRUPT_SIGNAL: usize = SIGUSR1;

/// Selector that waits for any child.
const WAIT_ANY: pid_t = -1;

/// Exit value of a waiter that exits from its signal handler.
const HANDLER_EXIT: usize = 0x3055_0001;

/// Exit value of a waiter whose interrupted wait was cancelled.
const CANCELLED_EXIT: usize = 0x3055_0002;

/// Exit value of a waiter that reaped its child.
const REAPED_EXIT: usize = 0x3055_0003;

/// Exit value of a waiter whose wait the process manager daemon did not block.
const WAIT_NOT_BLOCKED: usize = 0x3055_0004;

/// Exit value of a waiter whose wait ended in an unexpected way.
const WAIT_FAILED: usize = 0x3055_00ff;

/// Notice that a waiter sends to the main thread when it exits from its signal handler.
const EXIT_NOTICE: u8 = 0x5a;

/// Notice that a waiter sends to the main thread when it ends in any other way.
const FAILURE_NOTICE: u8 = 0xa5;

/// Exit status of the child of a single-waiter scenario.
const CHILD_STATUS: c_int = 7;

/// Exit status of the child of the first parked waiter. The other children count up from it.
const PARKED_CHILD_STATUS: c_int = 20;

/// Exit status of a scenario process that exits while its threads wait.
const PROCESS_EXIT_STATUS: c_int = 42;

/// Exit status of a scenario process in which a parked wait ended before the process terminated.
const PARKED_WAIT_ENDED: c_int = 113;

/// Number of threads that wait when their whole process terminates.
const PARKED_WAITERS: usize = 3;

/// State of a parked waiter whose wait is not settled yet.
const PARKED_PENDING: u8 = 0;

/// State of a parked waiter whose wait the process manager daemon blocked.
const PARKED_BLOCKED: u8 = 1;

/// State of a parked waiter whose wait the process manager daemon did not block.
const PARKED_FAILED: u8 = 2;

/// Report status of a scenario process whose waiting threads all hold a blocked wait.
const REPORT_PARKED: u8 = 1;

/// Report status of a scenario process with a waiting thread that holds no blocked wait.
const REPORT_NOT_PARKED: u8 = 2;

/// Size of the status that leads the report of the parked children.
const REPORT_STATUS_SIZE: usize = 1;

/// Size of the process identifier of a parked child in the report.
const PID_SIZE: usize = mem::size_of::<i32>();

/// Number of wait-and-exit cycles to run beyond the blocked-wait capacity of the daemon.
const EXTRA_CYCLES: usize = 8;

// The status and every parked child must fit in the report.
const _: () = assert!(REPORT_STATUS_SIZE + PARKED_WAITERS * PID_SIZE <= Message::PAYLOAD_SIZE);

//==================================================================================================
// Global State
//==================================================================================================

/// Child that the interrupted or reaping waiter waits for.
static WAIT_TARGET: AtomicI32 = AtomicI32::new(0);

/// Children that the parked waiters wait for, indexed by waiter.
static PARKED_TARGETS: [AtomicI32; PARKED_WAITERS] = [const { AtomicI32::new(0) }; PARKED_WAITERS];

/// Wait states of the parked waiters, indexed by waiter.
static PARKED_STATES: [AtomicU8; PARKED_WAITERS] =
    [const { AtomicU8::new(PARKED_PENDING) }; PARKED_WAITERS];

/// Whether the signal handler exits the interrupted thread instead of returning.
static HANDLER_EXITS_THREAD: AtomicBool = AtomicBool::new(false);

/// Whether the interrupted waiter holds a blocked wait, so that the signal handler may exit it.
static WAIT_BLOCKED: AtomicBool = AtomicBool::new(false);

/// Whether the interrupted waiter tells the main thread how it ended.
static NOTIFY_MAIN: AtomicBool = AtomicBool::new(false);

/// Number of times the signal handler ran.
static HANDLER_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Number of waiters that the signal handler exited.
static HANDLER_EXITS: AtomicUsize = AtomicUsize::new(0);

/// Value returned by the last `waitpid()` call that returned in a waiter.
static WAIT_RETURN: AtomicI32 = AtomicI32::new(0);

/// `errno` observed by the last `waitpid()` call that returned in a waiter.
static WAIT_ERRNO: AtomicI32 = AtomicI32::new(0);

/// Exit status that the deferred reply to the reaping waiter reported for its child.
static REAPED_STATUS: AtomicI32 = AtomicI32::new(0);

//==================================================================================================
// Waiters
//==================================================================================================

/// How the signal handler treats the interrupted waiter.
#[derive(Clone, Copy)]
enum HandlerAction {
    /// Exit the waiter from the handler once it holds a blocked wait, so it never cancels the wait.
    ExitThread,
    /// Return, so that the interrupted `waitpid()` cancels its wait.
    Return,
}

/// Handler of the interrupting signal.
extern "C" fn on_interrupt(_signum: c_int) {
    HANDLER_RUNS.fetch_add(1, Ordering::SeqCst);
    // A signal that lands before the waiter holds a blocked wait returns, and a later one exits it.
    if HANDLER_EXITS_THREAD.load(Ordering::SeqCst) && WAIT_BLOCKED.load(Ordering::SeqCst) {
        HANDLER_EXITS.fetch_add(1, Ordering::SeqCst);
        if NOTIFY_MAIN.load(Ordering::SeqCst) {
            let _ = notify_main(EXIT_NOTICE);
        }
        // On success, this does not return: the waiter terminates while the daemon holds its wait.
        let _ = pm::__kcall_exit_thread(HANDLER_EXIT);
    }
}

/// Sends `notice` to the calling process, where the main thread receives it.
fn notify_main(notice: u8) -> Result<(), Error> {
    send_byte(pm::getpid_uncached()?, notice)
}

/// Records the outcome of a `waitpid()` call that returned in a waiter.
fn record_wait(ret: pid_t) {
    WAIT_ERRNO.store(read_errno(), Ordering::SeqCst);
    WAIT_RETURN.store(ret, Ordering::SeqCst);
}

/// Waiter that holds a blocked wait for its child until the signal handler exits it.
extern "C" fn exiting_waiter(_arg: usize) -> usize {
    let exit: usize = wait_until_exited();
    // Reached only if the waiter did not exit from its signal handler.
    if NOTIFY_MAIN.load(Ordering::SeqCst) {
        let _ = notify_main(FAILURE_NOTICE);
    }
    exit
}

/// Blocks a wait for the child of the interrupted waiter and parks the calling thread until the
/// signal handler exits it. Returns only if that fails.
fn wait_until_exited() -> usize {
    if unblock_signal(INTERRUPT_SIGNAL).is_err() {
        return WAIT_FAILED;
    }
    let child: ProcessIdentifier = ProcessIdentifier::from(WAIT_TARGET.load(Ordering::SeqCst));
    let Ok(wait): Result<BlockedWait, Error> = block_wait(WaitTarget::Pid(child)) else {
        return WAIT_NOT_BLOCKED;
    };

    // From here on, the handler exits this thread, so only its thread-termination event can release
    // the wait.
    WAIT_BLOCKED.store(true, Ordering::SeqCst);
    let parked: Result<Message, Error> = park();
    WAIT_BLOCKED.store(false, Ordering::SeqCst);
    drop(wait);
    match parked {
        Ok(_) => ::syslog::error!("blocked wait ended before the waiter exited"),
        Err(error) => ::syslog::error!("waiter failed to park (error={error:?})"),
    }
    WAIT_FAILED
}

/// Waiter that blocks in `waitpid()` until a signal cancels its wait.
extern "C" fn cancelling_waiter(_arg: usize) -> usize {
    if unblock_signal(INTERRUPT_SIGNAL).is_err() {
        return WAIT_FAILED;
    }

    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let ret: pid_t = unsafe {
        bindings::waitpid::waitpid(WAIT_TARGET.load(Ordering::SeqCst), &raw mut status, 0)
    };

    // The child stays blocked, so only a signal can end the wait. `waitpid()` reports the
    // interruption only once it has cancelled a wait that the daemon held, so `EINTR` proves that
    // the wait was blocked.
    record_wait(ret);
    if ret == -1 && WAIT_ERRNO.load(Ordering::SeqCst) == ErrorCode::Interrupted.get() {
        CANCELLED_EXIT
    } else {
        WAIT_FAILED
    }
}

/// Waiter that blocks a wait for its child, then releases the child and collects the deferred reply.
extern "C" fn reaping_waiter(_arg: usize) -> usize {
    let child: ProcessIdentifier = ProcessIdentifier::from(WAIT_TARGET.load(Ordering::SeqCst));
    let Ok(wait): Result<BlockedWait, Error> = block_wait(WaitTarget::Pid(child)) else {
        return WAIT_NOT_BLOCKED;
    };
    // The child terminates only once released, so the daemon defers the reply to the blocked wait.
    if release_child(child).is_err() {
        return WAIT_FAILED;
    }
    match wait.complete() {
        Ok((reaped, status)) if reaped == child => {
            REAPED_STATUS.store(status, Ordering::SeqCst);
            REAPED_EXIT
        },
        _ => WAIT_FAILED,
    }
}

/// Waiter that holds a blocked wait until its whole process terminates.
extern "C" fn parked_waiter(index: usize) -> usize {
    let (Some(target), Some(state)) = (PARKED_TARGETS.get(index), PARKED_STATES.get(index)) else {
        return WAIT_FAILED;
    };
    let child: ProcessIdentifier = ProcessIdentifier::from(target.load(Ordering::SeqCst));
    let Ok(_wait): Result<BlockedWait, Error> = block_wait(WaitTarget::Pid(child)) else {
        state.store(PARKED_FAILED, Ordering::SeqCst);
        return WAIT_NOT_BLOCKED;
    };
    state.store(PARKED_BLOCKED, Ordering::SeqCst);

    // The child stays blocked, so the wait must last until the process terminates.
    let _ = park();
    ::syslog::error!("parked wait ended before its process terminated (index={index})");
    let _ = report_parked(REPORT_NOT_PARKED);
    exit_process(PARKED_WAIT_ENDED)
}

/// Prepares the calling process for an interrupted waiter that waits for `child`, with the handler
/// taking `action`, and forks the signaller child that interrupts the waiter.
fn arm_interrupted_waiter(
    child: ProcessIdentifier,
    action: HandlerAction,
) -> Result<ProcessIdentifier, Error> {
    WAIT_TARGET.store(i32::from(child), Ordering::SeqCst);
    HANDLER_EXITS_THREAD.store(matches!(action, HandlerAction::ExitThread), Ordering::SeqCst);
    WAIT_BLOCKED.store(false, Ordering::SeqCst);
    NOTIFY_MAIN.store(false, Ordering::SeqCst);
    HANDLER_RUNS.store(0, Ordering::SeqCst);
    HANDLER_EXITS.store(0, Ordering::SeqCst);
    install_handler(INTERRUPT_SIGNAL, on_interrupt)?;
    // The waiter inherits this mask and unblocks the signal itself.
    block_signal(INTERRUPT_SIGNAL)?;
    // The signaller posts the signal as soon as it runs, so the process must catch it first.
    spawn_signaller(INTERRUPT_SIGNAL)
}

//==================================================================================================
// Parked Waiters
//==================================================================================================

/// Returns the exit status of the child of the parked waiter at `index`.
fn parked_child_status(index: usize) -> Result<c_int, Error> {
    c_int::try_from(index)
        .ok()
        .and_then(|offset| PARKED_CHILD_STATUS.checked_add(offset))
        .ok_or_else(|| check_failed("parked child status overflow"))
}

/// Forks a blocked child for every parked waiter and parks a waiter on each child. Returns once
/// every waiter settled, and fails unless the process manager daemon blocked every wait.
fn park_waiters() -> Result<(), Error> {
    for (index, target) in PARKED_TARGETS.iter().enumerate() {
        let child: ProcessIdentifier = spawn_blocked_child(parked_child_status(index)?)?;
        target.store(i32::from(child), Ordering::SeqCst);
    }

    for index in 0..PARKED_WAITERS {
        // The waiters terminate with their process, so their stacks are never given back.
        let stack: ManuallyDrop<Stack> = ManuallyDrop::new(Stack::new(USER_THREAD_STACK_SIZE)?);
        let _waiter: ThreadIdentifier = spawn_thread(&stack, parked_waiter, index)?;
    }

    wait_until(
        || {
            PARKED_STATES
                .iter()
                .all(|state| state.load(Ordering::SeqCst) != PARKED_PENDING)
        },
        "parked waiters did not settle",
    )?;
    ensure(
        PARKED_STATES
            .iter()
            .all(|state| state.load(Ordering::SeqCst) == PARKED_BLOCKED),
        "process manager daemon did not block a parked wait",
    )
}

/// Returns the report status that matches the outcome of parking the waiting threads.
fn parked_status<T>(parked: &Result<T, Error>) -> u8 {
    if parked.is_ok() {
        REPORT_PARKED
    } else {
        REPORT_NOT_PARKED
    }
}

/// Reports `status` and the children of the parked waiters to the parent, which waits for the
/// report before it checks the scenario. The scenario reports even when it fails to park its
/// waiters, so the parent never waits for a report that does not come.
fn report_parked(status: u8) -> Result<(), Error> {
    let parent: ProcessIdentifier = pm::__kcall_getppid()?;
    let mut report: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    report[0] = status;
    for (index, target) in PARKED_TARGETS.iter().enumerate() {
        let offset: usize = REPORT_STATUS_SIZE + index * PID_SIZE;
        report[offset..offset + PID_SIZE]
            .copy_from_slice(&target.load(Ordering::SeqCst).to_le_bytes());
    }
    send_payload(parent, report)
}

/// Receives the children reported by the scenario process `scenario`, which must have parked every
/// waiting thread on a blocked wait.
fn receive_parked_children(
    scenario: ProcessIdentifier,
) -> Result<[ProcessIdentifier; PARKED_WAITERS], Error> {
    let report: Message = ipc::__kcall_recv()?;
    let source: MessageSender = { report.source };
    ensure(source.pid == scenario, "children were reported by an unexpected process")?;
    ensure(report.payload[0] == REPORT_PARKED, "scenario process did not park its waiters")?;

    let mut children: [ProcessIdentifier; PARKED_WAITERS] =
        [ProcessIdentifier::from(0); PARKED_WAITERS];
    for (index, child) in children.iter_mut().enumerate() {
        let offset: usize = REPORT_STATUS_SIZE + index * PID_SIZE;
        let mut bytes: [u8; PID_SIZE] = [0u8; PID_SIZE];
        bytes.copy_from_slice(&report.payload[offset..offset + PID_SIZE]);
        *child = ProcessIdentifier::from(i32::from_le_bytes(bytes));
    }
    Ok(children)
}

/// Releases and reaps the children that the test process adopted from a terminated scenario
/// process. Each child must exit with its own status, so no terminated waiter consumed it.
fn finish_adopted_children(children: &[ProcessIdentifier; PARKED_WAITERS]) -> Result<(), Error> {
    for (index, child) in children.iter().enumerate() {
        finish_child(*child, parked_child_status(index)?)?;
    }
    expect_no_child(WAIT_ANY)?;
    check_no_stale_message()
}

/// Scenario that parks several waiters and then exits the whole process.
fn exit_while_threads_wait() -> Result<(), Error> {
    let parked: Result<(), Error> = park_waiters();
    report_parked(parked_status(&parked))?;
    parked?;
    exit_process(PROCESS_EXIT_STATUS)
}

/// Scenario that parks every thread, the main one included, on a blocked wait until a fatal signal
/// terminates the process.
fn wait_on_every_thread() -> Result<(), Error> {
    let parked: Result<BlockedWait, Error> =
        park_waiters().and_then(|()| block_wait(WaitTarget::Any));
    report_parked(parked_status(&parked))?;
    let _wait: BlockedWait = parked?;
    let _ = park();
    Err(check_failed("fatal signal did not terminate the process"))
}

//==================================================================================================
// Scenarios
//==================================================================================================

/// Verifies that a waiter interrupted in its wait may exit before its child terminates. The waiter
/// exits from its signal handler and never cancels its wait, so only its thread-termination event
/// releases the wait, and the child must remain reapable.
fn interrupted_waiter_exits_before_child() -> Result<(), Error> {
    let child: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS)?;
    let signaller: ProcessIdentifier = arm_interrupted_waiter(child, HandlerAction::ExitThread)?;

    // The join keeps the main thread blocked while the waiter waits.
    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let exit: usize = join_thread(spawn_thread(&stack, exiting_waiter, 0)?)?;
    let stopped: Result<(), Error> = stop_signaller(signaller);
    ensure(exit == HANDLER_EXIT, "waiter did not exit from its signal handler")?;
    ensure(HANDLER_EXITS.load(Ordering::SeqCst) == 1, "handler did not exit the waiter once")?;
    stopped?;

    finish_child(child, CHILD_STATUS)?;
    check_no_stale_message()
}

/// Verifies that a child may terminate before the thread that waits for it exits. The waiter blocks
/// its wait before it releases the child, so the daemon defers the reply, and the later
/// thread-termination event of the waiter must not disturb the child that it reaped.
fn child_terminates_before_waiter_exits() -> Result<(), Error> {
    let child: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS)?;
    WAIT_TARGET.store(i32::from(child), Ordering::SeqCst);

    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let exit: usize = join_thread(spawn_thread(&stack, reaping_waiter, 0)?)?;
    ensure(exit == REAPED_EXIT, "waiter did not reap its child")?;
    ensure(
        REAPED_STATUS.load(Ordering::SeqCst) == CHILD_STATUS,
        "waiter observed a wrong child status",
    )?;

    // The waiter reaped the child, so it cannot be reaped again.
    expect_no_child(i32::from(child))?;
    check_no_stale_message()
}

/// Verifies that a wait cancellation may complete before the interrupted waiter exits. The handler
/// returns, so `waitpid()` cancels the wait and fails with `EINTR`, and the cancelled wait must not
/// consume the child.
fn cancelled_wait_precedes_waiter_exit() -> Result<(), Error> {
    let child: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS)?;
    let signaller: ProcessIdentifier = arm_interrupted_waiter(child, HandlerAction::Return)?;

    // The join keeps the main thread blocked while the waiter waits.
    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let exit: usize = join_thread(spawn_thread(&stack, cancelling_waiter, 0)?)?;
    let stopped: Result<(), Error> = stop_signaller(signaller);
    if exit != CANCELLED_EXIT {
        ::syslog::error!(
            "interrupted wait did not fail with EINTR (exit={exit:#x}, return={}, errno={})",
            WAIT_RETURN.load(Ordering::SeqCst),
            WAIT_ERRNO.load(Ordering::SeqCst)
        );
        return Err(check_failed("interrupted wait did not fail with EINTR"));
    }
    ensure(HANDLER_RUNS.load(Ordering::SeqCst) >= 1, "waiter was not interrupted")?;
    stopped?;

    finish_child(child, CHILD_STATUS)?;
    check_no_stale_message()
}

/// Verifies that a detached waiter interrupted in its wait may exit and be reaped without a join.
/// Its thread-termination event must still release its wait, and the child must remain reapable.
fn detached_waiter_exits() -> Result<(), Error> {
    let child: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS)?;
    let signaller: ProcessIdentifier = arm_interrupted_waiter(child, HandlerAction::ExitThread)?;
    NOTIFY_MAIN.store(true, Ordering::SeqCst);

    // A detached thread may outlive the handle of its stack, so the stack is never given back.
    let stack: ManuallyDrop<Stack> = ManuallyDrop::new(Stack::new(USER_THREAD_STACK_SIZE)?);
    let waiter: ThreadIdentifier = spawn_thread(&stack, exiting_waiter, 0)?;
    pm::__kcall_detach_thread(waiter)?;

    // Receiving the notice of the waiter keeps the main thread blocked while the waiter waits.
    let notice: u8 = receive_notice()?;
    wait_until_reaped(waiter)?;
    let stopped: Result<(), Error> = stop_signaller(signaller);
    ensure(notice == EXIT_NOTICE, "detached waiter did not exit from its signal handler")?;
    ensure(HANDLER_EXITS.load(Ordering::SeqCst) == 1, "handler did not exit the waiter once")?;
    stopped?;

    finish_child(child, CHILD_STATUS)?;
    check_no_stale_message()
}

/// Receives the notice that tells how the interrupted waiter ended.
fn receive_notice() -> Result<u8, Error> {
    let message: Message = ipc::__kcall_recv()?;
    let source: MessageSender = { message.source };
    ensure(source.pid == pm::getpid_uncached()?, "notice came from another process")?;
    Ok(message.payload[0])
}

/// Verifies that more waiters than the daemon can hold blocked may each be interrupted in their
/// wait and exit before the child terminates. Each wait must be released when its thread
/// terminates: otherwise, the daemon rejects a wait with `ENOBUFS` once terminated waiters hold
/// every blocked-wait slot, so a later waiter fails to block its wait, and the main thread cannot
/// block in `waitpid()` either.
fn interrupted_waits_exceed_waiter_capacity() -> Result<(), Error> {
    let child: ProcessIdentifier = spawn_blocked_child(CHILD_STATUS)?;
    let signaller: ProcessIdentifier = arm_interrupted_waiter(child, HandlerAction::ExitThread)?;

    // Waiters run one at a time, so they share one stack.
    let stack: Stack = Stack::new(USER_THREAD_STACK_SIZE)?;
    let cycles: usize = ::proc::MAX_BLOCKED_WAITERS + EXTRA_CYCLES;
    let cycled: Result<(), Error> = run_waiter_cycles(&stack, cycles);
    let stopped: Result<(), Error> = stop_signaller(signaller);
    cycled?;
    stopped?;
    ensure(HANDLER_EXITS.load(Ordering::SeqCst) == cycles, "handler did not exit every waiter")?;

    finish_child_while_waiting(child, CHILD_STATUS)?;
    check_no_stale_message()
}

/// Runs `cycles` interrupted waiters on `stack`, one after another. Each one must exit from the
/// signal handler while it holds a blocked wait.
fn run_waiter_cycles(stack: &Stack, cycles: usize) -> Result<(), Error> {
    for cycle in 0..cycles {
        // The previous waiter exited while it held a blocked wait, so the flag must be cleared
        // before the next waiter can take a signal.
        WAIT_BLOCKED.store(false, Ordering::SeqCst);
        let exit: usize = join_thread(spawn_thread(stack, exiting_waiter, cycle)?)?;
        if exit != HANDLER_EXIT {
            ::syslog::error!(
                "waiter did not exit from its handler (cycle={cycle}, exit={exit:#x})"
            );
            return Err(check_failed("interrupted waiter did not exit from its handler"));
        }
    }
    Ok(())
}

/// Verifies that a process may exit while several of its threads wait. Its children must be adopted
/// by the test process and remain reapable with their own statuses.
fn process_exit_with_waiting_threads() -> Result<(), Error> {
    let scenario: ProcessIdentifier = fork_scenario(exit_while_threads_wait)?;
    let children: [ProcessIdentifier; PARKED_WAITERS] = receive_parked_children(scenario)?;
    expect_exit(scenario, PROCESS_EXIT_STATUS)?;
    finish_adopted_children(&children)
}

/// Verifies that a fatal signal may terminate a process while all of its threads wait. No handler
/// runs, the kernel terminates every thread, and the children must remain reapable with their own
/// statuses.
fn fatal_signal_terminates_waiting_threads() -> Result<(), Error> {
    let scenario: ProcessIdentifier = fork_scenario(wait_on_every_thread)?;
    let children: [ProcessIdentifier; PARKED_WAITERS] = receive_parked_children(scenario)?;
    post_signal(scenario, SIGKILL)?;
    // The kernel assigns `EINTR` to a process that a signal terminates.
    expect_exit(scenario, ErrorCode::Interrupted.get())?;
    finish_adopted_children(&children)
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs `scenario` in its own process and checks that it passes.
fn run_scenario(name: &str, scenario: fn() -> Result<(), Error>) -> Result<(), Error> {
    let process: ProcessIdentifier = fork_scenario(scenario)?;
    expect_exit(process, SCENARIO_PASSED)?;
    ::syslog::info!("test-rust-thread-wait: PASS - {name}");
    Ok(())
}

/// Runs all thread-termination wait tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-rust-thread-wait: starting thread-termination wait tests");

    run_scenario("interrupted_waiter_exits_before_child", interrupted_waiter_exits_before_child)?;
    run_scenario("child_terminates_before_waiter_exits", child_terminates_before_waiter_exits)?;
    run_scenario("cancelled_wait_precedes_waiter_exit", cancelled_wait_precedes_waiter_exit)?;
    run_scenario("detached_waiter_exits", detached_waiter_exits)?;

    process_exit_with_waiting_threads()?;
    ::syslog::info!("test-rust-thread-wait: PASS - process_exit_with_waiting_threads");

    fatal_signal_terminates_waiting_threads()?;
    ::syslog::info!("test-rust-thread-wait: PASS - fatal_signal_terminates_waiting_threads");

    run_scenario(
        "interrupted_waits_exceed_waiter_capacity",
        interrupted_waits_exceed_waiter_capacity,
    )?;

    Ok(())
}
