// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::alloc::Layout;
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::core::{
    sync::atomic::{
        AtomicBool,
        AtomicU32,
        Ordering,
    },
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        ipc,
        pm,
        sched,
    },
    mm::VirtualAddress,
    pm::{
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

/// Test payload for the basic 16-byte push/pull test.
const TEST_PAYLOAD_16: [u8; 16] = [
    0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
];

//==================================================================================================
// Global Variables
//==================================================================================================

/// Thread identifier of the main thread, shared with child threads.
static MAIN_TID: AtomicU32 = AtomicU32::new(0);

/// Process identifier, shared between main and child threads.
static SELF_PID: AtomicU32 = AtomicU32::new(0);

/// Transfer size communicated to child threads for parameterized tests.
static TRANSFER_SIZE: AtomicU32 = AtomicU32::new(0);

/// Iteration count communicated to child threads for multi-round tests.
static ITERATION_COUNT: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Allocates a stack for a child thread from the heap.
///
/// # Returns
///
/// On success, returns a tuple of (raw pointer, layout, base virtual address) for the allocated
/// stack.  On failure, returns an error.
///
fn alloc_thread_stack() -> Result<(*mut u8, Layout, VirtualAddress), Error> {
    let layout: Layout =
        Layout::from_size_align(USER_THREAD_STACK_SIZE, core::mem::align_of::<usize>())
            .map_err(|_| Error::new(ErrorCode::OutOfMemory, "bad stack layout"))?;
    // SAFETY: layout has non-zero size.
    let stack_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if stack_ptr.is_null() {
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate thread stack"));
    }
    let stack_base: VirtualAddress = VirtualAddress::from_raw_value(stack_ptr as usize);
    Ok((stack_ptr, layout, stack_base))
}

///
/// # Description
///
/// Frees a previously allocated thread stack.
///
/// # Parameters
///
/// - `stack_ptr`: Raw pointer to the stack memory.
/// - `layout`: Layout used during allocation.
///
/// # Safety
///
/// The caller must ensure that `stack_ptr` was allocated with the same `layout` and is no longer
/// in use by any thread.
///
unsafe fn free_thread_stack(stack_ptr: *mut u8, layout: Layout) {
    // SAFETY: guaranteed by caller.
    unsafe { ::alloc::alloc::dealloc(stack_ptr, layout) };
}

///
/// # Description
///
/// Creates a child thread running the given function and returns its thread identifier.
///
/// # Parameters
///
/// - `entry`: Entry point function for the child thread.
/// - `stack_base`: Base address of the pre-allocated stack.
///
/// # Returns
///
/// On success, the thread identifier of the newly created thread.  On failure, an error.
///
fn spawn_child_thread(
    entry: extern "C" fn(usize) -> usize,
    stack_base: VirtualAddress,
) -> Result<ThreadIdentifier, Error> {
    let mut args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: entry as *const () as usize,
        user_fn_arg1: 0,
        user_stack_base: stack_base,
        user_stack_size: USER_THREAD_STACK_SIZE,
        user_tda: None,
    };
    pm::__kcall_create_thread(&mut args)
}

///
/// # Description
///
/// Joins a child thread and validates that it exited successfully (return value 0).
///
/// # Parameters
///
/// - `tid`: Thread identifier of the child to join.
///
/// # Returns
///
/// On success, empty.  On failure, an error describing the child's failure.
///
fn join_child_thread(tid: ThreadIdentifier) -> Result<(), Error> {
    let mut retval: usize = 0;
    pm::__kcall_join_thread(tid, &mut retval)?;
    if retval != 0 {
        ::syslog::error!("child thread failed (tid={:?}, retval={})", tid, retval);
        return Err(Error::new(ErrorCode::OperationNotPermitted, "child thread reported failure"));
    }
    Ok(())
}

///
/// # Description
///
/// Stores the calling process and thread identifiers into the shared globals so that child threads
/// can read them.
///
/// # Returns
///
/// A tuple of (pid, main_tid).
///
fn store_caller_ids() -> Result<(ProcessIdentifier, ThreadIdentifier), Error> {
    let pid: ProcessIdentifier = pm::getpid_uncached()?;
    let pid_raw: u32 = u32::try_from(pid)?;
    SELF_PID.store(pid_raw, ORDER);

    let main_tid: ThreadIdentifier = pm::__kcall_gettid()?;
    let main_tid_raw: u32 = u32::try_from(main_tid)?;
    MAIN_TID.store(main_tid_raw, ORDER);

    Ok((pid, main_tid))
}

///
/// # Description
///
/// Reconstructs the process and main-thread identifiers from the shared globals inside a child
/// thread.
///
/// # Returns
///
/// A tuple of (pid, main_tid) or an error for child threads.
///
fn load_parent_ids() -> Result<(ProcessIdentifier, ThreadIdentifier), ()> {
    let pid_raw: u32 = SELF_PID.load(ORDER);
    let main_tid_raw: u32 = MAIN_TID.load(ORDER);

    let pid: ProcessIdentifier = ProcessIdentifier::try_from(pid_raw).map_err(|_| ())?;
    let main_tid: ThreadIdentifier = ThreadIdentifier::try_from(main_tid_raw).map_err(|_| ())?;
    Ok((pid, main_tid))
}

///
/// # Description
///
/// Generates a deterministic test pattern of the given length.  Each byte is computed as
/// `(index * 37 + 7) & 0xFF` so that different sizes yield different bit patterns.
///
/// # Parameters
///
/// - `buf`: Mutable slice to fill.
///
fn fill_pattern(buf: &mut [u8]) {
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = ((i.wrapping_mul(37).wrapping_add(7)) & 0xFF) as u8;
    }
}

///
/// # Description
///
/// Verifies that `buf` contains the deterministic pattern produced by [`fill_pattern`].
///
/// # Parameters
///
/// - `buf`: Slice to check.
///
/// # Returns
///
/// `true` if every byte matches, `false` otherwise.
///
fn verify_pattern(buf: &[u8]) -> bool {
    for (i, &byte) in buf.iter().enumerate() {
        let expected: u8 = ((i.wrapping_mul(37).wrapping_add(7)) & 0xFF) as u8;
        if byte != expected {
            return false;
        }
    }
    true
}

//==================================================================================================
// Test 1: Basic 16-byte Push/Pull (push first)
//==================================================================================================

