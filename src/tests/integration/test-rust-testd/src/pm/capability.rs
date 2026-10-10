// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # `capctl()` Tests
//!
//! Verifies that the test process, which the kernel spawns directly, may acquire and release every
//! capability, and that a process forked from it may not acquire any capability. Otherwise, an
//! unprivileged process could grant itself the privileges that capability checks protect.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::proc::{
    wait,
    WaitOutcome,
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
    },
    kcall::{
        ipc,
        mm,
        pm,
        sched,
    },
    mm::{
        AccessPermission,
        VirtualAddress,
    },
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

/// Every capability that a process may attempt to acquire.
const CAPABILITIES: [Capability; 5] = [
    Capability::ExceptionControl,
    Capability::InterruptControl,
    Capability::IoManagement,
    Capability::MemoryManagement,
    Capability::ProcessManagement,
];

/// Size, in bytes, of an encoded `capctl()` result in the report of the unprivileged child.
const RESULT_SIZE: usize = ::core::mem::size_of::<i32>();

/// Size, in bytes, of the record that the unprivileged child reports for each capability: the
/// result of acquiring the capability followed by the result of releasing it.
const RECORD_SIZE: usize = 2 * RESULT_SIZE;

// Compile-time check: ensure the report of the unprivileged child fits in a message payload.
::static_assert::assert_eq!(CAPABILITIES.len() * RECORD_SIZE <= Message::PAYLOAD_SIZE);

/// Number of pages backing the child's main-thread stack.
const STACK_PAGES: usize = 2;

/// Size, in bytes, of the child's main-thread stack.
const STACK_BYTES: usize = STACK_PAGES * PAGE_SIZE;

/// Base virtual address of the region used to back the child stack.
///
/// This lies in the guard region between the unified mmap region and the user stack, matching the
/// convention used by the other low-level process-management tests. The stack is mapped only for
/// the duration of the test and unmapped before it returns.
const STACK_REGION_BASE: usize = ::config::memory_layout::USER_MMAP_END_RAW;

// Compile-time check: ensure the child stack fits within the guard region and does not overflow
// into the user stack.
::static_assert::assert_eq!(
    STACK_REGION_BASE + STACK_BYTES <= ::config::memory_layout::USER_STACK_TOP_RAW
);

//==================================================================================================
// Global State
//==================================================================================================

/// Parent process identifier, published before the child is spawned so that the child can recover
/// it from its copy-on-write inherited memory image.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Tests for capctl()
//==================================================================================================

