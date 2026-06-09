// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `duplicate()` Kernel Call CoW Regression Tests
//!
//! Exercises the [`__kcall_duplicate`] kernel call end-to-end and verifies that the child
//! process is created with copy-on-write semantics:
//!
//! 1. A write performed by the parent *after* `duplicate()` must not be visible in the child.
//! 2. A write performed by the child must not be visible in the parent.
//! 3. The child must observe data that the parent wrote *before* `duplicate()`.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::alloc::Layout;
use ::arch::mem::PAGE_SIZE;
use ::core::sync::atomic::{
    AtomicU32,
    AtomicU8,
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
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations.
const ORDER: Ordering = Ordering::SeqCst;

/// Value written by the parent before `duplicate()`.  Both processes should observe this
/// value until one of them writes a new value.
const PATTERN_INIT: u8 = 0x11;

/// Value written by the parent *after* `duplicate()`.  Must remain invisible to the child.
const PATTERN_PARENT: u8 = 0x22;

/// Value written by the child *after* `duplicate()`.  Must remain invisible to the parent.
const PATTERN_CHILD: u8 = 0x33;

/// Stack size handed to the child's main thread.  Sized to comfortably fit the small
/// [`child_entry`] frame while keeping the parent's `link_user_pages` mapping count low
/// enough to fit within the kernel heap.
const CHILD_STACK_SIZE: usize = 4 * PAGE_SIZE;

/// Stack size used to populate the `user_stack_size` field in the argument-validation tests.
/// The actual stack pages are never consumed because the kernel is expected to reject these
/// requests on the basis of their invalid `user_fn`/`user_stack_base` fields.
const DUMMY_STACK_SIZE: usize = 8192;

//==================================================================================================
// Global State
//==================================================================================================

/// Shared byte living in the test program's `.data` segment.  After `duplicate()` the page
/// backing this byte is mapped copy-on-write into both address spaces.
static SHARED_BYTE: AtomicU8 = AtomicU8::new(0);

/// Parent process identifier, written before `duplicate()` so that the child can recover it
/// from the inherited (copy-on-write) memory image.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Child Entry Point
//==================================================================================================

/// Entry point for the child process spawned by `duplicate()`.
///
/// The child:
///
/// 1. Blocks on `recv()` to synchronize with the parent's post-`duplicate()` write.
/// 2. Reads [`SHARED_BYTE`] and confirms it observes the value the parent wrote *before*
///    `duplicate()`.
/// 3. Writes [`PATTERN_CHILD`] to [`SHARED_BYTE`] to take a private copy of the page.
/// 4. Reports both observed values to the parent via IPC.
/// 5. Spins until the parent terminates it.
extern "C" fn child_entry(_arg: usize) -> usize {
    // Recover identifiers.
    let my_pid: ProcessIdentifier = match pm::__kcall_getpid() {
        Ok(p) => p,
        Err(_) => return 1,
    };
    let parent_pid: ProcessIdentifier =
        match ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER)) {
            Ok(p) => p,
            Err(_) => return 2,
        };

    // Wait for the parent's go-ahead message.  This barrier guarantees that the parent's
    // post-`duplicate()` write to SHARED_BYTE has already happened by the time the child reads.
    if ipc::__kcall_recv().is_err() {
        return 3;
    }

    // Observe the byte before mutating it.  Must equal PATTERN_INIT if CoW correctly isolates
    // the parent's post-duplicate write from the child's view.
    let observed_before: u8 = SHARED_BYTE.load(ORDER);

    // Mutate the byte; this triggers a private CoW copy in the child.
    SHARED_BYTE.store(PATTERN_CHILD, ORDER);
    let observed_after: u8 = SHARED_BYTE.load(ORDER);

    // Report observations to the parent.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0] = observed_before;
    payload[1] = observed_after;
    let reply: Message = Message::new(
        MessageSender::from(my_pid),
        MessageReceiver::from(parent_pid),
        MessageType::Ipc,
        None,
        payload,
    );
    if ipc::__kcall_send(&reply).is_err() {
        return 4;
    }

    // Spin until the parent terminates us.
    loop {
        let _ = sched::__kcall_sched_yield();
    }
}

//==================================================================================================
// Test
//==================================================================================================