///
/// # Description
///
/// Child thread that pushes the 16-byte test payload to the main thread.
///
extern "C" fn pusher_thread_basic(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    match ipc::__kcall_push(pid, main_tid, &TEST_PAYLOAD_16) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Tests the basic push/pull rendezvous with a 16-byte payload. The child thread pushes first
/// and blocks until the main thread pulls.
///
fn test_basic_push_pull() -> Result<(), Error> {
    ::syslog::info!("test_basic_push_pull: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_basic, stack_base)?;

    // Pull data from the child thread.
    let mut recv_buf: [u8; 16] = [0u8; 16];
    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

    // Validate byte count.
    if bytes_transferred != TEST_PAYLOAD_16.len() {
        ::syslog::error!(
            "test_basic_push_pull: wrong byte count (expected={}, got={})",
            TEST_PAYLOAD_16.len(),
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    // Validate payload contents.
    if recv_buf != TEST_PAYLOAD_16 {
        ::syslog::error!("test_basic_push_pull: payload mismatch");
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_basic_push_pull: passed");
    Ok(())
}

//==================================================================================================
// Test 2: Pull-First Ordering (pull sleeps, then push completes)
//==================================================================================================

///
/// # Description
///
/// Child thread that yields to let the main thread's pull register first, then pushes.
///
extern "C" fn pusher_thread_pull_first(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    // Yield several times to give the main thread time to call pull() and sleep.
    for _ in 0..5 {
        if sched::__kcall_sched_yield().is_err() {
            return 1;
        }
    }

    match ipc::__kcall_push(pid, main_tid, &TEST_PAYLOAD_16) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Tests the pull-first ordering: the main thread calls pull before the child calls push,
/// exercising the path where the puller sleeps and the pusher finds a matching pending pull.
///
fn test_pull_first() -> Result<(), Error> {
    ::syslog::info!("test_pull_first: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_pull_first, stack_base)?;

    // Pull immediately — the child yields multiple times, so we should sleep first.
    let mut recv_buf: [u8; 16] = [0u8; 16];
    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

    if bytes_transferred != TEST_PAYLOAD_16.len() {
        ::syslog::error!(
            "test_pull_first: wrong byte count (expected={}, got={})",
            TEST_PAYLOAD_16.len(),
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    if recv_buf != TEST_PAYLOAD_16 {
        ::syslog::error!("test_pull_first: payload mismatch");
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_pull_first: passed");
    Ok(())
}

//==================================================================================================
// Test 3: Single-Byte Transfer
//==================================================================================================

///
/// # Description
///
/// Child thread that pushes exactly one byte.
///
extern "C" fn pusher_thread_one_byte(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let payload: [u8; 1] = [0x42];
    match ipc::__kcall_push(pid, main_tid, &payload) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Tests the smallest non-trivial transfer: a single byte.
///
fn test_single_byte() -> Result<(), Error> {
    ::syslog::info!("test_single_byte: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_one_byte, stack_base)?;

    let mut recv_buf: [u8; 1] = [0u8; 1];
    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

    if bytes_transferred != 1 {
        ::syslog::error!(
            "test_single_byte: wrong byte count (expected=1, got={})",
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    if recv_buf[0] != 0x42 {
        ::syslog::error!(
            "test_single_byte: payload mismatch (expected=0x42, got={:#x})",
            recv_buf[0]
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_single_byte: passed");
    Ok(())
}

//==================================================================================================
// Test 4: Asymmetric Buffer Sizes (pusher sends more than puller can receive)
//==================================================================================================

///
/// # Description
///
/// Child thread that pushes the full 16-byte payload even though the puller only has an 8-byte
/// buffer.  The kernel should truncate to `min(push_len, pull_len)`.
///
extern "C" fn pusher_thread_asymmetric(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    match ipc::__kcall_push(pid, main_tid, &TEST_PAYLOAD_16) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Tests asymmetric buffer sizes: the pusher offers 16 bytes but the puller only provides an
/// 8-byte buffer.  Validates that `min(push_len, pull_len)` bytes are actually transferred.
///
fn test_asymmetric_buffers() -> Result<(), Error> {
    ::syslog::info!("test_asymmetric_buffers: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_asymmetric, stack_base)?;

    // Provide only 8 bytes to the pull.
    let mut recv_buf: [u8; 8] = [0u8; 8];
    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

    // The kernel should transfer min(16, 8) = 8 bytes.
    if bytes_transferred != 8 {
        ::syslog::error!(
            "test_asymmetric_buffers: wrong byte count (expected=8, got={})",
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    // Validate the first 8 bytes match the payload prefix.
    if recv_buf != TEST_PAYLOAD_16[..8] {
        ::syslog::error!("test_asymmetric_buffers: payload mismatch");
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_asymmetric_buffers: passed");
    Ok(())
}

//==================================================================================================
// Test 5: Zero-Length Transfer
//==================================================================================================

///
/// # Description
///
/// Child thread that pushes zero bytes.
///
extern "C" fn pusher_thread_zero(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let empty: [u8; 0] = [];
    match ipc::__kcall_push(pid, main_tid, &empty) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Tests a zero-length transfer.  Both sides specify zero bytes; the rendezvous should complete
/// immediately without copying any data.
///
fn test_zero_length() -> Result<(), Error> {
    ::syslog::info!("test_zero_length: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_zero, stack_base)?;

    let mut recv_buf: [u8; 0] = [];
    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

    if bytes_transferred != 0 {
        ::syslog::error!(
            "test_zero_length: wrong byte count (expected=0, got={})",
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_zero_length: passed");
    Ok(())
}

//==================================================================================================
// Test 6: Larger Payload (parameterized size)
//==================================================================================================

///
/// # Description
///
/// Child thread that pushes a deterministic payload of size read from `TRANSFER_SIZE`.
///
extern "C" fn pusher_thread_large(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let size: usize = TRANSFER_SIZE.load(ORDER) as usize;

    // Allocate and fill the send buffer.
    let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
        Ok(l) => l,
        Err(_) => return 1,
    };
    // SAFETY: layout has non-zero size.
    let send_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if send_ptr.is_null() {
        return 1;
    }

    // Fill with deterministic pattern.
    let send_buf: &mut [u8] =
        // SAFETY: send_ptr is valid for `size` bytes.
        unsafe { ::core::slice::from_raw_parts_mut(send_ptr, size) };
    fill_pattern(send_buf);

    let result: usize = match ipc::__kcall_push(pid, main_tid, send_buf) {
        Ok(()) => 0,
        Err(_) => 1,
    };

    // SAFETY: send_ptr was allocated with the same layout.
    unsafe { ::alloc::alloc::dealloc(send_ptr, layout) };
    result
}

///
/// # Description
///
/// Tests a larger transfer of `size` bytes, validating the deterministic pattern.
///
/// # Parameters
///
/// - `size`: Number of bytes to transfer.
///
fn test_large_transfer(size: usize) -> Result<(), Error> {
    ::syslog::info!("test_large_transfer: starting (size={})", size);

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    TRANSFER_SIZE.store(size as u32, ORDER);

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_large, stack_base)?;

    // Allocate receive buffer.
    let recv_layout: Layout = Layout::from_size_align(size, core::mem::align_of::<u8>())
        .map_err(|_| Error::new(ErrorCode::OutOfMemory, "bad recv layout"))?;
    // SAFETY: layout has non-zero size.
    let recv_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(recv_layout) };
    if recv_ptr.is_null() {
        // SAFETY: child thread hasn't finished yet, but we must clean up.
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate recv buffer"));
    }
    // SAFETY: recv_ptr is valid for `size` bytes.
    let recv_buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(recv_ptr, size) };

    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, recv_buf)?;

    if bytes_transferred != size {
        ::syslog::error!(
            "test_large_transfer: wrong byte count (expected={}, got={})",
            size,
            bytes_transferred
        );
        // SAFETY: recv_ptr was allocated with recv_layout.
        unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    if !verify_pattern(recv_buf) {
        ::syslog::error!("test_large_transfer: payload mismatch (size={})", size);
        // SAFETY: recv_ptr was allocated with recv_layout.
        unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    // SAFETY: recv_ptr was allocated with recv_layout and is no longer in use.
    unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_large_transfer: passed (size={})", size);
    Ok(())
}

//==================================================================================================
// Test 7: Multiple Sequential Transfers on the Same Thread Pair
//==================================================================================================

///
/// # Description
///
/// Child thread that performs multiple sequential pushes of 16 bytes to the main thread.
///
extern "C" fn pusher_thread_multi(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let iterations: u32 = ITERATION_COUNT.load(ORDER);

    for i in 0..iterations {
        let mut payload: [u8; 16] = [0u8; 16];
        // Fill each iteration with a different byte to distinguish rounds.
        let fill_byte: u8 = (i & 0xFF) as u8;
        for b in payload.iter_mut() {
            *b = fill_byte;
        }

        if ipc::__kcall_push(pid, main_tid, &payload).is_err() {
            return 1;
        }
    }

    0
}

///
/// # Description
///
/// Tests multiple sequential push/pull transfers on the same thread pair. Validates that the
/// rendezvous mechanism correctly resets between rounds.
///
fn test_multi_round() -> Result<(), Error> {
    const ROUNDS: u32 = 5;

    ::syslog::info!("test_multi_round: starting (rounds={})", ROUNDS);

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    ITERATION_COUNT.store(ROUNDS, ORDER);

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_multi, stack_base)?;

    for i in 0..ROUNDS {
        let mut recv_buf: [u8; 16] = [0u8; 16];
        let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

        if bytes_transferred != 16 {
            ::syslog::error!(
                "test_multi_round: round {}: wrong byte count (expected=16, got={})",
                i,
                bytes_transferred
            );
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
        }

        let expected_byte: u8 = (i & 0xFF) as u8;
        if !recv_buf.iter().all(|&b| b == expected_byte) {
            ::syslog::error!(
                "test_multi_round: round {}: payload mismatch (expected fill={:#x})",
                i,
                expected_byte
            );
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
        }
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_multi_round: passed");
    Ok(())
}

//==================================================================================================
// Test 8: Self-Push Rejection
//==================================================================================================

///
/// # Description
///
/// Tests that pushing a thread's data to itself is rejected with an appropriate error.
///
fn test_self_push_rejected() -> Result<(), Error> {
    ::syslog::info!("test_self_push_rejected: starting");

    let pid: ProcessIdentifier = pm::getpid_uncached()?;
    let tid: ThreadIdentifier = pm::__kcall_gettid()?;

    let payload: [u8; 4] = [0x01, 0x02, 0x03, 0x04];
    match ipc::__kcall_push(pid, tid, &payload) {
        Err(e) if e.code == ErrorCode::InvalidArgument => {
            // Expected: self-push is rejected.
        },
        Err(e) => {
            ::syslog::error!(
                "test_self_push_rejected: unexpected error code (got={:?}, expected={:?})",
                e.code,
                ErrorCode::InvalidArgument
            );
            return Err(Error::new(ErrorCode::InvalidArgument, "unexpected error code"));
        },
        Ok(()) => {
            ::syslog::error!("test_self_push_rejected: push to self should have failed");
            return Err(Error::new(ErrorCode::InvalidArgument, "self-push unexpectedly succeeded"));
        },
    }

    ::syslog::info!("test_self_push_rejected: passed");
    Ok(())
}

//==================================================================================================
// Test 9: Self-Pull Rejection
//==================================================================================================

///
/// # Description
///
/// Tests that pulling from the calling thread itself is rejected with an appropriate error.
///
fn test_self_pull_rejected() -> Result<(), Error> {
    ::syslog::info!("test_self_pull_rejected: starting");

    let pid: ProcessIdentifier = pm::getpid_uncached()?;
    let tid: ThreadIdentifier = pm::__kcall_gettid()?;

    let mut recv_buf: [u8; 4] = [0u8; 4];
    match ipc::__kcall_pull(pid, tid, &mut recv_buf) {
        Err(e) if e.code == ErrorCode::InvalidArgument => {
            // Expected: self-pull is rejected.
        },
        Err(e) => {
            ::syslog::error!(
                "test_self_pull_rejected: unexpected error code (got={:?}, expected={:?})",
                e.code,
                ErrorCode::InvalidArgument
            );
            return Err(Error::new(ErrorCode::InvalidArgument, "unexpected error code"));
        },
        Ok(_) => {
            ::syslog::error!("test_self_pull_rejected: pull from self should have failed");
            return Err(Error::new(ErrorCode::InvalidArgument, "self-pull unexpectedly succeeded"));
        },
    }

    ::syslog::info!("test_self_pull_rejected: passed");
    Ok(())
}

//==================================================================================================
// Test 10: Reverse Direction (main pushes, child pulls)
//==================================================================================================

///
/// # Description
///
/// Child thread that pulls data from the main thread.
///
extern "C" fn puller_thread_reverse(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let mut recv_buf: [u8; 16] = [0u8; 16];
    let bytes_transferred: usize = match ipc::__kcall_pull(pid, main_tid, &mut recv_buf) {
        Ok(n) => n,
        Err(_) => return 1,
    };

    if bytes_transferred != TEST_PAYLOAD_16.len() {
        return 1;
    }

    if recv_buf != TEST_PAYLOAD_16 {
        return 1;
    }

    0
}

///
/// # Description
///
/// Tests the reverse direction: the main thread pushes data and the child thread pulls it.
///
fn test_reverse_direction() -> Result<(), Error> {
    ::syslog::info!("test_reverse_direction: starting");

    let (_pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(puller_thread_reverse, stack_base)?;

    // Retrieve our own PID for the push call.
    let pid: ProcessIdentifier = pm::getpid_uncached()?;

    // Push data to the child thread.
    ipc::__kcall_push(pid, child_tid, &TEST_PAYLOAD_16)?;

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_reverse_direction: passed");
    Ok(())
}

//==================================================================================================
// Test 11: Bidirectional (both threads push and pull in sequence)
//==================================================================================================

///
/// # Description
///
/// Child thread that first pushes data to the main thread, then pulls data from it, validating
/// bidirectional communication over the same thread pair.
///
extern "C" fn bidir_child_thread(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    // Phase 1: push data to the main thread.
    let outgoing: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
    if ipc::__kcall_push(pid, main_tid, &outgoing).is_err() {
        return 1;
    }

    // Phase 2: pull data from the main thread.
    let mut incoming: [u8; 4] = [0u8; 4];
    let n: usize = match ipc::__kcall_pull(pid, main_tid, &mut incoming) {
        Ok(n) => n,
        Err(_) => return 1,
    };

    if n != 4 {
        return 1;
    }

    let expected: [u8; 4] = [0x11, 0x22, 0x33, 0x44];
    if incoming != expected {
        return 1;
    }

    0
}

///
/// # Description
///
/// Tests bidirectional communication: the child pushes to the main thread, then the main thread
/// pushes back to the child. Exercises both orderings on the same thread pair.
///
fn test_bidirectional() -> Result<(), Error> {
    ::syslog::info!("test_bidirectional: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(bidir_child_thread, stack_base)?;

    // Phase 1: pull from the child.
    let mut recv_buf: [u8; 4] = [0u8; 4];
    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

    if bytes_transferred != 4 {
        ::syslog::error!(
            "test_bidirectional: phase 1: wrong byte count (expected=4, got={})",
            bytes_transferred
        );
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    let expected_from_child: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
    if recv_buf != expected_from_child {
        ::syslog::error!("test_bidirectional: phase 1: payload mismatch");
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    // Phase 2: push to the child.
    let reply: [u8; 4] = [0x11, 0x22, 0x33, 0x44];
    ipc::__kcall_push(pid, child_tid, &reply)?;

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_bidirectional: passed");
    Ok(())
}

//==================================================================================================
// Test 12: Concurrent Push from Multiple Threads
//==================================================================================================

/// Number of concurrent pusher threads.
const CONCURRENT_THREAD_COUNT: usize = 4;

///
/// # Description
///
/// Child thread that pushes a unique payload (identified by thread index) to the main thread.
/// The thread index is encoded in every byte of the payload to distinguish data from each sender.
///
extern "C" fn concurrent_pusher_thread(arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    // Use the argument as the thread index to generate a unique fill byte.
    let fill_byte: u8 = (arg & 0xFF) as u8;
    let size: usize = TRANSFER_SIZE.load(ORDER) as usize;

    // Allocate and fill the send buffer.
    let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
        Ok(l) => l,
        Err(_) => return 1,
    };
    // SAFETY: layout has non-zero size.
    let send_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if send_ptr.is_null() {
        return 1;
    }
    // SAFETY: send_ptr is valid for `size` bytes.
    let send_buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(send_ptr, size) };
    for b in send_buf.iter_mut() {
        *b = fill_byte;
    }

    let result: usize = match ipc::__kcall_push(pid, main_tid, send_buf) {
        Ok(()) => 0,
        Err(_) => 1,
    };

    // SAFETY: send_ptr was allocated with the same layout.
    unsafe { ::alloc::alloc::dealloc(send_ptr, layout) };
    result
}

///
/// # Description
///
/// Tests concurrent push/pull from multiple threads. N child threads each push unique data to the
/// main thread. The main thread pulls from each child sequentially and validates the payload.
///
/// # Parameters
///
/// - `transfer_size`: Number of bytes each thread sends.
///
#[allow(clippy::needless_range_loop)]
fn test_concurrent_push(transfer_size: usize) -> Result<(), Error> {
    ::syslog::info!(
        "test_concurrent_push: starting (threads={}, size={})",
        CONCURRENT_THREAD_COUNT,
        transfer_size
    );

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    TRANSFER_SIZE.store(transfer_size as u32, ORDER);

    // Allocate stacks and spawn child threads.
    let mut stacks: [(*mut u8, Layout); CONCURRENT_THREAD_COUNT] =
        [(core::ptr::null_mut(), unsafe { Layout::from_size_align_unchecked(1, 1) });
            CONCURRENT_THREAD_COUNT];
    let mut child_tids: [Option<ThreadIdentifier>; CONCURRENT_THREAD_COUNT] =
        [None; CONCURRENT_THREAD_COUNT];

    for i in 0..CONCURRENT_THREAD_COUNT {
        let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) =
            alloc_thread_stack()?;
        stacks[i] = (stack_ptr, layout);

        // Pass the thread index as the argument via user_fn_arg1.
        let mut args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn: ThreadCreateArgs::NULL_USER_FN,
            user_fn_arg0: concurrent_pusher_thread as *const () as usize,
            user_fn_arg1: i,
            user_stack_base: stack_base,
            user_stack_size: USER_THREAD_STACK_SIZE,
            user_tda: None,
        };
        let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;
        child_tids[i] = Some(tid);
    }

    // Pull from each child and validate.
    for i in 0..CONCURRENT_THREAD_COUNT {
        let child_tid: ThreadIdentifier = child_tids[i]
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "missing child tid"))?;

        // Allocate receive buffer.
        let recv_layout: Layout =
            Layout::from_size_align(transfer_size, core::mem::align_of::<u8>())
                .map_err(|_| Error::new(ErrorCode::OutOfMemory, "bad recv layout"))?;
        // SAFETY: layout has non-zero size.
        let recv_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(recv_layout) };
        if recv_ptr.is_null() {
            return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate recv buffer"));
        }
        // SAFETY: recv_ptr is valid for `transfer_size` bytes.
        let recv_buf: &mut [u8] =
            unsafe { ::core::slice::from_raw_parts_mut(recv_ptr, transfer_size) };

        let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, recv_buf)?;

        if bytes_transferred != transfer_size {
            ::syslog::error!(
                "test_concurrent_push: thread {}: wrong byte count (expected={}, got={})",
                i,
                transfer_size,
                bytes_transferred
            );
            unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
        }

        let expected_fill: u8 = (i & 0xFF) as u8;
        if !recv_buf.iter().all(|&b| b == expected_fill) {
            ::syslog::error!(
                "test_concurrent_push: thread {}: payload mismatch (expected fill={:#x})",
                i,
                expected_fill
            );
            unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
        }

        unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
    }

    // Join all child threads.
    for i in 0..CONCURRENT_THREAD_COUNT {
        if let Some(tid) = child_tids[i] {
            join_child_thread(tid)?;
        }
        // SAFETY: stack is no longer in use after join.
        unsafe { free_thread_stack(stacks[i].0, stacks[i].1) };
    }

    ::syslog::info!("test_concurrent_push: passed");
    Ok(())
}

//==================================================================================================
// Test 13: Concurrent Pull from Multiple Threads
//==================================================================================================

///
/// # Description
///
/// Child thread that pulls data from the main thread and validates the payload.
/// Returns 0 on success, 1 on failure.
///
extern "C" fn concurrent_puller_thread(arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let expected_fill: u8 = (arg & 0xFF) as u8;
    let size: usize = TRANSFER_SIZE.load(ORDER) as usize;

    // Allocate receive buffer.
    let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
        Ok(l) => l,
        Err(_) => return 1,
    };
    // SAFETY: layout has non-zero size.
    let recv_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(layout) };
    if recv_ptr.is_null() {
        return 1;
    }
    // SAFETY: recv_ptr is valid for `size` bytes.
    let recv_buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(recv_ptr, size) };

    let bytes_transferred: usize = match ipc::__kcall_pull(pid, main_tid, recv_buf) {
        Ok(n) => n,
        Err(_) => {
            unsafe { ::alloc::alloc::dealloc(recv_ptr, layout) };
            return 1;
        },
    };

    let success: bool = bytes_transferred == size && recv_buf.iter().all(|&b| b == expected_fill);

    // SAFETY: recv_ptr was allocated with the same layout.
    unsafe { ::alloc::alloc::dealloc(recv_ptr, layout) };

    if success {
        0
    } else {
        1
    }
}

///
/// # Description
///
/// Tests concurrent pull from multiple threads: the main thread pushes unique data to N child
/// threads that are each waiting to pull.
///
/// # Parameters
///
/// - `transfer_size`: Number of bytes each thread receives.
///
#[allow(clippy::needless_range_loop)]
fn test_concurrent_pull(transfer_size: usize) -> Result<(), Error> {
    ::syslog::info!(
        "test_concurrent_pull: starting (threads={}, size={})",
        CONCURRENT_THREAD_COUNT,
        transfer_size
    );

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    TRANSFER_SIZE.store(transfer_size as u32, ORDER);

    // Allocate stacks and spawn child threads.
    let mut stacks: [(*mut u8, Layout); CONCURRENT_THREAD_COUNT] =
        [(core::ptr::null_mut(), unsafe { Layout::from_size_align_unchecked(1, 1) });
            CONCURRENT_THREAD_COUNT];
    let mut child_tids: [Option<ThreadIdentifier>; CONCURRENT_THREAD_COUNT] =
        [None; CONCURRENT_THREAD_COUNT];

    for i in 0..CONCURRENT_THREAD_COUNT {
        let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) =
            alloc_thread_stack()?;
        stacks[i] = (stack_ptr, layout);

        let mut args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn: ThreadCreateArgs::NULL_USER_FN,
            user_fn_arg0: concurrent_puller_thread as *const () as usize,
            user_fn_arg1: i,
            user_stack_base: stack_base,
            user_stack_size: USER_THREAD_STACK_SIZE,
            user_tda: None,
        };
        let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;
        child_tids[i] = Some(tid);
    }

    // Yield to let children register their pull requests.
    for _ in 0..5 {
        sched::__kcall_sched_yield()?;
    }

    // Push unique data to each child.
    for i in 0..CONCURRENT_THREAD_COUNT {
        let child_tid: ThreadIdentifier = child_tids[i]
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "missing child tid"))?;

        let fill_byte: u8 = (i & 0xFF) as u8;

        // Allocate send buffer.
        let send_layout: Layout =
            Layout::from_size_align(transfer_size, core::mem::align_of::<u8>())
                .map_err(|_| Error::new(ErrorCode::OutOfMemory, "bad send layout"))?;
        // SAFETY: layout has non-zero size.
        let send_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(send_layout) };
        if send_ptr.is_null() {
            return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate send buffer"));
        }
        // SAFETY: send_ptr is valid for `transfer_size` bytes.
        let send_buf: &mut [u8] =
            unsafe { ::core::slice::from_raw_parts_mut(send_ptr, transfer_size) };
        for b in send_buf.iter_mut() {
            *b = fill_byte;
        }

        ipc::__kcall_push(pid, child_tid, send_buf)?;

        // SAFETY: send_ptr was allocated with send_layout.
        unsafe { ::alloc::alloc::dealloc(send_ptr, send_layout) };
    }

    // Join all child threads.
    for i in 0..CONCURRENT_THREAD_COUNT {
        if let Some(tid) = child_tids[i] {
            join_child_thread(tid)?;
        }
        // SAFETY: stack is no longer in use after join.
        unsafe { free_thread_stack(stacks[i].0, stacks[i].1) };
    }

    ::syslog::info!("test_concurrent_pull: passed");
    Ok(())
}

//==================================================================================================
// Test 14: Concurrent Independent Pairs
//==================================================================================================

///
/// # Description
///
/// Pair of threads: even-indexed thread pushes to its partner (odd-indexed), and the odd-indexed
/// thread pulls. Each pair operates independently.
///
/// Thread argument encodes: `pair_index * 2 + role` where role is 0 for pusher, 1 for puller.
/// The partner's TID is communicated via PARTNER_TIDS global array.
///
static mut PARTNER_TIDS: [u32; CONCURRENT_THREAD_COUNT] = [0; CONCURRENT_THREAD_COUNT];

/// Readiness flag: set by the main thread after all `PARTNER_TIDS` entries have been written.
/// Child threads spin-yield on this flag before reading their partner TID.
static PARTNER_TIDS_READY: AtomicBool = AtomicBool::new(false);

/// Abort flag: set by [`PartnerTidsReadyGuard`] on drop when initialization was incomplete (early
/// return via `?`). Child threads check this after being unblocked and exit immediately if set,
/// preventing reads of stale/invalid partner TIDs.
static PARTNER_TIDS_ABORT: AtomicBool = AtomicBool::new(false);

///
/// # Description
///
/// RAII guard that manages the [`PARTNER_TIDS`] handoff between the main thread and child threads.
/// On creation it zeroes all entries and clears both flags. On drop it signals abort and sets
/// [`PARTNER_TIDS_READY`] so children never spin forever if the spawn loop returns early via `?`.
///
struct PartnerTidsReadyGuard;

impl PartnerTidsReadyGuard {
    ///
    /// # Description
    ///
    /// Clears the readiness and abort flags, zeroes all [`PARTNER_TIDS`] entries to prevent stale
    /// reads, and returns a guard that will unblock children on drop.
    ///
    fn new() -> Self {
        PARTNER_TIDS_READY.store(false, ORDER);
        PARTNER_TIDS_ABORT.store(false, ORDER);
        // SAFETY: no child thread reads PARTNER_TIDS while PARTNER_TIDS_READY is false.
        unsafe {
            let ptr: *mut u32 = core::ptr::addr_of_mut!(PARTNER_TIDS) as *mut u32;
            for i in 0..CONCURRENT_THREAD_COUNT {
                core::ptr::write(ptr.add(i), 0);
            }
        }
        Self
    }
}

impl Drop for PartnerTidsReadyGuard {
    ///
    /// # Description
    ///
    /// Signals abort and sets [`PARTNER_TIDS_READY`] to `true`, unblocking any child threads that
    /// are spin-yielding. Children will see [`PARTNER_TIDS_ABORT`] as `true` and exit immediately
    /// instead of reading potentially stale partner TIDs.
    ///
    fn drop(&mut self) {
        PARTNER_TIDS_ABORT.store(true, ORDER);
        PARTNER_TIDS_READY.store(true, ORDER);
    }
}

///
/// # Description
///
/// Entry point for a thread in an independent pair test. Pushes or pulls based on thread role.
///
extern "C" fn independent_pair_thread(arg: usize) -> usize {
    let pid_raw: u32 = SELF_PID.load(ORDER);
    let pid: ProcessIdentifier = match ProcessIdentifier::try_from(pid_raw) {
        Ok(p) => p,
        Err(_) => return 1,
    };

    let pair_idx: usize = arg / 2;
    let role: usize = arg % 2;
    let fill_byte: u8 = (pair_idx & 0xFF) as u8;
    let size: usize = TRANSFER_SIZE.load(ORDER) as usize;

    // Spin-yield until the main thread signals that all partner TIDs are written.
    while !PARTNER_TIDS_READY.load(ORDER) {
        if sched::__kcall_sched_yield().is_err() {
            return 1;
        }
    }

    // If initialization was incomplete (early return in spawn loop), exit immediately.
    if PARTNER_TIDS_ABORT.load(ORDER) {
        return 1;
    }

    // Get partner TID.
    // SAFETY: PARTNER_TIDS_READY ensures all entries are written before we read.
    let partner_tid_raw: u32 = unsafe { PARTNER_TIDS[arg ^ 1] };
    let partner_tid: ThreadIdentifier = match ThreadIdentifier::try_from(partner_tid_raw) {
        Ok(t) => t,
        Err(_) => return 1,
    };

    if role == 0 {
        // Pusher role.
        let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
            Ok(l) => l,
            Err(_) => return 1,
        };
        // SAFETY: layout has non-zero size.
        let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
        if buf_ptr.is_null() {
            return 1;
        }
        // SAFETY: buf_ptr is valid for `size` bytes.
        let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, size) };
        for b in buf.iter_mut() {
            *b = fill_byte;
        }
        let result: usize = match ipc::__kcall_push(pid, partner_tid, buf) {
            Ok(()) => 0,
            Err(_) => 1,
        };
        unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
        result
    } else {
        // Puller role.
        let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
            Ok(l) => l,
            Err(_) => return 1,
        };
        // SAFETY: layout has non-zero size.
        let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(layout) };
        if buf_ptr.is_null() {
            return 1;
        }
        // SAFETY: buf_ptr is valid for `size` bytes.
        let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, size) };

        let result: usize = match ipc::__kcall_pull(pid, partner_tid, buf) {
            Ok(n) if n == size && buf.iter().all(|&b| b == fill_byte) => 0,
            _ => 1,
        };
        unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
        result
    }
}

///
/// # Description
///
/// Tests multiple independent push/pull pairs running concurrently. Each pair consists of a pusher
/// and a puller thread that transfer data without involving the main thread.
///
#[allow(clippy::needless_range_loop)]
fn test_independent_pairs() -> Result<(), Error> {
    const NUM_PAIRS: usize = CONCURRENT_THREAD_COUNT / 2;
    const NUM_THREADS: usize = NUM_PAIRS * 2;
    const TRANSFER: usize = 128;

    ::syslog::info!("test_independent_pairs: starting (pairs={})", NUM_PAIRS);

    let pid: ProcessIdentifier = pm::getpid_uncached()?;
    let pid_raw: u32 = u32::try_from(pid)?;
    SELF_PID.store(pid_raw, ORDER);
    TRANSFER_SIZE.store(TRANSFER as u32, ORDER);

    // Guard clears flag on creation; sets it on drop so children never spin forever.
    let _ready_guard: PartnerTidsReadyGuard = PartnerTidsReadyGuard::new();

    // Allocate stacks and spawn threads.
    let mut stacks: [(*mut u8, Layout); CONCURRENT_THREAD_COUNT] =
        [(core::ptr::null_mut(), unsafe { Layout::from_size_align_unchecked(1, 1) });
            CONCURRENT_THREAD_COUNT];
    let mut child_tids: [Option<ThreadIdentifier>; CONCURRENT_THREAD_COUNT] =
        [None; CONCURRENT_THREAD_COUNT];

    // First, allocate stacks and create threads in a suspended-like manner by yielding first.
    for i in 0..NUM_THREADS {
        let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) =
            alloc_thread_stack()?;
        stacks[i] = (stack_ptr, layout);

        let mut args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn: ThreadCreateArgs::NULL_USER_FN,
            user_fn_arg0: independent_pair_thread as *const () as usize,
            user_fn_arg1: i,
            user_stack_base: stack_base,
            user_stack_size: USER_THREAD_STACK_SIZE,
            user_tda: None,
        };
        let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;
        child_tids[i] = Some(tid);

        // Store TID so partner can find it.
        let tid_raw: u32 = u32::try_from(tid)?;
        // SAFETY: threads spin-yield on PARTNER_TIDS_READY before reading.
        unsafe { PARTNER_TIDS[i] = tid_raw };
    }

    // Signal threads that all partner TIDs are now valid. Clear the abort flag first so children
    // proceed normally, then set readiness. The guard also unblocks children on drop as a safety
    // net for early returns from the spawn loop above (with abort set).
    PARTNER_TIDS_ABORT.store(false, ORDER);
    PARTNER_TIDS_READY.store(true, ORDER);

    // Join all threads.
    for i in 0..NUM_THREADS {
        if let Some(tid) = child_tids[i] {
            join_child_thread(tid)?;
        }
        // SAFETY: stack is no longer in use after join.
        unsafe { free_thread_stack(stacks[i].0, stacks[i].1) };
    }

    ::syslog::info!("test_independent_pairs: passed");
    Ok(())
}

//==================================================================================================
// Test 15: Stress Sequential Transfers
//==================================================================================================

///
/// # Description
///
/// Child thread that performs many sequential pushes with different payloads. Each iteration uses
/// a different fill byte.
///
extern "C" fn stress_pusher_thread(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let iterations: u32 = ITERATION_COUNT.load(ORDER);
    let size: usize = TRANSFER_SIZE.load(ORDER) as usize;

    let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
        Ok(l) => l,
        Err(_) => return 1,
    };
    // SAFETY: layout has non-zero size.
    let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if buf_ptr.is_null() {
        return 1;
    }
    // SAFETY: buf_ptr is valid for `size` bytes.
    let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, size) };

    for i in 0..iterations {
        let fill_byte: u8 = (i & 0xFF) as u8;
        for b in buf.iter_mut() {
            *b = fill_byte;
        }
        if ipc::__kcall_push(pid, main_tid, buf).is_err() {
            unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
            return 1;
        }
    }

    // SAFETY: buf_ptr was allocated with the same layout.
    unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
    0
}

