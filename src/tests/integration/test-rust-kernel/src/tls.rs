// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # TLS (Thread-Local Storage) Stress Tests
//!
//! This module exercises the `set_thread_data_area()` / `get_thread_data_area()`
//! kernel calls to verify that the GDT entry for the running thread's TLS
//! segment is updated **immediately** (without requiring a context switch) and
//! that various edge cases are handled correctly.
//!
//! ## Bug Description (regression test)
//!
//! The kernel's `set_thread_data_area()` kcall previously only stored the TDA
//! pointer in the thread struct.  The GDT entry (which backs the `%gs` segment
//! register) was only updated during context switches in
//! `ContextInformation::switch()`.  In single-threaded processes, no context
//! switch ever occurred, so `%gs:0x0` resolved to a NULL dereference, causing
//! a page fault. This broke any code using `thread_local` variables (e.g., the
//! V8 JavaScript engine's `GetCurrentThreadId()`).

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::boxed::Box;
use ::config::memory_layout::USER_THREAD_STACK_SIZE;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm,
    mm::VirtualAddress,
    pm::{
        ThreadCreateArgs,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Magic value written to the TDA region, used to verify `%gs`-based reads.
const TDA_MAGIC: u32 = 0xDEAD_BEEF;

/// Number of `u32` slots in each heap-allocated TDA buffer. Four slots
/// (16 bytes) suffice for the multiple-offsets test; extra slots provide margin.
const TDA_SLOTS: usize = 16;

/// Number of iterations for the repeated-overwrite and reassignment stress tests.
const REPEAT_ITERATIONS: u32 = 64;

/// Magic value for the main thread's TDA in cross-thread tests.
const MAIN_TDA_MAGIC: u32 = 0xFEED_FACE;

/// Magic value for the worker thread's TDA in cross-thread tests.
const WORKER_TDA_MAGIC: u32 = 0xCAFE_BABE;

/// Expected exit status returned by the worker thread.
const WORKER_EXIT_STATUS: usize = 0xDEAD_BEEF;

//==================================================================================================
// Types
//==================================================================================================

/// A heap-allocated TDA buffer with guaranteed `u32` alignment.
type TdaBuffer = Box<[u32; TDA_SLOTS]>;

//==================================================================================================
// Helper Functions
//==================================================================================================

/// # Description
///
/// Reads the 32-bit value at `%gs:0x0` using inline assembly.
///
/// # Safety
///
/// The caller must ensure that the `%gs` segment register points to a valid,
/// readable memory region of at least 4 bytes.
// NOTE: `#[inline(never)]` prevents the compiler from inlining this function,
// ensuring the `%gs:0x0` read is not reordered relative to the preceding
// `write_volatile` and `set_thread_data_area` calls.
#[inline(never)]
unsafe fn read_gs_offset_0() -> u32 {
    let value: u32;
    // The thread data area is referenced through %gs on x86 (GDT-based) and
    // through %fs (FS_BASE MSR) on x86_64.
    #[cfg(target_arch = "x86")]
    core::arch::asm!(
        "movl %gs:0x0, {out:e}",
        out = out(reg) value,
        options(nostack, preserves_flags, att_syntax),
    );
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!(
        "movl %fs:0x0, {out:e}",
        out = out(reg) value,
        options(nostack, preserves_flags, att_syntax),
    );
    value
}

/// # Description
///
/// Reads the 32-bit value at `%gs:<offset>` for an arbitrary byte offset.
///
/// # Safety
///
/// The caller must ensure that the `%gs` segment register points to a valid,
/// readable memory region that covers at least `offset + 4` bytes.
// NOTE: The offset is added to the base at runtime, so the function cannot be
// constant-folded away.
#[inline(never)]
#[cfg_attr(target_arch = "x86_64", allow(asm_sub_register))]
unsafe fn read_gs_at(offset: u32) -> u32 {
    let value: u32;
    // The thread data area is referenced through %gs on x86 (GDT-based) and
    // through %fs (FS_BASE MSR) on x86_64.
    #[cfg(target_arch = "x86")]
    core::arch::asm!(
        "movl %gs:({off}), {out:e}",
        off = in(reg) offset,
        out = out(reg) value,
        options(nostack, preserves_flags, att_syntax),
    );
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!(
        "movl %fs:({off}), {out:e}",
        off = in(reg) offset,
        out = out(reg) value,
        options(nostack, preserves_flags, att_syntax),
    );
    value
}

/// # Description
///
/// Asserts that a read value matches an expected value, returning an error on mismatch.
///
/// # Parameters
///
/// - `test`: A short label for the failing test (used in the error message).
/// - `expected`: The value that was expected.
/// - `actual`: The value that was actually read.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] when `expected != actual`.
fn assert_eq_or_fail(test: &'static str, expected: u32, actual: u32) -> Result<(), Error> {
    if actual != expected {
        ::syslog::error!(
            "test-kernel: tls: {test}: FAIL - expected {expected:#010x}, got {actual:#010x}"
        );
        return Err(Error::new(ErrorCode::InvalidArgument, test));
    }
    Ok(())
}

/// # Description
///
/// Sets the given heap-allocated buffer as the thread data area and returns the previous TDA
/// pointer.
///
/// # Parameters
///
/// - `tda`: The heap-allocated buffer to install as the TDA.
///
/// # Returns
///
/// The original TDA pointer that was active before this call.
///
/// # Errors
///
/// Propagates errors from `get_thread_data_area()` or `set_thread_data_area()`.
fn setup_tda(tda: &mut TdaBuffer) -> Result<*mut u8, Error> {
    let original_tda: *mut u8 = pm::__kcall_get_thread_data_area()?;
    pm::__kcall_set_thread_data_area(tda.as_mut_ptr().cast::<u8>())?;
    Ok(original_tda)
}

/// # Description
///
/// Restores the original TDA pointer.
///
/// # Parameters
///
/// - `original_tda`: The original TDA pointer to restore.
///
/// # Errors
///
/// Propagates errors from `set_thread_data_area()`.
fn teardown_tda(original_tda: *mut u8) -> Result<(), Error> {
    pm::__kcall_set_thread_data_area(original_tda)?;
    Ok(())
}

//==================================================================================================
// Test Cases
//==================================================================================================

/// # Description
///
/// Test 1 – Basic GDT update (regression test).
///
/// Writes a magic value at offset 0 of a heap-allocated TDA buffer, sets it
/// as the TDA via `set_thread_data_area()`, then reads `%gs:0x0` and checks it
/// matches.
///
/// # Errors
///
/// Returns an error if the read value does not match the written magic.
fn test_basic_gdt_update() -> Result<(), Error> {
    let mut tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut tda)?;

    let base: *mut u32 = tda.as_mut_ptr();
    unsafe { core::ptr::write_volatile(base, TDA_MAGIC) };
    let gs_value: u32 = unsafe { read_gs_offset_0() };

    teardown_tda(original)?;
    assert_eq_or_fail("basic_gdt_update", TDA_MAGIC, gs_value)
}