///
/// # Description
///
/// Tests if [`Capability::ExceptionControl`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_exception_control() -> bool {
    // Attempt to acquire and release exception control capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false), Ok(()))
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::InterruptControl`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_interrupt_control() -> bool {
    // Attempt to acquire and release interrupt control capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, false), Ok(()))
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::IoManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_io_management() -> bool {
    // Attempt to acquire and release I/O management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, false), Ok(()))
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::MemoryManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_memory_management() -> bool {
    // Attempt to acquire and release memory management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false), Ok(()))
        },
        _ => false,
    }
}

///
/// # Description
///
/// Tests if [`Capability::ProcessManagement`] capability may be acquired and release.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_process_management() -> bool {
    // Attempt to acquire and release process management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
        Ok(()) => {
            matches!(::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false), Ok(()))
        },
        _ => false,
    }
}

///
/// # Description
///
/// Attempts to acquire the same capability twice.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_invalid_acquire() -> bool {
    // Attempt to acquire exception control capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire interrupt control capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire I/O management capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire memory management capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    // Attempt to acquire process management capability twice.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
        Ok(()) => match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
            Ok(()) => return false,
            _ => match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false) {
                Ok(()) => (),
                _ => return false,
            },
        },
        _ => return false,
    }

    true
}

///
/// # Description
///
/// Attempts to release a capability that was not acquired.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_invalid_release() -> bool {
    // Attempt to release exception control capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false) {
        return false;
    }

    // Attempt to release interrupt control capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::InterruptControl, false) {
        return false;
    }

    // Attempt to release I/O management capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::IoManagement, false) {
        return false;
    }

    // Attempt to release memory management capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
        return false;
    }

    // Attempt to release process management capability without acquiring it.
    if let Ok(()) = ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false) {
        return false;
    }

    true
}

//==================================================================================================
// Tests for capctl() from Unprivileged Processes
//==================================================================================================

/// Spins forever, yielding the processor on each iteration.
fn spin() -> ! {
    loop {
        let _ = sched::__kcall_sched_yield();
    }
}

/// Encodes the result of a `capctl()` call as zero on success, or as the error code on failure.
fn encode_result(result: Result<(), Error>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => i32::from(error.code),
    }
}

/// Decodes a `capctl()` result encoded by [`encode_result`].
fn decode_result(bytes: &[u8]) -> Option<i32> {
    let bytes: [u8; RESULT_SIZE] = bytes.try_into().ok()?;
    Some(i32::from_ne_bytes(bytes))
}

/// Entry point for the unprivileged child process spawned by the test.
///
/// The child is forked from the test process, so the kernel classifies it as a user process. It
/// attempts to acquire and then release every capability, reports the result of each attempt back
/// to the parent, and spins until the parent terminates it.
extern "C" fn unprivileged_child_entry(_arg: usize) -> usize {
    // Drop the parent's cached pid inherited through the duplicated address space. Unlike fork(),
    // the raw duplicate() primitive has no in-child choke point, so the child invalidates here.
    pm::invalidate_cached_pid();

    let my_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(_) => spin(),
    };
    let parent_pid: ProcessIdentifier =
        match ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER)) {
            Ok(pid) => pid,
            Err(_) => spin(),
        };

    // Attempt to acquire and then release every capability, recording both results.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let (records, _): (&mut [[u8; RECORD_SIZE]], &mut [u8]) =
        payload.as_chunks_mut::<RECORD_SIZE>();
    for (capability, record) in CAPABILITIES.iter().zip(records.iter_mut()) {
        let acquire: i32 = encode_result(pm::__kcall_capctl(*capability, true));
        let release: i32 = encode_result(pm::__kcall_capctl(*capability, false));
        record[..RESULT_SIZE].copy_from_slice(&acquire.to_ne_bytes());
        record[RESULT_SIZE..].copy_from_slice(&release.to_ne_bytes());
    }

    // Report the results to the parent.
    let report: Message = Message::new(
        MessageSender::new(my_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    let _ = ipc::__kcall_send(&report);

    // Spin until the parent terminates us.
    spin()
}

///
/// # Description
///
/// Checks the report of the unprivileged child. Every acquisition must have been denied with
/// [`ErrorCode::PermissionDenied`], and every release must have failed with
/// [`ErrorCode::NoSuchEntry`], because the child never held the capability that it released.
///
/// # Parameters
///
/// - `payload`: Payload of the report sent by the unprivileged child.
///
/// # Returns
///
/// If the report matches the expected results, `true` is returned. Otherwise, `false` is returned
/// instead.
///
fn check_unprivileged_report(payload: &[u8]) -> bool {
    let denied: i32 = i32::from(ErrorCode::PermissionDenied);
    let missing: i32 = i32::from(ErrorCode::NoSuchEntry);
    let mut success: bool = true;

    let (records, _): (&[[u8; RECORD_SIZE]], &[u8]) = payload.as_chunks::<RECORD_SIZE>();
    for (capability, record) in CAPABILITIES.iter().zip(records) {
        let acquire: Option<i32> = decode_result(&record[..RESULT_SIZE]);
        let release: Option<i32> = decode_result(&record[RESULT_SIZE..]);
        if acquire != Some(denied) || release != Some(missing) {
            ::syslog::error!(
                "unexpected capctl() results in unprivileged process (capability={:?}, \
                 acquire={:?}, release={:?})",
                capability,
                acquire,
                release
            );
            success = false;
        }
    }

    success
}

///
/// # Description
///
/// Verifies that a process forked from the test process cannot acquire any capability, and that it
/// observes a consistent result when it releases a capability that it does not hold. The test
/// spawns a child while holding the memory-management and process-management capabilities, which
/// the child does not inherit. The child attempts to acquire and then release every capability,
/// and the test checks the results that the child reports back before terminating and reaping it.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_capctl_unprivileged_acquire() -> bool {
    let parent_pid: ProcessIdentifier = match pm::getpid_uncached() {
        Ok(pid) => pid,
        Err(_) => return false,
    };
    match u32::try_from(parent_pid) {
        Ok(raw) => PARENT_PID_RAW.store(raw, ORDER),
        Err(_) => return false,
    }

    // Acquire the memory-management capability so that the child's stack can be mapped.
    if pm::__kcall_capctl(Capability::MemoryManagement, true).is_err() {
        return false;
    }

    // Acquire the process-management capability before spawning the child, so that the child can
    // always be torn down. Release the memory-management capability again if this fails so that no
    // capability leaks to later tests.
    if pm::__kcall_capctl(Capability::ProcessManagement, true).is_err() {
        let _ = pm::__kcall_capctl(Capability::MemoryManagement, false);
        return false;
    }

    let stack_base: VirtualAddress = VirtualAddress::from_raw_value(STACK_REGION_BASE);
    let mut success: bool = true;
    let mut child: Option<ProcessIdentifier> = None;

    // Map the child's stack.
    if mm::__kcall_mmap(parent_pid, stack_base, STACK_PAGES, AccessPermission::RDWR).is_err() {
        success = false;
    }

    // Spawn the unprivileged child.
    if success {
        let args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn: VirtualAddress::from_raw_value(unprivileged_child_entry as *const () as usize),
            user_fn_arg0: 0,
            user_fn_arg1: 0,
            user_stack_base: stack_base,
            user_stack_size: STACK_BYTES,
            user_tda: None,
        };
        match pm::__kcall_duplicate(&args) {
            Ok(spawned) if spawned != parent_pid => child = Some(spawned),
            _ => success = false,
        }
    }

    // Receive and check the report of the child.
    if let Some(child) = child {
        match ipc::__kcall_recv() {
            Ok(message) => {
                let message_type: MessageType = { message.message_type };
                let source: MessageSender = { message.source };
                if message_type != MessageType::Ipc
                    || source.pid != child
                    || !check_unprivileged_report(&message.payload)
                {
                    success = false;
                }
            },
            Err(_) => success = false,
        }
    }

    // Terminate the child and reap it through procd.
    if let Some(child) = child {
        if pm::__kcall_terminate(child).is_err() {
            success = false;
        } else {
            match wait(WaitTarget::Pid(child), 0) {
                Ok(WaitOutcome::Reaped { child: reaped, .. }) if reaped == child => {},
                _ => success = false,
            }
        }
    }

    // Reclaim the child's stack. `mmap()` reserves `STACK_PAGES` pages in a single call, but
    // `munmap()` releases a single page at a time, so every page must be released individually.
    for page in 0..STACK_PAGES {
        let page_addr: usize = STACK_REGION_BASE + page * PAGE_SIZE;
        if mm::__kcall_munmap(parent_pid, VirtualAddress::from_raw_value(page_addr)).is_err() {
            success = false;
        }
    }

    // Release the process-management capability.
    if pm::__kcall_capctl(Capability::ProcessManagement, false).is_err() {
        success = false;
    }

    // Release the memory-management capability.
    if pm::__kcall_capctl(Capability::MemoryManagement, false).is_err() {
        success = false;
    }

    success
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests kernel calls in the process management facility.
///
pub fn test() {
    crate::test!(test_capctl_exception_control());
    crate::test!(test_capctl_interrupt_control());
    crate::test!(test_capctl_io_management());
    crate::test!(test_capctl_memory_management());
    crate::test!(test_capctl_process_management());
    crate::test!(test_capctl_invalid_acquire());
    crate::test!(test_capctl_invalid_release());
    crate::test!(test_capctl_unprivileged_acquire());
}