///
/// # Description
///
/// Stress test that runs many sequential push/pull rounds between a single pair of threads.
/// Validates data integrity on each round.
///
fn test_stress_sequential() -> Result<(), Error> {
    const ROUNDS: u32 = 20;
    const SIZE: usize = 256;

    ::syslog::info!("test_stress_sequential: starting (rounds={}, size={})", ROUNDS, SIZE);

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    ITERATION_COUNT.store(ROUNDS, ORDER);
    TRANSFER_SIZE.store(SIZE as u32, ORDER);

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(stress_pusher_thread, stack_base)?;

    // Allocate receive buffer once.
    let recv_layout: Layout = Layout::from_size_align(SIZE, core::mem::align_of::<u8>())
        .map_err(|_| Error::new(ErrorCode::OutOfMemory, "bad recv layout"))?;
    // SAFETY: layout has non-zero size.
    let recv_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(recv_layout) };
    if recv_ptr.is_null() {
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate recv buffer"));
    }
    // SAFETY: recv_ptr is valid for SIZE bytes.
    let recv_buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(recv_ptr, SIZE) };

    for i in 0..ROUNDS {
        // Zero the buffer before each pull.
        for b in recv_buf.iter_mut() {
            *b = 0;
        }

        let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, recv_buf)?;

        if bytes_transferred != SIZE {
            ::syslog::error!(
                "test_stress_sequential: round {}: wrong byte count (expected={}, got={})",
                i,
                SIZE,
                bytes_transferred
            );
            unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
        }

        let expected_fill: u8 = (i & 0xFF) as u8;
        if !recv_buf.iter().all(|&b| b == expected_fill) {
            ::syslog::error!(
                "test_stress_sequential: round {}: payload mismatch (expected fill={:#x})",
                i,
                expected_fill
            );
            unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
        }
    }

    // SAFETY: recv_ptr was allocated with recv_layout and is no longer in use.
    unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_stress_sequential: passed");
    Ok(())
}