/// # Description
///
/// Test 2 – Multiple offsets within a TDA buffer.
///
/// Writes distinct values at byte offsets 0, 4, 8 and 12 inside the TDA buffer
/// and verifies each one via `%gs`-relative reads. This exercises the segment
/// base calculation for non-zero offsets.
///
/// # Errors
///
/// Returns an error if any read value does not match the corresponding write.
fn test_multiple_offsets() -> Result<(), Error> {
    let mut tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut tda)?;

    // Write four distinct values at consecutive 4-byte slots.
    let values: [u32; 4] = [0xAAAA_AAAA, 0xBBBB_BBBB, 0xCCCC_CCCC, 0xDDDD_DDDD];
    let base: *mut u32 = tda.as_mut_ptr();
    for (i, &val) in values.iter().enumerate() {
        unsafe { core::ptr::write_volatile(base.add(i), val) };
    }

    // Read back via %gs at each offset and verify.
    let mut result: Result<(), Error> = Ok(());
    for (i, &expected) in values.iter().enumerate() {
        let offset: u32 = (i * 4) as u32;
        let actual: u32 = unsafe { read_gs_at(offset) };
        if let Err(e) = assert_eq_or_fail("multiple_offsets", expected, actual) {
            result = Err(e);
            break;
        }
    }

    teardown_tda(original)?;
    result
}

