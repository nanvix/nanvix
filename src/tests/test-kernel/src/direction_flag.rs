// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Direction Flag (DF) Regression Tests
//!
//! This module verifies that the kernel clears the x86 direction flag on every
//! entry path, as required by the System V ABI and all compiler-generated code.
//!
//! ## Scenario
//!
//! User-mode code may set DF=1 via the `std` instruction.  If a kernel call
//! (`int 0x81`) fires while DF is set and the kernel does not clear it, the
//! kernel's Rust handlers run with DF=1.  Compiler-generated `rep movsl`
//! instructions (used for struct copies in debug builds) then copy memory
//! **backwards**, silently corrupting scheduler linked-list nodes and heap
//! metadata.  The most visible symptom is a triple fault during
//! `LinkedList::push_back(RunnableProcess)` inside `sched_yield()`.
//!
//! The kernel must issue a `cld` instruction in every entry stub (`_do_kcall`,
//! `context_save`, etc.) before running any Rust code.  These tests exercise
//! various kernel-call paths with DF=1 to confirm that invariant holds.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::Error,
    kcall::{
        debug,
        pm,
        sched,
    },
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of iterations for the stress test.  At the ~2–4% per-call failure
/// rate observed without the fix, 256 iterations gives >99.9% probability of
/// triggering the bug if the `cld` instruction is removed.
const STRESS_ITERATIONS: u32 = 256;

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Sets the x86 direction flag (DF=1) via the `std` instruction.
///
/// After this call, `rep movsb`/`rep movsl` will copy **backwards**.  Only thin kcall wrappers
/// (whose body is a single `int 0x81` instruction) may be called while DF=1; the kernel clears DF
/// on entry, so the flag never leaks into compiler-generated code.  The caller must call
/// `clear_df()` before returning to general Rust code.
///
// NOTE: `#[inline(never)]` prevents the compiler from reordering this
// relative to the subsequent kernel call.
#[inline(never)]
fn set_df() {
    // SAFETY: Setting DF is safe in user mode; we clear it before returning
    // to normal Rust code.
    unsafe {
        core::arch::asm!("std", options(nostack, att_syntax));
    }
}

/// Clears the x86 direction flag (DF=0) via the `cld` instruction.
///
/// Must be called after every kernel call that was issued with DF=1 to
/// restore the ABI-required DF=0 state before any further Rust code runs.
// NOTE: `#[inline(never)]` prevents the compiler from reordering this
// relative to the preceding kernel call.
#[inline(never)]
fn clear_df() {
    // SAFETY: Clearing DF is always safe and restores the ABI-expected state.
    unsafe {
        core::arch::asm!("cld", options(nostack, att_syntax));
    }
}

/// Reads the current value of the direction flag from EFLAGS.
///
/// Returns `true` if DF=1 (backwards), `false` if DF=0 (forwards).
#[inline(never)]
fn read_df() -> bool {
    let eflags: u32;
    // SAFETY: Reading EFLAGS via pushfd/pop is safe in user mode.
    unsafe {
        core::arch::asm!(
            "pushfd",
            "pop {out:e}",
            out = out(reg) eflags,
            options(att_syntax),
        );
    }
    const DF_BIT: u32 = 1 << 10;
    (eflags & DF_BIT) != 0
}

//==================================================================================================
// Test Cases
//==================================================================================================

/// Test 1 – DF canary: verify `set_df()` / `clear_df()` / `read_df()` work.
///
/// Pure user-space check that the inline assembly helpers produce the expected
/// EFLAGS state.  If this fails, all subsequent tests are invalid.
fn test_df_canary() -> Result<(), Error> {
    // DF should be 0 at function entry (ABI guarantee).
    if read_df() {
        ::syslog::error!("test-kernel: direction_flag: df_canary: DF=1 at entry");
        return Err(Error::new(::sys::error::ErrorCode::InvalidArgument, "DF=1 at function entry"));
    }

    // Set DF=1 and verify.
    set_df();
    let df_after_set: bool = read_df();
    clear_df();

    if !df_after_set {
        ::syslog::error!("test-kernel: direction_flag: df_canary: std did not set DF");
        return Err(Error::new(::sys::error::ErrorCode::InvalidArgument, "std did not set DF"));
    }

    // DF should be 0 again after clear.
    if read_df() {
        ::syslog::error!("test-kernel: direction_flag: df_canary: cld did not clear DF");
        return Err(Error::new(::sys::error::ErrorCode::InvalidArgument, "cld did not clear DF"));
    }

    Ok(())
}

