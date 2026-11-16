// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Peer-Exit Rendezvous Regression Tests
//!
//! Exercises the exit of a process while another thread is blocked in a rendezvous `push()` or
//! `pull()` whose peer is the exiting process.
//!
//! `ProcessManager::do_exit()` takes the exiting process out of the running slot *before* it
//! releases the rendezvous state that the process owned. A blocked counterpart must be released
//! from that window without going through a path that assumes a running process: waking it with
//! `do_wakeup()` dereferences the (empty) running slot and panics the kernel. The kernel instead
//! *interrupts* the counterpart, so its blocked call fails with [`ErrorCode::OperationTimedOut`]
//! and the kernel carries on.
//!
//! Every test forks a child that exits without ever taking part in the rendezvous, while the parent
//! is blocked, with no timeout, on the other half of it. Each test then checks that:
//!
//! 1. the blocked call returns [`ErrorCode::OperationTimedOut`] once the child exits, and
//! 2. the parent can reap the child and carry on, which it could not do if the kernel had panicked.
//!
//! The tests are:
//!
//! - [`test_peer_exit_interrupts_pull`]: the parent blocks in `pull()` waiting for a `push()` that
//!   the child never issues.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    fmt::Debug,
    sync::atomic::{
        AtomicU32,
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
        fork,
        ipc,
        pm,
        sched,
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
        wexitstatus,
        wifexited,
    },
};
use ::syscall::unistd::bindings;

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations.
const ORDER: Ordering = Ordering::SeqCst;

/// Number of times that the child yields the processor, after announcing itself, before it exits.
/// The child has to be alive when the parent blocks in its rendezvous call: neither `push()` nor
/// `pull()` checks that its peer is alive, so a child that exited first would leave the parent
/// blocked forever (the harness then times the test out instead of reporting a false pass).
///
/// The announcement makes the parent ready. The scheduler runs the ready thread that has waited the
/// longest, and a thread that yields goes to the back of the line, so the first yield already hands
/// the processor to the parent, which keeps it until it blocks. The remaining yields only absorb a
/// preemption of the parent on its way there. Unlike a fixed sleep, this does not depend on how
/// long the parent takes to get there.
const CHILD_EXIT_YIELDS: usize = 16;

/// Length of the buffer handed to the blocking call. The transfer never takes place, so the value
/// only needs to be non-zero.
const TRANSFER_LEN: usize = 64;

/// Exit status reported by the child when it ran to completion.
const CHILD_EXIT_OK: c_int = 0;

/// Exit status reported by the child when its own setup failed.
const CHILD_EXIT_FAIL: c_int = 1;

//==================================================================================================
// Global State
//==================================================================================================

/// Parent process identifier, recorded before `fork()` so the child can recover it from
/// copy-on-write memory and address its announcement.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Helpers
//==================================================================================================

/// Child half shared by every test: announces its thread identifier to the parent and yields the
/// processor [`CHILD_EXIT_YIELDS`] times so that the parent can block. The caller then exits the
/// process without the child ever having taken part in a rendezvous.
///
/// The announcement is a regular IPC message sent under the child's real identity, so the kernel's
/// source-spoofing check accepts it.
fn run_child() -> Result<(), Error> {
    let child_pid: ProcessIdentifier = pm::getpid_uncached()?;
    let child_tid: ThreadIdentifier = pm::__kcall_gettid()?;

    // Recover the parent identifier stashed before fork() through copy-on-write memory.
    let parent_pid: ProcessIdentifier = ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER))?;

    // Report the child's thread identifier so the parent can target its rendezvous at this thread.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0..4].copy_from_slice(&u32::try_from(child_tid)?.to_le_bytes());
    let announce: Message = Message::new(
        MessageSender::new(child_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&announce)?;

    // Give way to the parent until it is blocked in its rendezvous call, so that this process exits
    // only afterwards.
    for _ in 0..CHILD_EXIT_YIELDS {
        sched::__kcall_sched_yield()?;
    }

    Ok(())
}