/// # Description
///
/// Test 3 – TDA reassignment between two buffers.
///
/// Allocates two TDA buffers on the heap, writes different magic values to
/// each, then switches the TDA from A to B and verifies that `%gs:0x0` reflects
/// the new buffer immediately.
///
/// # Errors
///
/// Returns an error if `%gs:0x0` does not reflect the active buffer's value.
fn test_reassignment() -> Result<(), Error> {
    let original: *mut u8 = pm::__kcall_get_thread_data_area()?;

    let mut tda_a: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let mut tda_b: TdaBuffer = Box::new([0u32; TDA_SLOTS]);

    let magic_a: u32 = 0x1111_1111;
    let magic_b: u32 = 0x2222_2222;
    unsafe {
        core::ptr::write_volatile(tda_a.as_mut_ptr(), magic_a);
        core::ptr::write_volatile(tda_b.as_mut_ptr(), magic_b);
    }

    // Point TDA to buffer A and verify.
    pm::__kcall_set_thread_data_area(tda_a.as_mut_ptr().cast::<u8>())?;
    let val_a: u32 = unsafe { read_gs_offset_0() };
    let mut result: Result<(), Error> = assert_eq_or_fail("reassignment_a", magic_a, val_a);

    // Switch TDA to buffer B and verify (only if buffer A succeeded).
    if result.is_ok() {
        pm::__kcall_set_thread_data_area(tda_b.as_mut_ptr().cast::<u8>())?;
        let val_b: u32 = unsafe { read_gs_offset_0() };
        result = assert_eq_or_fail("reassignment_b", magic_b, val_b);
    }

    // Cleanup.
    pm::__kcall_set_thread_data_area(original)?;
    result
}

/// # Description
///
/// Test 4 – Clear and restore TDA.
///
/// Sets a TDA, clears it by passing a null pointer, then re-sets it to the
/// same buffer and verifies the value is still accessible via `%gs:0x0`.
///
/// # Errors
///
/// Returns an error if the round-trip clear/restore produces an incorrect read.
fn test_clear_and_restore() -> Result<(), Error> {
    let mut tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut tda)?;

    unsafe { core::ptr::write_volatile(tda.as_mut_ptr(), TDA_MAGIC) };

    // Verify before clearing.
    let before: u32 = unsafe { read_gs_offset_0() };
    let mut result: Result<(), Error> =
        assert_eq_or_fail("clear_restore_before", TDA_MAGIC, before);

    if result.is_ok() {
        // Clear TDA (set to null).
        pm::__kcall_set_thread_data_area(core::ptr::null_mut())?;

        // Verify get_thread_data_area() returns null after clearing.
        // NOTE: we intentionally do NOT read %gs:0x0 here because the segment
        // base has been zeroed and the selector set to null, so a %gs-relative
        // read would fault (GP or PF). The getter is the only safe way to
        // verify that the TDA was actually cleared.
        let cleared_tda: *mut u8 = pm::__kcall_get_thread_data_area()?;
        if !cleared_tda.is_null() {
            ::syslog::error!(
                "test-kernel: tls: clear_restore: TDA not null after clear ({cleared_tda:?})"
            );
            result = Err(Error::new(ErrorCode::InvalidArgument, "TDA not null after clear"));
        }
    }

    if result.is_ok() {
        // Restore TDA and verify the magic value is accessible again.
        pm::__kcall_set_thread_data_area(tda.as_mut_ptr().cast::<u8>())?;
        let after: u32 = unsafe { read_gs_offset_0() };
        result = assert_eq_or_fail("clear_restore_after", TDA_MAGIC, after);
    }

    teardown_tda(original)?;
    result
}