/// Test 2 – `gettime()` with DF=1.
///
/// Issues the `gettime` kernel call while DF=1.  The kernel copies a
/// `SystemTime` struct to the user buffer via `vmcopy_to_user`.  Validates
/// that the returned nanoseconds field is sane (< 1 billion).
fn test_gettime_with_df_set() -> Result<(), Error> {
    let mut time: SystemTime = SystemTime::EPOCH;

    set_df();
    let result: Result<(), Error> = pm::__kcall_gettime(&mut time);
    clear_df();

    result?;

    if time.nanoseconds() >= 1_000_000_000 {
        ::syslog::error!(
            "test-kernel: direction_flag: gettime_with_df_set: nanoseconds={} >= 1e9",
            time.nanoseconds()
        );
        return Err(Error::new(
            ::sys::error::ErrorCode::InvalidArgument,
            "gettime returned invalid nanoseconds",
        ));
    }

    Ok(())
}

/// Test 3 – `sched_yield()` with DF=1.
///
/// This is the **primary regression test**.  `sched_yield()` calls
/// `ProcessManager::schedule()`, which pushes the current `RunnableProcess`
/// (~300 bytes) onto the scheduler's `LinkedList` via `push_back()`.  In
/// debug builds the compiler generates `rep movsl` for this struct copy.
/// Without the `cld` fix, the backwards copy corrupts the linked-list node,
/// leading to a triple fault or data corruption.
fn test_yield_with_df_set() -> Result<(), Error> {
    set_df();
    let result: Result<(), Error> = sched::__kcall_sched_yield();
    clear_df();

    result
}

/// Test 4 – `debug()` with DF=1.
///
/// Issues the `debug` kernel call while DF=1.  The kernel copies the message
/// buffer from user space via `vmcopy_from_user`.  Validates that the call
/// succeeds (the kernel correctly interprets the UTF-8 buffer).
fn test_debug_with_df_set() -> Result<(), Error> {
    let msg: &[u8] = b"direction_flag: df-test probe";

    set_df();
    let result: Result<(), Error> = debug::__kcall_debug(msg.as_ptr(), msg.len());
    clear_df();

    result
}

/// Test 5 – Stress: repeated `sched_yield()` with DF=1.
///
/// Repeats the yield-with-DF test 256 times.  At the ~2–4% per-call failure
/// rate observed in debug builds without the fix, this gives >99.9%
/// probability of triggering the bug.  Each iteration sets DF=1, yields
/// (exercising the ~300-byte struct copy onto the scheduler linked list),
/// and clears DF.
fn test_stress_yield_with_df_set() -> Result<(), Error> {
    for i in 0..STRESS_ITERATIONS {
        set_df();
        let result: Result<(), Error> = sched::__kcall_sched_yield();
        clear_df();

        if let Err(e) = result {
            ::syslog::error!(
                "test-kernel: direction_flag: stress_yield_with_df_set: failed at iteration {i}"
            );
            return Err(e);
        }
    }

    Ok(())
}

//==================================================================================================
// Public Interface
//==================================================================================================

/// Runs all direction-flag regression tests.
///
/// # Returns
///
/// `Ok(())` if all tests pass.
///
/// # Errors
///
/// Returns the first error encountered by any test case.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-kernel: direction_flag: starting DF regression tests");

    test_df_canary()?;
    ::syslog::info!("test-kernel: direction_flag: PASS - df_canary");

    test_gettime_with_df_set()?;
    ::syslog::info!("test-kernel: direction_flag: PASS - gettime_with_df_set");

    test_yield_with_df_set()?;
    ::syslog::info!("test-kernel: direction_flag: PASS - yield_with_df_set");

    test_debug_with_df_set()?;
    ::syslog::info!("test-kernel: direction_flag: PASS - debug_with_df_set");

    test_stress_yield_with_df_set()?;
    ::syslog::info!("test-kernel: direction_flag: PASS - stress_yield_with_df_set");

    ::syslog::info!("test-kernel: direction_flag: all tests passed");

    Ok(())
}