/// Runs the duplicate/CoW correctness scenario described in the module docstring.
fn test_duplicate_cow() -> Result<(), Error> {
    // Record the parent's PID where the child can find it via CoW-shared memory.
    let parent_pid: ProcessIdentifier = pm::__kcall_getpid()?;
    PARENT_PID_RAW.store(u32::try_from(parent_pid)?, ORDER);

    // Prime the shared byte with the pre-duplicate pattern.
    SHARED_BYTE.store(PATTERN_INIT, ORDER);

    // Allocate a page-aligned stack for the child's main thread.
    let layout: Layout = Layout::from_size_align(CHILD_STACK_SIZE, PAGE_SIZE)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "bad stack layout"))?;
    // SAFETY: layout has non-zero size.
    let stack_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if stack_ptr.is_null() {
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate child stack"));
    }
    let stack_base: VirtualAddress = VirtualAddress::from_raw_value(stack_ptr as usize);

    let args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: VirtualAddress::from_raw_value(child_entry as *const () as usize),
        user_fn_arg0: 0,
        user_fn_arg1: 0,
        user_stack_base: stack_base,
        user_stack_size: CHILD_STACK_SIZE,
        user_tda: None,
    };

    // Duplicate the calling process.
    let child_pid: ProcessIdentifier = pm::__kcall_duplicate(&args)?;
    assert!(child_pid != parent_pid, "child_pid must differ from parent_pid");

    // Write a new pattern to SHARED_BYTE.  This must trigger CoW: the parent obtains a private
    // copy while the child continues to see PATTERN_INIT until it performs its own write.
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
    assert!(reply_type == MessageType::Ipc, "expected IPC reply");

    let child_observed_before: u8 = reply.payload[0];
    let child_observed_after: u8 = reply.payload[1];
    let parent_observed: u8 = SHARED_BYTE.load(ORDER);

    // CoW invariant 1: parent's post-duplicate write is invisible to the child.
    assert!(
        child_observed_before == PATTERN_INIT,
        "child observed {:#x} before its own write; expected {:#x} (parent->child isolation \
         broken)",
        child_observed_before,
        PATTERN_INIT
    );
    // Sanity: the child's own write is visible to itself.
    assert!(
        child_observed_after == PATTERN_CHILD,
        "child observed {:#x} after its own write; expected {:#x}",
        child_observed_after,
        PATTERN_CHILD
    );
    // CoW invariant 2: child's write is invisible to the parent.
    assert!(
        parent_observed == PATTERN_PARENT,
        "parent observed {:#x} after child's write; expected {:#x} (child->parent isolation \
         broken)",
        parent_observed,
        PATTERN_PARENT
    );

    // Acquire the process-management capability required to terminate the child, tear it down,
    // then release the capability. The capability is always released and the stack always
    // reclaimed, even if the termination fails.
    let teardown: Result<(), Error> = match pm::__kcall_capctl(Capability::ProcessManagement, true)
    {
        Ok(()) => {
            let result: Result<(), Error> = pm::__kcall_terminate(child_pid);
            let _ = pm::__kcall_capctl(Capability::ProcessManagement, false);
            result
        },
        Err(e) => Err(e),
    };
    // SAFETY: `stack_ptr`/`layout` came from the matching `alloc::alloc::alloc` above and the
    // child is being terminated so it no longer references the parent's mapping of these pages.
    unsafe { ::alloc::alloc::dealloc(stack_ptr, layout) };

    teardown
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all `duplicate()` regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-kernel: duplicate: starting duplicate/CoW regression tests");

    test_duplicate_rejects_invalid_user_fn()?;
    ::syslog::info!("test-kernel: duplicate: PASS - rejects_invalid_user_fn");

    test_duplicate_rejects_invalid_stack()?;
    ::syslog::info!("test-kernel: duplicate: PASS - rejects_invalid_stack");

    test_duplicate_cow()?;
    ::syslog::info!("test-kernel: duplicate: PASS - duplicate_cow");

    ::syslog::info!("test-kernel: duplicate: all tests passed");

    Ok(())
}

//==================================================================================================
// Argument-Validation Tests
//==================================================================================================

/// Verifies that `__kcall_duplicate` rejects an invalid (null) user-space entry point with
/// [`ErrorCode::InvalidArgument`].
fn test_duplicate_rejects_invalid_user_fn() -> Result<(), Error> {
    let args: ThreadCreateArgs = ThreadCreateArgs {
        // Null is not a valid user-space address.
        user_fn: VirtualAddress::from_raw_value(0),
        user_fn_arg0: 0,
        user_fn_arg1: 0,
        // Pick any non-null value; the kernel must short-circuit on the invalid `user_fn` first
        // and not even reach the stack validation. The address itself does not need to be mapped.
        user_stack_base: VirtualAddress::from_raw_value(0x4000_0000),
        user_stack_size: DUMMY_STACK_SIZE,
        user_tda: None,
    };

    match pm::__kcall_duplicate(&args) {
        Err(e) if e.code == ErrorCode::InvalidArgument => Ok(()),
        Err(e) => Err(Error::new(e.code, "__kcall_duplicate returned unexpected error")),
        Ok(_) => Err(Error::new(
            ErrorCode::OperationNotPermitted,
            "__kcall_duplicate unexpectedly succeeded with null user_fn",
        )),
    }
}

/// Verifies that `__kcall_duplicate` rejects a stack region that lies outside user space with
/// [`ErrorCode::InvalidArgument`].
fn test_duplicate_rejects_invalid_stack() -> Result<(), Error> {
    let args: ThreadCreateArgs = ThreadCreateArgs {
        // Pick a plausible-looking user-space address so validation reaches the stack check.
        user_fn: VirtualAddress::from_raw_value(0x4000_0000),
        user_fn_arg0: 0,
        user_fn_arg1: 0,
        // Null is not a valid user-space address.
        user_stack_base: VirtualAddress::from_raw_value(0),
        user_stack_size: DUMMY_STACK_SIZE,
        user_tda: None,
    };

    match pm::__kcall_duplicate(&args) {
        Err(e) if e.code == ErrorCode::InvalidArgument => Ok(()),
        Err(e) => Err(Error::new(e.code, "__kcall_duplicate returned unexpected error")),
        Ok(_) => Err(Error::new(
            ErrorCode::OperationNotPermitted,
            "__kcall_duplicate unexpectedly succeeded with null stack",
        )),
    }
}