/// # Description
///
/// Test 5 – Repeated overwrite stress.
///
/// Sets the TDA once and then repeatedly overwrites the value at offset 0,
/// reading it back via `%gs:0x0` on each iteration. This stresses the
/// segment-register caching behaviour under rapid writes.
///
/// # Errors
///
/// Returns an error if any iteration produces a mismatched read.
fn test_repeated_overwrite() -> Result<(), Error> {
    let mut tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut tda)?;

    let mut result: Result<(), Error> = Ok(());
    let base: *mut u32 = tda.as_mut_ptr();
    for i in 0..REPEAT_ITERATIONS {
        let magic: u32 = 0xCAFE_0000 | i;
        unsafe { core::ptr::write_volatile(base, magic) };
        let actual: u32 = unsafe { read_gs_offset_0() };
        if let Err(e) = assert_eq_or_fail("repeated_overwrite", magic, actual) {
            result = Err(e);
            break;
        }
    }

    teardown_tda(original)?;
    result
}

/// # Description
///
/// Test 6 – `get_thread_data_area()` round-trip.
///
/// Calls `set_thread_data_area()` with a known address and then verifies that
/// `get_thread_data_area()` returns exactly that address. Tests the getter and
/// setter symmetry.
///
/// # Errors
///
/// Returns an error if the returned pointer does not match the one that was set.
fn test_get_set_round_trip() -> Result<(), Error> {
    let mut tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut tda)?;
    let expected_ptr: *mut u8 = tda.as_mut_ptr().cast::<u8>();

    let returned_tda: *mut u8 = pm::__kcall_get_thread_data_area()?;
    if returned_tda != expected_ptr {
        ::syslog::error!(
            "test-kernel: tls: get_set_round_trip: expected {expected_ptr:?}, got {returned_tda:?}"
        );
        teardown_tda(original)?;
        return Err(Error::new(ErrorCode::InvalidArgument, "get/set round-trip mismatch"));
    }

    teardown_tda(original)
}

/// # Description
///
/// Test 7 – Repeated reassignment stress.
///
/// Alternates the TDA between two heap-allocated buffers for several
/// iterations, verifying that each switch immediately takes effect.
///
/// # Errors
///
/// Returns an error if any iteration produces a mismatched read.
fn test_repeated_reassignment() -> Result<(), Error> {
    let original: *mut u8 = pm::__kcall_get_thread_data_area()?;

    let mut tda_a: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let mut tda_b: TdaBuffer = Box::new([0u32; TDA_SLOTS]);

    let magic_a: u32 = 0xA0A0_A0A0;
    let magic_b: u32 = 0xB0B0_B0B0;
    unsafe {
        core::ptr::write_volatile(tda_a.as_mut_ptr(), magic_a);
        core::ptr::write_volatile(tda_b.as_mut_ptr(), magic_b);
    }

    let mut result: Result<(), Error> = Ok(());
    for _ in 0..REPEAT_ITERATIONS {
        pm::__kcall_set_thread_data_area(tda_a.as_mut_ptr().cast::<u8>())?;
        let val_a: u32 = unsafe { read_gs_offset_0() };
        if let Err(e) = assert_eq_or_fail("repeated_reassign_a", magic_a, val_a) {
            result = Err(e);
            break;
        }

        pm::__kcall_set_thread_data_area(tda_b.as_mut_ptr().cast::<u8>())?;
        let val_b: u32 = unsafe { read_gs_offset_0() };
        if let Err(e) = assert_eq_or_fail("repeated_reassign_b", magic_b, val_b) {
            result = Err(e);
            break;
        }
    }

    pm::__kcall_set_thread_data_area(original)?;
    result
}

//==================================================================================================
// Worker Thread Infrastructure
//==================================================================================================

/// # Description
///
/// A heap-allocated user stack with RAII cleanup.
struct WorkerStack {
    /// Raw pointer to the base of the allocated memory.
    ptr: *mut u8,
    /// Layout used for allocation (needed for deallocation).
    layout: core::alloc::Layout,
    /// Size of the stack in bytes.
    size: usize,
}

