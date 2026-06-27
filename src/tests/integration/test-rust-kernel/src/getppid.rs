// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `getppid()` Kernel Call Regression Tests
//!
//! Exercises the [`__kcall_getppid`] kernel call end-to-end and verifies that the parent
//! process identifier reported to a child created via `duplicate()` matches the identifier of
//! the process that performed the duplication:
//!
//! 1. The calling process records its own PID in copy-on-write shared memory.
//! 2. It duplicates itself, producing a child whose kernel-tracked parent is the caller.
//! 3. The child calls `getppid()` and reports the result back to the parent via IPC.
//! 4. The parent asserts that the child observed the caller's PID as its parent.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::alloc::Layout;
use ::arch::mem::PAGE_SIZE;
use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
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
        sched,
    },
    mm::VirtualAddress,
    pm::{
        Capability,
        ProcessIdentifier,
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations.
const ORDER: Ordering = Ordering::SeqCst;

/// Stack size handed to the child's main thread.
const CHILD_STACK_SIZE: usize = 4 * PAGE_SIZE;

/// Child exit code: failed to query its own PID via `getpid()`.
const CHILD_ERR_GETPID: usize = 1;

/// Child exit code: failed to recover the parent's PID from CoW-shared memory.
const CHILD_ERR_PARENT_PID: usize = 2;

/// Child exit code: failed to receive the parent's go-ahead message.
const CHILD_ERR_RECV: usize = 3;

/// Child exit code: failed to query the parent PID via `getppid()`.
const CHILD_ERR_GETPPID: usize = 4;

/// Child exit code: failed to convert the observed parent PID to a raw value.
const CHILD_ERR_PPID_CONVERT: usize = 5;

/// Child exit code: failed to send the report back to the parent.
const CHILD_ERR_SEND: usize = 6;

//==================================================================================================
// Global State
//==================================================================================================

/// Parent process identifier, written before `duplicate()` so that the child can recover it from
/// the inherited (copy-on-write) memory image and compare it against `getppid()`.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Child Entry Point
//==================================================================================================

/// Entry point for the child process spawned by `duplicate()`.
///
/// The child:
///
/// 1. Recovers its own PID and the parent's PID (the latter from CoW-shared memory).
/// 2. Blocks on `recv()` to synchronize with the parent.
/// 3. Calls `getppid()` and reports the raw result back to the parent via IPC.
/// 4. Spins until the parent terminates it.
extern "C" fn child_entry(_arg: usize) -> usize {
    // Drop the parent's cached pid inherited through the duplicated address space. Unlike fork(),
    // the raw duplicate() primitive has no in-child choke point, so the child invalidates here.
    pm::invalidate_cached_pid();

    // Recover identifiers.
    let my_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(p) => p,
        Err(_) => return CHILD_ERR_GETPID,
    };
    let parent_pid: ProcessIdentifier =
        match ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER)) {
            Ok(p) => p,
            Err(_) => return CHILD_ERR_PARENT_PID,
        };

    // Wait for the parent's go-ahead message before reporting back.
    if ipc::__kcall_recv().is_err() {
        return CHILD_ERR_RECV;
    }

    // Query the parent process identifier via the kernel call under test.
    let ppid: ProcessIdentifier = match pm::__kcall_getppid() {
        Ok(p) => p,
        Err(_) => return CHILD_ERR_GETPPID,
    };
    let ppid_raw: i32 = match u32::try_from(ppid).ok().and_then(|v| i32::try_from(v).ok()) {
        Some(v) => v,
        None => return CHILD_ERR_PPID_CONVERT,
    };

    // Report the observed parent PID to the parent.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0..4].copy_from_slice(&ppid_raw.to_le_bytes());
    let reply: Message = Message::new(
        MessageSender::new(my_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    if ipc::__kcall_send(&reply).is_err() {
        return CHILD_ERR_SEND;
    }

    // Spin until the parent terminates us.
    loop {
        let _ = sched::__kcall_sched_yield();
    }
}

//==================================================================================================
// Test
//==================================================================================================

/// Verifies that a child created via `duplicate()` observes the duplicating process as its parent
/// through the `getppid()` kernel call.
fn test_getppid_reports_parent() -> Result<(), Error> {
    // Record the parent's PID where the child can find it via CoW-shared memory.
    let parent_pid: ProcessIdentifier = pm::getpid_uncached()?;
    PARENT_PID_RAW.store(u32::try_from(parent_pid)?, ORDER);

    // Allocate a page-aligned stack for the child's main thread.
    let layout: Layout = Layout::from_size_align(CHILD_STACK_SIZE, PAGE_SIZE)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "bad stack layout"))?;
    // SAFETY: layout has non-zero size.
    let stack_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if stack_ptr.is_null() {
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate child stack"));
    }
    let stack_base: VirtualAddress = VirtualAddress::from_raw_value(stack_ptr as usize);

    // Pre-fault every page of the child's stack so that the pages are present in the parent's
    // address space at `duplicate()` time. This guarantees they are inherited copy-on-write by the
    // child; otherwise the child's first stack write would fault on a page that the kernel cannot
    // resolve (no demand-fill metadata is inherited for never-touched pages).
    // SAFETY: `stack_ptr` points to a freshly allocated `CHILD_STACK_SIZE`-byte region.
    unsafe { ::core::ptr::write_bytes(stack_ptr, 0, CHILD_STACK_SIZE) };

    let args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: VirtualAddress::from_raw_value(child_entry as *const () as usize),
        user_fn_arg0: 0,
        user_fn_arg1: 0,
        user_stack_base: stack_base,
        user_stack_size: CHILD_STACK_SIZE,
        user_tda: None,
    };

    // Duplicate the calling process. The child's kernel-tracked parent is this process.
    let child_pid: ProcessIdentifier = match pm::__kcall_duplicate(&args) {
        Ok(child_pid) => child_pid,
        Err(e) => {
            // The duplication failed, so no child references the parent's mapping of these pages.
            // SAFETY: `stack_ptr`/`layout` came from the matching `alloc::alloc::alloc` above.
            unsafe { ::alloc::alloc::dealloc(stack_ptr, layout) };
            return Err(e);
        },
    };

    // Interact with the child. The interaction is fallible, but the child must always be torn
    // down and its stack reclaimed afterwards so that a failure does not leave an orphaned
    // spinning process or leak the stack allocation.
    let outcome: Result<(), Error> = observe_child_parent(parent_pid, child_pid);

    // Tear down the child and reclaim the stack regardless of the interaction outcome. The
    // process-management capability is acquired only for the termination and released afterwards
    // (only when this path acquired it). A release failure is surfaced as well so that it does not
    // leave `ProcessManagement` enabled for subsequent tests. `ResourceBusy` means the capability
    // is already held, so termination must still proceed without leaving it disabled afterwards.
    let acquired: Result<bool, Error> =
        match pm::__kcall_capctl(Capability::ProcessManagement, true) {
            Ok(()) => Ok(true),
            Err(e) if e.code == ErrorCode::ResourceBusy => Ok(false),
            Err(e) => Err(e),
        };
    let teardown: Result<(), Error> = match acquired {
        Ok(acquired) => {
            let terminate: Result<(), Error> = pm::__kcall_terminate(child_pid);
            let release: Result<(), Error> = if acquired {
                pm::__kcall_capctl(Capability::ProcessManagement, false)
            } else {
                Ok(())
            };
            terminate.and(release)
        },
        Err(e) => Err(e),
    };
    // SAFETY: `stack_ptr`/`layout` came from the matching `alloc::alloc::alloc` above and the
    // child is being terminated so it no longer references the parent's mapping of these pages.
    unsafe { ::alloc::alloc::dealloc(stack_ptr, layout) };

    // Surface the interaction error first, then any teardown error.
    outcome.and(teardown)
}