/// Forks a child that exits without taking part in any rendezvous and returns its process and
/// thread identifiers once it has announced itself.
///
/// The child never returns from this function: it exits with [`CHILD_EXIT_OK`] once it has yielded
/// to the parent [`CHILD_EXIT_YIELDS`] times, or with [`CHILD_EXIT_FAIL`] if [`run_child`] failed.
fn fork_exiting_peer() -> Result<(ProcessIdentifier, ThreadIdentifier), Error> {
    let parent_pid: ProcessIdentifier = pm::getpid_uncached()?;
    PARENT_PID_RAW.store(u32::try_from(parent_pid)?, ORDER);

    let child_pid: ProcessIdentifier = fork::__kcall_fork()?;
    if child_pid == ProcessIdentifier::from(0) {
        // Child path: never returns to the shared flow.
        let status: c_int = match run_child() {
            Ok(()) => CHILD_EXIT_OK,
            Err(_) => CHILD_EXIT_FAIL,
        };
        pm::__kcall_exit(status)?;
    }

    assert!(child_pid != parent_pid, "child PID must differ from parent PID");

    // Receive the child's announcement and extract the thread identifier that the rendezvous has to
    // be matched against.
    let announce: Message = ipc::__kcall_recv()?;
    assert!(announce.message_type == MessageType::Ipc, "expected IPC announcement from child");
    let source: MessageSender = announce.source;
    assert!(
        source.pid == child_pid,
        "announcement must come from the forked child (source={:?}, child={child_pid:?})",
        source.pid
    );
    let child_tid_raw: u32 = u32::from_le_bytes([
        announce.payload[0],
        announce.payload[1],
        announce.payload[2],
        announce.payload[3],
    ]);
    let child_tid: ThreadIdentifier = ThreadIdentifier::try_from(child_tid_raw)?;

    Ok((child_pid, child_tid))
}

/// Asserts that `result`, the outcome of the blocking rendezvous call `call`, reports that the call
/// was interrupted by the exit of its peer.
fn assert_interrupted<T: Debug>(call: &str, result: Result<T, Error>) {
    assert!(
        matches!(&result, Err(error) if error.code == ErrorCode::OperationTimedOut),
        "{call}() blocked on an exiting peer must fail with OperationTimedOut (got {result:?})"
    );
}

/// Reaps the already exited `child` and checks that it terminated normally with
/// [`CHILD_EXIT_OK`], which shows that the parent can still use the kernel after the peer's exit.
fn reap_child(child: ProcessIdentifier) {
    let mut status: c_int = 0;
    // SAFETY: `status` is a valid `c_int`.
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(i32::from(child), &raw mut status, 0) };
    assert!(
        reaped == i32::from(child),
        "waitpid() must return the child's PID (ret={reaped}, child={})",
        i32::from(child)
    );
    assert!(wifexited(status), "child must have exited normally (status={status:#x})");
    assert!(
        wexitstatus(status) == CHILD_EXIT_OK,
        "child exit status mismatch (got={}, expected={CHILD_EXIT_OK})",
        wexitstatus(status)
    );
}

//==================================================================================================
// Peer Exit While Blocked In pull()
//==================================================================================================

/// Verifies that the exit of a peer interrupts a thread that is blocked in `pull()` waiting for it.
///
/// The parent blocks in `pull()`, with no timeout, on a child that never pushes. The child exits
/// while the parent is blocked, so `do_exit()` has to release a pending pull whose source is the
/// exiting process. On the unfixed kernel that release woke the puller through `do_wakeup()` while
/// the running slot was empty, which panicked the kernel.
fn test_peer_exit_interrupts_pull() -> Result<(), Error> {
    let (child_pid, child_tid): (ProcessIdentifier, ThreadIdentifier) = fork_exiting_peer()?;

    let mut buffer: [u8; TRANSFER_LEN] = [0u8; TRANSFER_LEN];
    let result: Result<usize, Error> = ipc::__kcall_pull(child_pid, child_tid, &mut buffer);
    assert_interrupted("pull", result);

    reap_child(child_pid);

    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all peer-exit rendezvous regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-fork-kcall: starting peer-exit rendezvous tests");
    test_peer_exit_interrupts_pull()?;
    ::syslog::info!("test-fork-kcall: PASS - peer_exit_interrupts_pull");
    Ok(())
}