impl WorkerStack {
    /// # Description
    ///
    /// Allocates a new worker stack of the given size, aligned to pointer width.
    ///
    /// # Parameters
    ///
    /// - `size`: Stack size in bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::OutOfMemory`] if the allocation fails.
    fn new(size: usize) -> Result<Self, Error> {
        let layout: core::alloc::Layout =
            core::alloc::Layout::from_size_align(size, core::mem::align_of::<usize>())
                .map_err(|_| Error::new(ErrorCode::OutOfMemory, "invalid stack layout"))?;
        // SAFETY: layout is non-zero and properly aligned.
        let ptr: *mut u8 = unsafe { ::alloc::alloc::alloc(layout) };
        if ptr.is_null() {
            return Err(Error::new(ErrorCode::OutOfMemory, "stack allocation failed"));
        }
        Ok(Self { ptr, layout, size })
    }

    /// Returns the base address as a `VirtualAddress`.
    fn base(&self) -> VirtualAddress {
        VirtualAddress::from_raw_value(self.ptr as usize)
    }

    /// Returns the stack size.
    fn size(&self) -> usize {
        self.size
    }
}

impl Drop for WorkerStack {
    fn drop(&mut self) {
        // SAFETY: `ptr` was allocated with the same `layout`.
        unsafe { ::alloc::alloc::dealloc(self.ptr, self.layout) };
    }
}

/// # Description
///
/// Worker thread entry point for the cross-thread TDA test.
///
/// Receives a pointer to a `[u32; TDA_SLOTS]` buffer in `arg`.  Installs it as
/// the thread's TDA and verifies that `%gs:0x0` reads the expected magic value.
///
/// # Returns
///
/// [`WORKER_EXIT_STATUS`] on success, `0` on failure.
extern "C" fn tda_worker(arg: usize) -> usize {
    // `arg` is the raw address of the worker's TDA buffer.
    let tda_ptr: *mut u8 = arg as *mut u8;

    // Install the worker's TDA.
    if pm::__kcall_set_thread_data_area(tda_ptr).is_err() {
        return 0;
    }

    // Read %gs:0x0 and verify it matches the worker magic.
    let gs_value: u32 = unsafe { read_gs_offset_0() };
    if gs_value != WORKER_TDA_MAGIC {
        ::syslog::error!("tda_worker: expected {WORKER_TDA_MAGIC:#010x}, got {gs_value:#010x}");
        return 0;
    }

    WORKER_EXIT_STATUS
}

/// # Description
///
/// Builds [`ThreadCreateArgs`] for a worker thread.
///
/// # Parameters
///
/// - `stack`: The pre-allocated user stack.
/// - `entry`: The worker entry point.
/// - `arg`: The argument passed to the worker.
fn make_thread_args(
    stack: &WorkerStack,
    entry: extern "C" fn(usize) -> usize,
    arg: usize,
) -> ThreadCreateArgs {
    ThreadCreateArgs {
        user_fn: ThreadCreateArgs::NULL_USER_FN,
        user_fn_arg0: entry as usize,
        user_fn_arg1: arg,
        user_stack_base: stack.base(),
        user_stack_size: stack.size(),
        user_tda: None,
    }
}

//==================================================================================================
// Cross-Thread Test Cases
//==================================================================================================