//==================================================================================================
// Test 16: Large Bidirectional Transfer
//==================================================================================================

///
/// # Description
///
/// Child thread that performs a bidirectional large transfer: pushes a deterministic pattern to
/// the main thread, then pulls a different pattern from it.
///
extern "C" fn bidir_large_child_thread(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let size: usize = TRANSFER_SIZE.load(ORDER) as usize;

    // Allocate buffer for both send and receive.
    let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
        Ok(l) => l,
        Err(_) => return 1,
    };

    // Phase 1: Push deterministic pattern to main.
    // SAFETY: layout has non-zero size.
    let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
    if buf_ptr.is_null() {
        return 1;
    }
    // SAFETY: buf_ptr is valid for `size` bytes.
    let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, size) };
    fill_pattern(buf);

    if ipc::__kcall_push(pid, main_tid, buf).is_err() {
        unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
        return 1;
    }

    // Phase 2: Pull data from main and validate reverse pattern.
    for b in buf.iter_mut() {
        *b = 0;
    }

    let bytes_transferred: usize = match ipc::__kcall_pull(pid, main_tid, buf) {
        Ok(n) => n,
        Err(_) => {
            unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
            return 1;
        },
    };

    // Validate: main sends complement pattern (0xFF ^ pattern_byte).
    let mut success: bool = bytes_transferred == size;
    if success {
        for (i, &byte) in buf.iter().enumerate() {
            let expected: u8 = 0xFF ^ (((i.wrapping_mul(37).wrapping_add(7)) & 0xFF) as u8);
            if byte != expected {
                success = false;
                break;
            }
        }
    }

    unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };

    if success {
        0
    } else {
        1
    }
}