/// Releases the child, receives its `getppid()` report, and verifies that the child observed the
/// duplicating process as its parent.
fn observe_child_parent(
    parent_pid: ProcessIdentifier,
    child_pid: ProcessIdentifier,
) -> Result<(), Error> {
    assert!(child_pid != parent_pid, "child_pid must differ from parent_pid");

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
    assert!(reply_type == MessageType::Ipc, "expected IPC reply");

    let mut ppid_bytes: [u8; 4] = [0u8; 4];
    ppid_bytes.copy_from_slice(&reply.payload[0..4]);
    let child_ppid_raw: i32 = i32::from_le_bytes(ppid_bytes);

    // The child must observe the duplicating process as its parent.
    let parent_pid_raw: i32 = i32::try_from(u32::try_from(parent_pid)?)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "parent pid out of range"))?;
    assert!(
        child_ppid_raw == parent_pid_raw,
        "child getppid() returned {}; expected {} (parent lineage broken)",
        child_ppid_raw,
        parent_pid_raw
    );

    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all `getppid()` regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-kernel: getppid: starting getppid() regression tests");

    test_getppid_reports_parent()?;
    ::syslog::info!("test-kernel: getppid: PASS - getppid_reports_parent");

    ::syslog::info!("test-kernel: getppid: all tests passed");

    Ok(())
}