/// # Description
///
/// Test 8 – TDA survives a create/join cycle (regression for stale `%gs` hidden cache).
///
/// Mirrors the C test in `tda.c`:
/// 1. Main thread installs a TDA with [`MAIN_TDA_MAGIC`] at offset 0.
/// 2. Verifies `%gs:0x0 == MAIN_TDA_MAGIC`.
/// 3. Spawns a worker that installs its own TDA ([`WORKER_TDA_MAGIC`]) and verifies it.
/// 4. Joins the worker.
/// 5. Reads `%gs:0x0` again and verifies it still equals [`MAIN_TDA_MAGIC`].
///
/// Without the `__context_switch` fix that saves/restores `%gs`/`%fs`, step 5
/// fails because the hidden descriptor cache retains the worker's TDA base.
///
/// # Errors
///
/// Returns an error if any step produces a mismatched read or a kcall failure.
fn test_tda_survives_create_join() -> Result<(), Error> {
    // --- Set up main thread's TDA ---
    let mut main_tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut main_tda)?;
    unsafe { core::ptr::write_volatile(main_tda.as_mut_ptr(), MAIN_TDA_MAGIC) };

    // Verify before spawning worker.
    let before: u32 = unsafe { read_gs_offset_0() };
    assert_eq_or_fail("tda_create_join_before", MAIN_TDA_MAGIC, before)?;

    // --- Set up worker TDA buffer (stays alive until after join) ---
    let mut worker_tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    unsafe { core::ptr::write_volatile(worker_tda.as_mut_ptr(), WORKER_TDA_MAGIC) };

    // --- Spawn and join worker ---
    let stack: WorkerStack = WorkerStack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs =
        make_thread_args(&stack, tda_worker, worker_tda.as_mut_ptr() as usize);
    let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;

    let mut retval: usize = 0;
    pm::__kcall_join_thread(tid, &mut retval)?;
    drop(stack);

    if retval != WORKER_EXIT_STATUS {
        ::syslog::error!(
            "tda_create_join: worker retval mismatch (expected {WORKER_EXIT_STATUS:#x}, got \
             {retval:#x})"
        );
        teardown_tda(original)?;
        return Err(Error::new(ErrorCode::InvalidArgument, "worker retval mismatch"));
    }

    // --- Critical check: main thread's %gs must still resolve to its own TDA ---
    let after: u32 = unsafe { read_gs_offset_0() };
    let result: Result<(), Error> =
        assert_eq_or_fail("tda_create_join_after", MAIN_TDA_MAGIC, after);

    teardown_tda(original)?;
    result
}

/// # Description
///
/// Test 9 – Repeated create/join cycles preserve the main thread's TDA.
///
/// Runs [`REPEAT_ITERATIONS`] create/join cycles, verifying after each one that
/// the main thread's `%gs:0x0` still reads the correct magic.  This catches
/// intermittent stale-cache bugs that only manifest after several context
/// switches.
///
/// # Errors
///
/// Returns the first error encountered.
fn test_repeated_create_join() -> Result<(), Error> {
    let mut main_tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut main_tda)?;
    unsafe { core::ptr::write_volatile(main_tda.as_mut_ptr(), MAIN_TDA_MAGIC) };

    let mut result: Result<(), Error> = Ok(());

    for i in 0..REPEAT_ITERATIONS {
        // Worker TDA with a per-iteration magic to ensure each worker writes a unique value.
        let worker_magic: u32 = 0xBB00_0000 | i;
        let mut worker_tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
        unsafe { core::ptr::write_volatile(worker_tda.as_mut_ptr(), worker_magic) };

        let stack: WorkerStack = WorkerStack::new(USER_THREAD_STACK_SIZE)?;
        let mut args: ThreadCreateArgs =
            make_thread_args(&stack, tda_worker_generic, worker_tda.as_mut_ptr() as usize);
        let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;

        let mut retval: usize = 0;
        pm::__kcall_join_thread(tid, &mut retval)?;
        drop(stack);

        // Check that the worker succeeded.
        if retval == 0 {
            ::syslog::error!("repeated_create_join: worker failed at iteration {i}");
            result = Err(Error::new(ErrorCode::InvalidArgument, "worker failed"));
            break;
        }

        // Verify main thread's TDA is intact.
        let after: u32 = unsafe { read_gs_offset_0() };
        if let Err(e) = assert_eq_or_fail("repeated_create_join", MAIN_TDA_MAGIC, after) {
            ::syslog::error!("repeated_create_join: stale %%gs at iteration {i}");
            result = Err(e);
            break;
        }
    }

    teardown_tda(original)?;
    result
}

/// # Description
///
/// Generic worker that installs whatever TDA buffer `arg` points to, reads
/// `%gs:0x0`, and returns the read value as the exit status.  The caller can
/// verify the value after joining.
extern "C" fn tda_worker_generic(arg: usize) -> usize {
    let tda_ptr: *mut u8 = arg as *mut u8;
    if pm::__kcall_set_thread_data_area(tda_ptr).is_err() {
        return 0;
    }
    let gs_value: u32 = unsafe { read_gs_offset_0() };
    gs_value as usize
}