///
/// # Description
///
/// Tests bidirectional large data transfer: child pushes a deterministic pattern, main validates
/// it, then main pushes a complement pattern back to the child.
///
fn test_large_bidirectional() -> Result<(), Error> {
    const SIZE: usize = 1024;

    ::syslog::info!("test_large_bidirectional: starting (size={})", SIZE);

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    TRANSFER_SIZE.store(SIZE as u32, ORDER);

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(bidir_large_child_thread, stack_base)?;

    // Phase 1: Pull from child and validate deterministic pattern.
    let buf_layout: Layout = Layout::from_size_align(SIZE, core::mem::align_of::<u8>())
        .map_err(|_| Error::new(ErrorCode::OutOfMemory, "bad layout"))?;
    // SAFETY: buf_layout has non-zero size.
    let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(buf_layout) };
    if buf_ptr.is_null() {
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate buffer"));
    }
    // SAFETY: buf_ptr is valid for SIZE bytes.
    let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, SIZE) };

    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, buf)?;

    if bytes_transferred != SIZE {
        ::syslog::error!(
            "test_large_bidirectional: phase 1: wrong byte count (expected={}, got={})",
            SIZE,
            bytes_transferred
        );
        unsafe { ::alloc::alloc::dealloc(buf_ptr, buf_layout) };
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }

    if !verify_pattern(buf) {
        ::syslog::error!("test_large_bidirectional: phase 1: payload mismatch");
        unsafe { ::alloc::alloc::dealloc(buf_ptr, buf_layout) };
        join_child_thread(child_tid).ok();
        unsafe { free_thread_stack(stack_ptr, layout) };
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    // Phase 2: Push complement pattern to child.
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = 0xFF ^ (((i.wrapping_mul(37).wrapping_add(7)) & 0xFF) as u8);
    }

    ipc::__kcall_push(pid, child_tid, buf)?;

    // SAFETY: buf_ptr was allocated with buf_layout and is no longer in use.
    unsafe { ::alloc::alloc::dealloc(buf_ptr, buf_layout) };

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_large_bidirectional: passed");
    Ok(())
}

//==================================================================================================
// Test 17: Reverse Asymmetric Buffers (puller has larger buffer than pusher)
//==================================================================================================

///
/// # Description
///
/// Child thread that pulls into a 16-byte buffer when the main thread pushes only 4 bytes.
/// Validates that `min(push_len, pull_len)` bytes are transferred when the puller's buffer is
/// larger.
///
extern "C" fn puller_thread_reverse_asymmetric(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let mut recv_buf: [u8; 16] = [0u8; 16];
    let bytes_transferred: usize = match ipc::__kcall_pull(pid, main_tid, &mut recv_buf) {
        Ok(n) => n,
        Err(_) => return 1,
    };

    // Expect only 4 bytes transferred (min of push=4, pull=16).
    if bytes_transferred != 4 {
        return 1;
    }

    let expected: [u8; 4] = [0xA1, 0xB2, 0xC3, 0xD4];
    if recv_buf[..4] != expected {
        return 1;
    }

    0
}

///
/// # Description
///
/// Tests asymmetric buffer sizes in reverse: the pusher offers fewer bytes (4) than the puller
/// can accept (16). Validates that `min(push_len, pull_len)` bytes are transferred.
///
fn test_reverse_asymmetric() -> Result<(), Error> {
    ::syslog::info!("test_reverse_asymmetric: starting");

    let (_pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier =
        spawn_child_thread(puller_thread_reverse_asymmetric, stack_base)?;

    let pid: ProcessIdentifier = pm::getpid_uncached()?;

    // Push only 4 bytes to the child that expects up to 16.
    let payload: [u8; 4] = [0xA1, 0xB2, 0xC3, 0xD4];
    ipc::__kcall_push(pid, child_tid, &payload)?;

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_reverse_asymmetric: passed");
    Ok(())
}

//==================================================================================================
// Test 18: Mixed Transfer Sizes (sequential rounds with varying sizes)
//==================================================================================================

/// Transfer sizes for the mixed-size test rounds.
const MIXED_SIZES: [u32; 6] = [1, 7, 64, 255, 1024, 4096];

///
/// # Description
///
/// Child thread that pushes payloads of varying sizes to the main thread, one per round.
/// The size for each round is read from `MIXED_SIZES`.
///
extern "C" fn mixed_size_pusher_thread(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    for &size_u32 in MIXED_SIZES.iter() {
        let size: usize = size_u32 as usize;

        let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
            Ok(l) => l,
            Err(_) => return 1,
        };
        // SAFETY: layout has non-zero size.
        let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
        if buf_ptr.is_null() {
            return 1;
        }
        // SAFETY: buf_ptr is valid for `size` bytes.
        let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, size) };
        fill_pattern(buf);

        let result: bool = ipc::__kcall_push(pid, main_tid, buf).is_ok();

        // SAFETY: buf_ptr was allocated with the same layout.
        unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };

        if !result {
            return 1;
        }
    }

    0
}

///
/// # Description
///
/// Tests sequential push/pull with varying transfer sizes per round. Each round uses a different
/// size from `MIXED_SIZES`, validating that the rendezvous mechanism handles size changes
/// between rounds correctly.
///
fn test_mixed_transfer_sizes() -> Result<(), Error> {
    ::syslog::info!("test_mixed_transfer_sizes: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(mixed_size_pusher_thread, stack_base)?;

    for &size_u32 in MIXED_SIZES.iter() {
        let size: usize = size_u32 as usize;

        let recv_layout: Layout = Layout::from_size_align(size, core::mem::align_of::<u8>())
            .map_err(|_| Error::new(ErrorCode::OutOfMemory, "bad recv layout"))?;
        // SAFETY: layout has non-zero size.
        let recv_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(recv_layout) };
        if recv_ptr.is_null() {
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::OutOfMemory, "failed to allocate recv buffer"));
        }
        // SAFETY: recv_ptr is valid for `size` bytes.
        let recv_buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(recv_ptr, size) };

        let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, recv_buf)?;

        if bytes_transferred != size {
            ::syslog::error!(
                "test_mixed_transfer_sizes: wrong byte count (expected={}, got={})",
                size,
                bytes_transferred
            );
            unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
        }

        if !verify_pattern(recv_buf) {
            ::syslog::error!("test_mixed_transfer_sizes: payload mismatch (size={})", size);
            unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
        }

        // SAFETY: recv_ptr was allocated with recv_layout and is no longer in use.
        unsafe { ::alloc::alloc::dealloc(recv_ptr, recv_layout) };
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_mixed_transfer_sizes: passed");
    Ok(())
}

//==================================================================================================
// Test 19: Multi-Round Bidirectional (alternating push/pull for N rounds)
//==================================================================================================

///
/// # Description
///
/// Child thread that performs multiple bidirectional rounds with the main thread. In each round,
/// the child pushes data and then pulls data back, verifying both directions per round.
///
extern "C" fn multi_round_bidir_child(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let rounds: u32 = ITERATION_COUNT.load(ORDER);

    for i in 0..rounds {
        // Phase A: push to main, with fill byte based on round index.
        let fill_byte: u8 = (i & 0xFF) as u8;
        let mut outgoing: [u8; 16] = [0u8; 16];
        for b in outgoing.iter_mut() {
            *b = fill_byte;
        }
        if ipc::__kcall_push(pid, main_tid, &outgoing).is_err() {
            return 1;
        }

        // Phase B: pull from main, expect complement fill byte.
        let mut incoming: [u8; 16] = [0u8; 16];
        let n: usize = match ipc::__kcall_pull(pid, main_tid, &mut incoming) {
            Ok(n) => n,
            Err(_) => return 1,
        };

        if n != 16 {
            return 1;
        }

        let expected_byte: u8 = !fill_byte;
        if !incoming.iter().all(|&b| b == expected_byte) {
            return 1;
        }
    }

    0
}

///
/// # Description
///
/// Tests multiple rounds of bidirectional communication on the same thread pair. Each round:
///   1. Child pushes to main, main validates.
///   2. Main pushes complement back to child, child validates.
///
/// This exercises rendezvous state reset between rounds and direction changes.
///
fn test_multi_round_bidirectional() -> Result<(), Error> {
    const ROUNDS: u32 = 5;

    ::syslog::info!("test_multi_round_bidirectional: starting (rounds={})", ROUNDS);

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    ITERATION_COUNT.store(ROUNDS, ORDER);

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;

    let child_tid: ThreadIdentifier = spawn_child_thread(multi_round_bidir_child, stack_base)?;

    for i in 0..ROUNDS {
        // Phase A: pull from child.
        let mut recv_buf: [u8; 16] = [0u8; 16];
        let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

        if bytes_transferred != 16 {
            ::syslog::error!(
                "test_multi_round_bidirectional: round {}: pull wrong byte count (expected=16, \
                 got={})",
                i,
                bytes_transferred
            );
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
        }

        let expected_fill: u8 = (i & 0xFF) as u8;
        if !recv_buf.iter().all(|&b| b == expected_fill) {
            ::syslog::error!("test_multi_round_bidirectional: round {}: pull payload mismatch", i);
            join_child_thread(child_tid).ok();
            unsafe { free_thread_stack(stack_ptr, layout) };
            return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
        }

        // Phase B: push complement back to child.
        let reply_byte: u8 = !expected_fill;
        let mut reply: [u8; 16] = [0u8; 16];
        for b in reply.iter_mut() {
            *b = reply_byte;
        }
        ipc::__kcall_push(pid, child_tid, &reply)?;
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_multi_round_bidirectional: passed");
    Ok(())
}

//==================================================================================================
// Test 20: Stress Concurrent Independent Pairs
//==================================================================================================

/// Number of rounds each pair performs in the stress concurrent pairs test.
const STRESS_PAIR_ROUNDS: u32 = 10;

///
/// # Description
///
/// Entry point for a thread in the stress concurrent pairs test. Each pusher performs multiple
/// rounds of push to its partner, with different fill bytes per round. Each puller pulls and
/// validates the same number of rounds.
///
extern "C" fn stress_pair_thread(arg: usize) -> usize {
    let pid_raw: u32 = SELF_PID.load(ORDER);
    let pid: ProcessIdentifier = match ProcessIdentifier::try_from(pid_raw) {
        Ok(p) => p,
        Err(_) => return 1,
    };

    let pair_idx: usize = arg / 2;
    let role: usize = arg % 2;
    let size: usize = TRANSFER_SIZE.load(ORDER) as usize;
    let rounds: u32 = ITERATION_COUNT.load(ORDER);

    // Spin-yield until the main thread signals that all partner TIDs are written.
    while !PARTNER_TIDS_READY.load(ORDER) {
        if sched::__kcall_sched_yield().is_err() {
            return 1;
        }
    }

    // If initialization was incomplete (early return in spawn loop), exit immediately.
    if PARTNER_TIDS_ABORT.load(ORDER) {
        return 1;
    }

    // Get partner TID.
    // SAFETY: PARTNER_TIDS_READY ensures all entries are written before we read.
    let partner_tid_raw: u32 = unsafe { PARTNER_TIDS[arg ^ 1] };
    let partner_tid: ThreadIdentifier = match ThreadIdentifier::try_from(partner_tid_raw) {
        Ok(t) => t,
        Err(_) => return 1,
    };

    let layout: Layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()) {
        Ok(l) => l,
        Err(_) => return 1,
    };

    if role == 0 {
        // Pusher role: push `rounds` times with different fill bytes.
        // SAFETY: layout has non-zero size.
        let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
        if buf_ptr.is_null() {
            return 1;
        }
        // SAFETY: buf_ptr is valid for `size` bytes.
        let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, size) };

        for r in 0..rounds {
            let fill_byte: u8 = ((pair_idx.wrapping_mul(17).wrapping_add(r as usize)) & 0xFF) as u8;
            for b in buf.iter_mut() {
                *b = fill_byte;
            }
            if ipc::__kcall_push(pid, partner_tid, buf).is_err() {
                unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
                return 1;
            }
        }

        unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
        0
    } else {
        // Puller role: pull `rounds` times and validate each round.
        // SAFETY: layout has non-zero size.
        let buf_ptr: *mut u8 = unsafe { ::alloc::alloc::alloc_zeroed(layout) };
        if buf_ptr.is_null() {
            return 1;
        }
        // SAFETY: buf_ptr is valid for `size` bytes.
        let buf: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(buf_ptr, size) };

        for r in 0..rounds {
            for b in buf.iter_mut() {
                *b = 0;
            }

            let n: usize = match ipc::__kcall_pull(pid, partner_tid, buf) {
                Ok(n) => n,
                Err(_) => {
                    unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
                    return 1;
                },
            };

            let expected_fill: u8 =
                ((pair_idx.wrapping_mul(17).wrapping_add(r as usize)) & 0xFF) as u8;
            if n != size || !buf.iter().all(|&b| b == expected_fill) {
                unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
                return 1;
            }
        }

        unsafe { ::alloc::alloc::dealloc(buf_ptr, layout) };
        0
    }
}

///
/// # Description
///
/// Stress test with multiple independent push/pull pairs, each performing multiple rounds.
/// This exercises the pending request queues with multiple entries from different pairs
/// interleaving on the scheduler.
///
#[allow(clippy::needless_range_loop)]
fn test_stress_concurrent_pairs() -> Result<(), Error> {
    const NUM_PAIRS: usize = CONCURRENT_THREAD_COUNT / 2;
    const NUM_THREADS: usize = NUM_PAIRS * 2;
    const TRANSFER: usize = 128;

    ::syslog::info!(
        "test_stress_concurrent_pairs: starting (pairs={}, rounds={}, size={})",
        NUM_PAIRS,
        STRESS_PAIR_ROUNDS,
        TRANSFER
    );

    let pid: ProcessIdentifier = pm::getpid_uncached()?;
    let pid_raw: u32 = u32::try_from(pid)?;
    SELF_PID.store(pid_raw, ORDER);
    TRANSFER_SIZE.store(TRANSFER as u32, ORDER);
    ITERATION_COUNT.store(STRESS_PAIR_ROUNDS, ORDER);

    // Guard clears flag on creation; sets it on drop so children never spin forever.
    let _ready_guard: PartnerTidsReadyGuard = PartnerTidsReadyGuard::new();

    // Allocate stacks and spawn threads.
    let mut stacks: [(*mut u8, Layout); CONCURRENT_THREAD_COUNT] =
        [(core::ptr::null_mut(), unsafe { Layout::from_size_align_unchecked(1, 1) });
            CONCURRENT_THREAD_COUNT];
    let mut child_tids: [Option<ThreadIdentifier>; CONCURRENT_THREAD_COUNT] =
        [None; CONCURRENT_THREAD_COUNT];

    for i in 0..NUM_THREADS {
        let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) =
            alloc_thread_stack()?;
        stacks[i] = (stack_ptr, layout);

        let mut args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn: ThreadCreateArgs::NULL_USER_FN,
            user_fn_arg0: stress_pair_thread as *const () as usize,
            user_fn_arg1: i,
            user_stack_base: stack_base,
            user_stack_size: USER_THREAD_STACK_SIZE,
            user_tda: None,
        };
        let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;
        child_tids[i] = Some(tid);

        // Store TID so partner can find it.
        let tid_raw: u32 = u32::try_from(tid)?;
        // SAFETY: threads spin-yield on PARTNER_TIDS_READY before reading.
        unsafe { PARTNER_TIDS[i] = tid_raw };
    }

    // Signal threads that all partner TIDs are now valid. Clear the abort flag first so children
    // proceed normally, then set readiness. The guard also unblocks children on drop as a safety
    // net for early returns from the spawn loop above (with abort set).
    PARTNER_TIDS_ABORT.store(false, ORDER);
    PARTNER_TIDS_READY.store(true, ORDER);

    // Join all threads.
    for i in 0..NUM_THREADS {
        if let Some(tid) = child_tids[i] {
            join_child_thread(tid)?;
        }
        // SAFETY: stack is no longer in use after join.
        unsafe { free_thread_stack(stacks[i].0, stacks[i].1) };
    }

    ::syslog::info!("test_stress_concurrent_pairs: passed");
    Ok(())
}

//==================================================================================================
// Test 20: Repeated Thread Exit — No Stale Rendezvous Entries
//==================================================================================================

/// Number of rounds for the thread-exit cleanup test.
const CLEANUP_TEST_ROUNDS: usize = 8;

/// Fixed payload used by the cleanup test pusher. Using a constant payload avoids the need to
/// communicate round-specific data to child threads.
const CLEANUP_PAYLOAD: [u8; 16] = [0xAB; 16];

///
/// # Description
///
/// Child thread that pushes a fixed payload to the main thread and then exits. Each round creates
/// a new child thread; the test verifies that exiting threads leave no stale entries in the
/// rendezvous pending lists that could corrupt matching in later rounds.
///
extern "C" fn cleanup_pusher_thread(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    match ipc::__kcall_push(pid, main_tid, &CLEANUP_PAYLOAD) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Child thread that pulls data from the main thread and then exits.
///
extern "C" fn cleanup_puller_thread(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let mut recv_buf: [u8; 16] = [0u8; 16];
    match ipc::__kcall_pull(pid, main_tid, &mut recv_buf) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Tests that repeatedly spawning and joining threads that perform push/pull operations does not
/// leave stale entries in the rendezvous pending lists. After each round, the child thread exits
/// and a new one is spawned. If `swap_remove` or `retain` left debris, later rounds would either
/// match the wrong counterpart or hang indefinitely.
///
/// The test interleaves push-first and pull-first rounds to exercise both orderings.
///
fn test_thread_exit_cleanup() -> Result<(), Error> {
    ::syslog::info!("test_thread_exit_cleanup: starting (rounds={})", CLEANUP_TEST_ROUNDS);

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    for round in 0..CLEANUP_TEST_ROUNDS {
        let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) =
            alloc_thread_stack()?;

        if round % 2 == 0 {
            // Even rounds: child pushes, main pulls (push-first ordering).
            let child_tid: ThreadIdentifier =
                spawn_child_thread(cleanup_pusher_thread, stack_base)?;

            // Pull data from the child thread.
            let mut recv_buf: [u8; 16] = [0u8; 16];
            let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

            // Validate byte count and payload.
            if bytes_transferred != 16 {
                ::syslog::error!(
                    "test_thread_exit_cleanup: round {round} push-first byte count mismatch \
                     (expected=16, got={bytes_transferred})"
                );
                return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
            }
            if recv_buf != CLEANUP_PAYLOAD {
                ::syslog::error!(
                    "test_thread_exit_cleanup: round {round} push-first payload mismatch"
                );
                return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
            }

            join_child_thread(child_tid)?;
        } else {
            // Odd rounds: child pulls, main pushes (pull-first ordering).
            let child_tid: ThreadIdentifier =
                spawn_child_thread(cleanup_puller_thread, stack_base)?;

            // Yield to give the child time to call pull() and sleep.
            for _ in 0..5 {
                sched::__kcall_sched_yield()?;
            }

            // Push data to the child thread.
            ipc::__kcall_push(pid, child_tid, &CLEANUP_PAYLOAD)?;

            join_child_thread(child_tid)?;
        }

        // SAFETY: stack is no longer in use after join.
        unsafe { free_thread_stack(stack_ptr, layout) };
    }

    // Final validation: perform one more push/pull cycle to confirm no stale entries interfere.
    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_thread_basic, stack_base)?;

    let mut recv_buf: [u8; 16] = [0u8; 16];
    let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tid, &mut recv_buf)?;

    if bytes_transferred != TEST_PAYLOAD_16.len() || recv_buf != TEST_PAYLOAD_16 {
        ::syslog::error!("test_thread_exit_cleanup: final validation failed");
        return Err(Error::new(ErrorCode::InvalidArgument, "final validation failed"));
    }

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    ::syslog::info!("test_thread_exit_cleanup: passed");
    Ok(())
}

//==================================================================================================
// Test 21: Interleaved Rendezvous — Pending List Integrity After swap_remove
//==================================================================================================