/// # Description
///
/// Test 10 – Worker TDA is independent from main TDA.
///
/// Verifies that the worker thread reads its own TDA value (not the main
/// thread's) and that the main thread's TDA is unaffected by the worker's
/// writes.  This is the kernel-level equivalent of the C `tda.c` test that
/// checks thread-specific data isolation.
///
/// # Errors
///
/// Returns an error on any mismatch.
fn test_worker_tda_independence() -> Result<(), Error> {
    let mut main_tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    let original: *mut u8 = setup_tda(&mut main_tda)?;
    unsafe { core::ptr::write_volatile(main_tda.as_mut_ptr(), MAIN_TDA_MAGIC) };

    // Verify main's TDA before spawn.
    let before: u32 = unsafe { read_gs_offset_0() };
    assert_eq_or_fail("worker_independence_before", MAIN_TDA_MAGIC, before)?;

    // Worker gets its own distinct TDA.
    let mut worker_tda: TdaBuffer = Box::new([0u32; TDA_SLOTS]);
    unsafe { core::ptr::write_volatile(worker_tda.as_mut_ptr(), WORKER_TDA_MAGIC) };

    let stack: WorkerStack = WorkerStack::new(USER_THREAD_STACK_SIZE)?;
    let mut args: ThreadCreateArgs =
        make_thread_args(&stack, tda_worker_generic, worker_tda.as_mut_ptr() as usize);
    let tid: ThreadIdentifier = pm::__kcall_create_thread(&mut args)?;

    let mut retval: usize = 0;
    pm::__kcall_join_thread(tid, &mut retval)?;
    drop(stack);

    // Verify the worker read its own magic (not the main thread's).
    let worker_read: u32 = retval as u32;
    assert_eq_or_fail("worker_independence_worker", WORKER_TDA_MAGIC, worker_read)?;

    // Verify main's TDA is still intact.
    let after: u32 = unsafe { read_gs_offset_0() };
    let result: Result<(), Error> =
        assert_eq_or_fail("worker_independence_after", MAIN_TDA_MAGIC, after);

    teardown_tda(original)?;
    result
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// # Description
///
/// Runs all TLS stress tests.
///
/// Executes ten test cases that exercise the `set_thread_data_area()` and
/// `get_thread_data_area()` kernel calls in various scenarios:
///
/// 1. Basic GDT update (original regression test).
/// 2. Multiple offsets within a single TDA buffer.
/// 3. Reassignment between two different TDA buffers.
/// 4. Clearing and restoring the TDA.
/// 5. Repeated overwrite of the value at offset 0.
/// 6. `get_thread_data_area()` / `set_thread_data_area()` round-trip.
/// 7. Repeated alternation between two TDA buffers.
/// 8. TDA survives a create/join cycle (stale `%gs` hidden-cache regression).
/// 9. Repeated create/join cycles preserve the main thread's TDA.
/// 10. Worker TDA is independent from the main thread's TDA.
///
/// # Returns
///
/// `Ok(())` if all tests pass.
///
/// # Errors
///
/// Returns the first error encountered by any test case.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-kernel: tls: starting TLS stress tests");

    test_basic_gdt_update()?;
    ::syslog::info!("test-kernel: tls: PASS - basic_gdt_update");

    test_multiple_offsets()?;
    ::syslog::info!("test-kernel: tls: PASS - multiple_offsets");

    test_reassignment()?;
    ::syslog::info!("test-kernel: tls: PASS - reassignment");

    test_clear_and_restore()?;
    ::syslog::info!("test-kernel: tls: PASS - clear_and_restore");

    test_repeated_overwrite()?;
    ::syslog::info!("test-kernel: tls: PASS - repeated_overwrite");

    test_get_set_round_trip()?;
    ::syslog::info!("test-kernel: tls: PASS - get_set_round_trip");

    test_repeated_reassignment()?;
    ::syslog::info!("test-kernel: tls: PASS - repeated_reassignment");

    test_tda_survives_create_join()?;
    ::syslog::info!("test-kernel: tls: PASS - tda_survives_create_join");

    test_repeated_create_join()?;
    ::syslog::info!("test-kernel: tls: PASS - repeated_create_join");

    test_worker_tda_independence()?;
    ::syslog::info!("test-kernel: tls: PASS - worker_tda_independence");

    ::syslog::info!("test-kernel: tls: all tests passed");

    Ok(())
}