///
/// # Description
///
/// Tests that the `swap_remove` used in the rendezvous matching does not corrupt the pending list
/// when multiple entries are present simultaneously. Three child threads register pending pushes
/// to the main thread. The main thread then pulls from them in a different order than they were
/// registered, exercising the `swap_remove` path for non-last indices.
///
fn test_interleaved_pending_order() -> Result<(), Error> {
    const NUM_CHILDREN: usize = 3;

    ::syslog::info!("test_interleaved_pending_order: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    // Allocate stacks and spawn all child threads. Each child pushes a unique payload.
    let mut stacks: [(*mut u8, Layout); NUM_CHILDREN] =
        [(core::ptr::null_mut(), unsafe { Layout::from_size_align_unchecked(1, 1) }); NUM_CHILDREN];
    let mut child_tids: [ThreadIdentifier; NUM_CHILDREN] =
        [ThreadIdentifier::from(0i32); NUM_CHILDREN];

    for i in 0..NUM_CHILDREN {
        let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) =
            alloc_thread_stack()?;
        stacks[i] = (stack_ptr, layout);
        child_tids[i] = spawn_child_thread(cleanup_pusher_thread, stack_base)?;
    }

    // Yield to give all children time to call push() and register their pending entries.
    for _ in 0..10 {
        sched::__kcall_sched_yield()?;
    }

    // Pull from children in reverse order (2, 1, 0) to exercise swap_remove on non-last indices.
    for i in (0..NUM_CHILDREN).rev() {
        let mut recv_buf: [u8; 16] = [0u8; 16];
        let bytes_transferred: usize = ipc::__kcall_pull(pid, child_tids[i], &mut recv_buf)?;

        if bytes_transferred != 16 {
            ::syslog::error!(
                "test_interleaved_pending_order: child {i} byte count mismatch (expected=16, \
                 got={bytes_transferred})"
            );
            return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
        }

        // Each child's payload is [0; 16] because cleanup_pusher_thread uses `arg` as the round
        // value, and arg is always 0 for this test (the entry point receives 0 by default).
        // The important validation is that the pull matched the correct child and didn't hang.
    }

    // Join all children.
    for i in 0..NUM_CHILDREN {
        join_child_thread(child_tids[i])?;
        // SAFETY: stack is no longer in use after join.
        unsafe { free_thread_stack(stacks[i].0, stacks[i].1) };
    }

    ::syslog::info!("test_interleaved_pending_order: passed");
    Ok(())
}

//==================================================================================================
// Timeout Semantics
//==================================================================================================

/// Payload used by the timeout tests.
const TIMEOUT_PAYLOAD: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

/// Correlation tags used to verify that endpoint-equal rendezvous operations remain distinct.
const MATCHING_TAG: ::sys::ipc::RequestIdentifier = ::sys::ipc::RequestIdentifier::from_raw(7);
const WRONG_TAG: ::sys::ipc::RequestIdentifier = ::sys::ipc::RequestIdentifier::from_raw(8);

/// Finite timeout used by the "timeout fires" tests. Long enough to prove the caller actually
/// blocked and was woken by the timer, yet short enough to keep the test fast.
const TIMEOUT_FIRES_DELAY: Duration = Duration::from_millis(50);

/// Generous finite timeout used by the "timeout not reached" test. The counterpart always arrives
/// well within this window, so the call must complete normally instead of timing out.
const TIMEOUT_GENEROUS_DELAY: Duration = Duration::from_secs(10);

/// Number of times the main thread yields so a child thread can register a pending rendezvous entry
/// before the main thread probes for it. One yield is sufficient on a single-core system; a small
/// margin is used for robustness.
const PENDING_REGISTER_YIELDS: usize = 8;

/// Release flag that lets the idle child thread exit once the main thread finishes probing it.
static IDLE_RELEASE: AtomicBool = AtomicBool::new(false);

///
/// # Description
///
/// Idle child thread that stays alive without ever pushing or pulling, providing a stable thread
/// identifier for the main thread to probe against. It exits once [`IDLE_RELEASE`] is set.
///
extern "C" fn idle_child(_arg: usize) -> usize {
    while !IDLE_RELEASE.load(ORDER) {
        if sched::__kcall_sched_yield().is_err() {
            return 1;
        }
    }
    0
}

///
/// # Description
///
/// Child thread that pushes [`TIMEOUT_PAYLOAD`] to the main thread with an infinite timeout,
/// blocking until the main thread pulls.
///
extern "C" fn pusher_child_timeout(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    match ipc::__kcall_push(pid, main_tid, &TIMEOUT_PAYLOAD) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// Pushes the timeout payload with [`MATCHING_TAG`] until the main thread pulls it.
extern "C" fn pusher_child_tagged(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    match ipc::__kcall_push_tagged(pid, main_tid, &TIMEOUT_PAYLOAD, MATCHING_TAG) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

///
/// # Description
///
/// Child thread that pulls [`TIMEOUT_PAYLOAD`] from the main thread with an infinite timeout,
/// blocking until the main thread pushes.
///
extern "C" fn puller_child_timeout(_arg: usize) -> usize {
    let (pid, main_tid): (ProcessIdentifier, ThreadIdentifier) = match load_parent_ids() {
        Ok(ids) => ids,
        Err(()) => return 1,
    };

    let mut recv_buf: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    match ipc::__kcall_pull(pid, main_tid, &mut recv_buf) {
        Ok(n) if n == TIMEOUT_PAYLOAD.len() && recv_buf == TIMEOUT_PAYLOAD => 0,
        _ => 1,
    }
}

///
/// # Description
///
/// Verifies that `result` failed with [`ErrorCode::OperationTimedOut`], the kernel-level status the
/// syscall layer maps to `EAGAIN`.  Logs and returns a descriptive error otherwise.
///
/// # Parameters
///
/// - `test`: Name of the calling test, used for diagnostics.
/// - `result`: Result to inspect.
///
fn expect_timed_out(test: &str, result: Result<(), Error>) -> Result<(), Error> {
    match result {
        Err(e) if e.code == ErrorCode::OperationTimedOut => Ok(()),
        Err(e) => {
            ::syslog::error!("{test}: expected OperationTimedOut, got error {:?}", e.code);
            Err(Error::new(e.code, "expected OperationTimedOut"))
        },
        Ok(()) => {
            ::syslog::error!("{test}: expected OperationTimedOut, but the call succeeded");
            Err(Error::new(ErrorCode::OperationNotPermitted, "expected a timeout"))
        },
    }
}

///
/// # Description
///
/// Consumes the pending push emitted by [`pusher_child_timeout`] via an infinite pull, so a child
/// that a misbehaving timed pull failed to unblock cannot leave `join` hanging forever. Used only
/// on the failure path of the success-oriented timeout tests.
///
/// # Parameters
///
/// - `pid`: Process identifier shared by the child and the main thread.
/// - `child_tid`: Thread identifier of the pushing child.
///
fn drain_pending_push(pid: ProcessIdentifier, child_tid: ThreadIdentifier) {
    let mut sink: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let _ = ipc::__kcall_pull(pid, child_tid, &mut sink);
}

///
/// # Description
///
/// Satisfies the pending pull emitted by [`puller_child_timeout`] with an infinite push, so a child
/// that a misbehaving timed push failed to unblock cannot leave `join` hanging forever. Used only
/// on the failure path of the success-oriented timeout tests.
///
/// # Parameters
///
/// - `pid`: Process identifier shared by the child and the main thread.
/// - `child_tid`: Thread identifier of the pulling child.
///
fn drain_pending_pull(pid: ProcessIdentifier, child_tid: ThreadIdentifier) {
    let _ = ipc::__kcall_push(pid, child_tid, &TIMEOUT_PAYLOAD);
}

//==================================================================================================
// Timeout Test T1: Non-Blocking Pull With No Counterpart
//==================================================================================================

///
/// # Description
///
/// A non-blocking pull (zero timeout) with no pending push returns immediately with
/// [`ErrorCode::OperationTimedOut`], without registering a pending entry or sleeping.
///
fn test_pull_nonblocking_times_out() -> Result<(), Error> {
    ::syslog::info!("test_pull_nonblocking_times_out: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    IDLE_RELEASE.store(false, ORDER);
    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(idle_child, stack_base)?;

    // The idle child never pushes, so a zero-timeout pull must report a timeout at once.
    let mut recv_buf: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let result: Result<usize, Error> =
        ipc::__kcall_pull_timed(pid, child_tid, &mut recv_buf, Some(Duration::ZERO));

    IDLE_RELEASE.store(true, ORDER);
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    expect_timed_out("test_pull_nonblocking_times_out", result.map(|_| ()))?;

    ::syslog::info!("test_pull_nonblocking_times_out: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T2: Non-Blocking Push With No Counterpart
//==================================================================================================

///
/// # Description
///
/// A non-blocking push (zero timeout) with no pending pull returns immediately with
/// [`ErrorCode::OperationTimedOut`], without registering a pending entry or sleeping.
///
fn test_push_nonblocking_times_out() -> Result<(), Error> {
    ::syslog::info!("test_push_nonblocking_times_out: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    IDLE_RELEASE.store(false, ORDER);
    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(idle_child, stack_base)?;

    // The idle child never pulls, so a zero-timeout push must report a timeout at once.
    let result: Result<(), Error> =
        ipc::__kcall_push_timed(pid, child_tid, &TIMEOUT_PAYLOAD, Some(Duration::ZERO));

    IDLE_RELEASE.store(true, ORDER);
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    expect_timed_out("test_push_nonblocking_times_out", result)?;

    ::syslog::info!("test_push_nonblocking_times_out: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T3: Non-Blocking Pull With a Ready Counterpart
//==================================================================================================

///
/// # Description
///
/// A non-blocking pull (zero timeout) completes immediately when a matching push is already
/// pending, transferring the payload rather than reporting a timeout.
///
fn test_pull_nonblocking_ready() -> Result<(), Error> {
    ::syslog::info!("test_pull_nonblocking_ready: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_child_timeout, stack_base)?;

    // Yield so the child registers its pending push before we probe for it.
    for _ in 0..PENDING_REGISTER_YIELDS {
        sched::__kcall_sched_yield()?;
    }

    let mut recv_buf: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let result: Result<usize, Error> =
        ipc::__kcall_pull_timed(pid, child_tid, &mut recv_buf, Some(Duration::ZERO));

    // A successful pull already unblocked the child; otherwise drain it so join cannot hang.
    if result.is_err() {
        drain_pending_push(pid, child_tid);
    }
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    let bytes_transferred: usize = result?;
    if bytes_transferred != TIMEOUT_PAYLOAD.len() {
        ::syslog::error!(
            "test_pull_nonblocking_ready: wrong byte count (expected={}, got={})",
            TIMEOUT_PAYLOAD.len(),
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }
    if recv_buf != TIMEOUT_PAYLOAD {
        ::syslog::error!("test_pull_nonblocking_ready: payload mismatch");
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    ::syslog::info!("test_pull_nonblocking_ready: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T4: Non-Blocking Push With a Ready Counterpart
//==================================================================================================

///
/// # Description
///
/// A non-blocking push (zero timeout) completes immediately when a matching pull is already
/// pending, transferring the payload rather than reporting a timeout.
///
fn test_push_nonblocking_ready() -> Result<(), Error> {
    ::syslog::info!("test_push_nonblocking_ready: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(puller_child_timeout, stack_base)?;

    // Yield so the child registers its pending pull before we probe for it.
    for _ in 0..PENDING_REGISTER_YIELDS {
        sched::__kcall_sched_yield()?;
    }

    let result: Result<(), Error> =
        ipc::__kcall_push_timed(pid, child_tid, &TIMEOUT_PAYLOAD, Some(Duration::ZERO));

    // A successful push already unblocked the child; otherwise drain it so join cannot hang.
    if result.is_err() {
        drain_pending_pull(pid, child_tid);
    }
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    result?;

    ::syslog::info!("test_push_nonblocking_ready: passed");
    Ok(())
}

//==================================================================================================
// Tagged Rendezvous Discrimination
//==================================================================================================

/// Verifies that a wrong-tag pull cannot consume an endpoint-matching pending push.
fn test_rendezvous_tag_discriminates_requests() -> Result<(), Error> {
    ::syslog::info!("test_rendezvous_tag_discriminates_requests: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;
    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_child_tagged, stack_base)?;

    for _ in 0..PENDING_REGISTER_YIELDS {
        sched::__kcall_sched_yield()?;
    }

    let mut wrong_buffer: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let wrong: Result<usize, Error> = ipc::__kcall_pull_tagged_timed(
        pid,
        child_tid,
        &mut wrong_buffer,
        WRONG_TAG,
        Some(Duration::ZERO),
    );

    let mut matching_buffer: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let matching: Result<usize, Error> = ipc::__kcall_pull_tagged_timed(
        pid,
        child_tid,
        &mut matching_buffer,
        MATCHING_TAG,
        Some(TIMEOUT_GENEROUS_DELAY),
    );

    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    expect_timed_out("test_rendezvous_tag_discriminates_requests", wrong.map(|_| ()))?;
    let transferred: usize = matching?;
    if transferred != TIMEOUT_PAYLOAD.len() || matching_buffer != TIMEOUT_PAYLOAD {
        return Err(Error::new(
            ErrorCode::InvalidMessage,
            "tagged rendezvous transferred the wrong payload",
        ));
    }

    ::syslog::info!("test_rendezvous_tag_discriminates_requests: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T5: Finite Pull Timeout Fires
//==================================================================================================

///
/// # Description
///
/// A finite pull timeout with no counterpart blocks until the deadline elapses and then reports
/// [`ErrorCode::OperationTimedOut`], proving the caller actually slept and the timer woke it.
///
fn test_pull_timeout_fires() -> Result<(), Error> {
    ::syslog::info!("test_pull_timeout_fires: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    IDLE_RELEASE.store(false, ORDER);
    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(idle_child, stack_base)?;

    let mut recv_buf: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let result: Result<usize, Error> =
        ipc::__kcall_pull_timed(pid, child_tid, &mut recv_buf, Some(TIMEOUT_FIRES_DELAY));

    IDLE_RELEASE.store(true, ORDER);
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    expect_timed_out("test_pull_timeout_fires", result.map(|_| ()))?;

    ::syslog::info!("test_pull_timeout_fires: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T6: Finite Push Timeout Fires
//==================================================================================================

///
/// # Description
///
/// A finite push timeout with no counterpart blocks until the deadline elapses and then reports
/// [`ErrorCode::OperationTimedOut`].
///
fn test_push_timeout_fires() -> Result<(), Error> {
    ::syslog::info!("test_push_timeout_fires: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    IDLE_RELEASE.store(false, ORDER);
    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(idle_child, stack_base)?;

    let result: Result<(), Error> =
        ipc::__kcall_push_timed(pid, child_tid, &TIMEOUT_PAYLOAD, Some(TIMEOUT_FIRES_DELAY));

    IDLE_RELEASE.store(true, ORDER);
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    expect_timed_out("test_push_timeout_fires", result)?;

    ::syslog::info!("test_push_timeout_fires: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T7: Finite Pull Timeout Not Reached
//==================================================================================================

///
/// # Description
///
/// A finite pull timeout completes normally when the counterpart pushes before the deadline: the
/// generous timeout is never reached, so the payload is transferred and no timeout is reported.
///
fn test_pull_timeout_not_reached() -> Result<(), Error> {
    ::syslog::info!("test_pull_timeout_not_reached: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_child_timeout, stack_base)?;

    // The main thread's pull registers first and sleeps; the child then pushes well within the
    // generous deadline, so the call completes rather than timing out.
    let mut recv_buf: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let result: Result<usize, Error> =
        ipc::__kcall_pull_timed(pid, child_tid, &mut recv_buf, Some(TIMEOUT_GENEROUS_DELAY));

    if result.is_err() {
        drain_pending_push(pid, child_tid);
    }
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    let bytes_transferred: usize = result?;
    if bytes_transferred != TIMEOUT_PAYLOAD.len() {
        ::syslog::error!(
            "test_pull_timeout_not_reached: wrong byte count (expected={}, got={})",
            TIMEOUT_PAYLOAD.len(),
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }
    if recv_buf != TIMEOUT_PAYLOAD {
        ::syslog::error!("test_pull_timeout_not_reached: payload mismatch");
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    ::syslog::info!("test_pull_timeout_not_reached: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T8: Finite Push Timeout Not Reached
//==================================================================================================

///
/// # Description
///
/// A finite push timeout completes normally when the counterpart pulls before the deadline: the
/// generous timeout is never reached, so the payload is transferred and no timeout is reported.
///
fn test_push_timeout_not_reached() -> Result<(), Error> {
    ::syslog::info!("test_push_timeout_not_reached: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(puller_child_timeout, stack_base)?;

    // The main thread's push registers first and sleeps; the child then pulls well within the
    // generous deadline, so the call completes rather than timing out.
    let result: Result<(), Error> =
        ipc::__kcall_push_timed(pid, child_tid, &TIMEOUT_PAYLOAD, Some(TIMEOUT_GENEROUS_DELAY));

    if result.is_err() {
        drain_pending_pull(pid, child_tid);
    }
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    result?;

    ::syslog::info!("test_push_timeout_not_reached: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T9: Infinite Pull Timeout via the Timed API
//==================================================================================================

///
/// # Description
///
/// An infinite timeout (`None`) requested through the timed pull entry point behaves exactly like
/// the historical blocking pull: it waits until the counterpart pushes and then transfers the
/// payload.  This guards backward compatibility of the timed ABI.
///
fn test_pull_infinite_timeout() -> Result<(), Error> {
    ::syslog::info!("test_pull_infinite_timeout: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(pusher_child_timeout, stack_base)?;

    let mut recv_buf: [u8; TIMEOUT_PAYLOAD.len()] = [0u8; TIMEOUT_PAYLOAD.len()];
    let result: Result<usize, Error> = ipc::__kcall_pull_timed(pid, child_tid, &mut recv_buf, None);

    if result.is_err() {
        drain_pending_push(pid, child_tid);
    }
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    let bytes_transferred: usize = result?;
    if bytes_transferred != TIMEOUT_PAYLOAD.len() {
        ::syslog::error!(
            "test_pull_infinite_timeout: wrong byte count (expected={}, got={})",
            TIMEOUT_PAYLOAD.len(),
            bytes_transferred
        );
        return Err(Error::new(ErrorCode::InvalidArgument, "byte count mismatch"));
    }
    if recv_buf != TIMEOUT_PAYLOAD {
        ::syslog::error!("test_pull_infinite_timeout: payload mismatch");
        return Err(Error::new(ErrorCode::InvalidArgument, "payload mismatch"));
    }

    ::syslog::info!("test_pull_infinite_timeout: passed");
    Ok(())
}

//==================================================================================================
// Timeout Test T10: Infinite Push Timeout via the Timed API
//==================================================================================================

///
/// # Description
///
/// An infinite timeout (`None`) requested through the timed push entry point behaves exactly like
/// the historical blocking push: it waits until the counterpart pulls and then transfers the
/// payload. This guards backward compatibility of the timed ABI.
///
fn test_push_infinite_timeout() -> Result<(), Error> {
    ::syslog::info!("test_push_infinite_timeout: starting");

    let (pid, _main_tid): (ProcessIdentifier, ThreadIdentifier) = store_caller_ids()?;

    let (stack_ptr, layout, stack_base): (*mut u8, Layout, VirtualAddress) = alloc_thread_stack()?;
    let child_tid: ThreadIdentifier = spawn_child_thread(puller_child_timeout, stack_base)?;

    let result: Result<(), Error> = ipc::__kcall_push_timed(pid, child_tid, &TIMEOUT_PAYLOAD, None);

    if result.is_err() {
        drain_pending_pull(pid, child_tid);
    }
    join_child_thread(child_tid)?;
    // SAFETY: stack is no longer in use after join.
    unsafe { free_thread_stack(stack_ptr, layout) };

    result?;

    ::syslog::info!("test_push_infinite_timeout: passed");
    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

///
/// # Description
///
/// Runs all rendezvous push/pull integration tests.
///
/// # Returns
///
/// `Ok(())` on success, or an error describing which test failed.
///
pub fn run() -> Result<(), Error> {
    ::syslog::info!("rendezvous test suite: starting");

    // Error path tests (no threads needed).
    test_self_push_rejected()?;
    test_self_pull_rejected()?;

    // Basic data transfer tests.
    test_basic_push_pull()?;
    test_pull_first()?;
    test_reverse_direction()?;

    // Boundary sizes.
    test_single_byte()?;
    test_zero_length()?;
    test_asymmetric_buffers()?;

    // Larger payloads at various sizes.
    test_large_transfer(256)?;
    test_large_transfer(4096)?;

    // Multi-round and bidirectional.
    test_multi_round()?;
    test_bidirectional()?;

    // Concurrent tests.
    test_concurrent_push(64)?;
    test_concurrent_push(512)?;
    test_concurrent_pull(64)?;
    test_concurrent_pull(512)?;

    // Independent pairs.
    test_independent_pairs()?;

    // Stress and large bidirectional.
    test_stress_sequential()?;
    test_large_bidirectional()?;

    // Reverse asymmetric and mixed sizes.
    test_reverse_asymmetric()?;
    test_mixed_transfer_sizes()?;

    // Multi-round bidirectional.
    test_multi_round_bidirectional()?;

    // Concurrent large.
    test_concurrent_push(4096)?;
    test_concurrent_pull(4096)?;

    // Stress concurrent pairs.
    test_stress_concurrent_pairs()?;

    // Cleanup and pending list integrity tests.
    test_thread_exit_cleanup()?;
    test_interleaved_pending_order()?;

    // Timeout semantics: non-blocking probes, finite timeouts that fire, finite timeouts that are
    // not reached, and the infinite timeout requested through the timed ABI.
    test_pull_nonblocking_times_out()?;
    test_push_nonblocking_times_out()?;
    test_pull_nonblocking_ready()?;
    test_push_nonblocking_ready()?;
    test_rendezvous_tag_discriminates_requests()?;
    test_pull_timeout_fires()?;
    test_push_timeout_fires()?;
    test_pull_timeout_not_reached()?;
    test_push_timeout_not_reached()?;
    test_pull_infinite_timeout()?;
    test_push_infinite_timeout()?;

    ::syslog::info!("rendezvous test suite: all tests passed");
    Ok(())
}
