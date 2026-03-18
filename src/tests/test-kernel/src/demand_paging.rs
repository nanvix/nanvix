// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Demand Paging Regression Tests
//!
//! This module verifies that the kernel correctly demand-pages the user stack beyond the initially
//! mapped region.  At process creation the kernel maps only
//! [`USER_STACK_MIN_SIZE`](config::memory_layout::USER_STACK_MIN_SIZE) bytes of the total
//! [`USER_STACK_SIZE`](config::memory_layout::USER_STACK_SIZE)-byte stack.  Accessing addresses
//! deeper than the initial mapping triggers a page fault that the kernel resolves transparently by
//! mapping a new page.  If demand paging is broken the process will triple-fault instead.

//==================================================================================================
// Imports
//==================================================================================================

use ::config::memory_layout::{
    USER_STACK_MIN_SIZE,
    USER_STACK_SIZE,
};
use ::sys::error::Error;

//==================================================================================================
// Constants
//==================================================================================================

/// Page size in bytes.
const PAGE_SIZE: usize = ::arch::mem::PAGE_SIZE;

/// Size of the basic demand-paging buffer — 1.5× the initial mapping so it always crosses into
/// demand-paged territory regardless of the configured [`USER_STACK_MIN_SIZE`].
const BASIC_BUF_SIZE: usize = USER_STACK_MIN_SIZE + USER_STACK_MIN_SIZE / 2;

/// Number of pages to touch in the incremental test — 2× the initial mapping.
const INCREMENTAL_PAGES: usize = (USER_STACK_MIN_SIZE * 2) / PAGE_SIZE;

/// Size of the large integrity buffer — 4× the initial mapping, capped at half the total stack
/// size.  This exercises demand paging across many pages beyond the initial mapping.
const INTEGRITY_BUF_SIZE: usize = if USER_STACK_MIN_SIZE * 4 < USER_STACK_SIZE {
    USER_STACK_MIN_SIZE * 4
} else {
    USER_STACK_SIZE / 2
};

/// Size of the per-frame local buffer in the recursion test.
const RECURSION_FRAME_SIZE: usize = PAGE_SIZE;

/// Recursion depth for the deep-recursion test.  Each frame uses [`RECURSION_FRAME_SIZE`] bytes,
/// and we recurse deep enough to exceed [`USER_STACK_MIN_SIZE`] by at least 50%.
const RECURSION_DEPTH: usize =
    (USER_STACK_MIN_SIZE + USER_STACK_MIN_SIZE / 2) / RECURSION_FRAME_SIZE;

// The basic buffer must be larger than the initially mapped region so that writing to it triggers
// at least one demand-paged page fault.
static_assert::assert_eq!(BASIC_BUF_SIZE > USER_STACK_MIN_SIZE);
// The basic buffer must fit within the total stack so it does not overflow into unmappable space.
static_assert::assert_eq!(BASIC_BUF_SIZE <= USER_STACK_SIZE);
// The incremental test must span more bytes than the initial mapping to guarantee that some of the
// per-page touches land on demand-paged pages.
static_assert::assert_eq!(INCREMENTAL_PAGES * PAGE_SIZE > USER_STACK_MIN_SIZE);
// The incremental test must not exceed the total stack capacity.
static_assert::assert_eq!(INCREMENTAL_PAGES * PAGE_SIZE <= USER_STACK_SIZE);
// The integrity buffer must exceed the initial mapping to exercise demand paging across many pages.
static_assert::assert_eq!(INTEGRITY_BUF_SIZE > USER_STACK_MIN_SIZE);
// The integrity buffer must remain within the total stack limit.
static_assert::assert_eq!(INTEGRITY_BUF_SIZE <= USER_STACK_SIZE);
// The total stack consumed by recursion must exceed the initial mapping so that deeper frames
// trigger demand-paged faults.
static_assert::assert_eq!(RECURSION_DEPTH * RECURSION_FRAME_SIZE > USER_STACK_MIN_SIZE);
// The total stack consumed by recursion must not overflow the maximum stack size.
static_assert::assert_eq!(RECURSION_DEPTH * RECURSION_FRAME_SIZE <= USER_STACK_SIZE);
// Sanity: the initial mapping must be strictly smaller than the total stack; otherwise there is
// nothing to demand-page and these tests are meaningless.
static_assert::assert_eq!(USER_STACK_MIN_SIZE < USER_STACK_SIZE);

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Fills `buf` with a deterministic, position-dependent pattern.
fn fill_pattern(buf: &mut [u8]) {
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = ((i.wrapping_mul(37).wrapping_add(7)) & 0xFF) as u8;
    }
}

/// Verifies that `buf` matches the deterministic pattern produced by [`fill_pattern`].
///
/// Returns `true` if and only if every byte matches.
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
// Test Cases
//==================================================================================================

/// Allocate a [`BASIC_BUF_SIZE`]-byte buffer on the stack (exceeds [`USER_STACK_MIN_SIZE`]), fill
/// it with a deterministic pattern, and verify the contents.  If demand paging is not working the
/// write to the unmapped region will cause a fatal page fault.
fn test_basic_demand_paging() -> Result<(), Error> {
    let mut buf: [u8; BASIC_BUF_SIZE] = [0u8; BASIC_BUF_SIZE];
    // Prevent the compiler from eliding the stack allocation.
    let buf: &mut [u8; BASIC_BUF_SIZE] = core::hint::black_box(&mut buf);
    fill_pattern(buf);
    assert!(verify_pattern(buf), "basic demand-paging buffer mismatch");
    Ok(())
}

/// Touch one byte on every page across [`INCREMENTAL_PAGES`] pages of stack depth, verifying each
/// write/read.  Pages are touched individually in reverse order (toward the stack-growth direction)
/// so that demand faults are triggered one page at a time.
fn test_incremental_page_touch() -> Result<(), Error> {
    // Create a buffer large enough to span INCREMENTAL_PAGES pages.
    let mut buf: [u8; INCREMENTAL_PAGES * PAGE_SIZE] = [0u8; INCREMENTAL_PAGES * PAGE_SIZE];
    let buf: &mut [u8; INCREMENTAL_PAGES * PAGE_SIZE] = core::hint::black_box(&mut buf);

    // Touch one byte per page in reverse order (toward stack growth direction).
    for page in (0..INCREMENTAL_PAGES).rev() {
        let offset: usize = page * PAGE_SIZE;
        let canary: u8 = (page & 0xFF) as u8;
        buf[offset] = canary;
    }

    // Verify all written canaries.
    for page in 0..INCREMENTAL_PAGES {
        let offset: usize = page * PAGE_SIZE;
        let expected: u8 = (page & 0xFF) as u8;
        assert!(buf[offset] == expected, "incremental page touch mismatch at page {}", page);
    }

    Ok(())
}

/// Recursive function that consumes [`RECURSION_FRAME_SIZE`] bytes of stack per frame.  At each
/// level a local buffer is filled with a canary value and verified after the recursive call
/// returns, ensuring that demand-paged pages remain valid across nested calls.
#[inline(never)]
fn recurse(depth: usize) -> Result<(), Error> {
    if depth == 0 {
        return Ok(());
    }

    let mut frame_buf: [u8; RECURSION_FRAME_SIZE] = [0u8; RECURSION_FRAME_SIZE];
    let frame_buf: &mut [u8; RECURSION_FRAME_SIZE] = core::hint::black_box(&mut frame_buf);

    // Write a canary pattern unique to this recursion level.
    let canary: u8 = (depth & 0xFF) as u8;
    for byte in frame_buf.iter_mut() {
        *byte = canary;
    }

    // Recurse deeper.
    recurse(depth - 1)?;

    // Verify the canary is still intact after returning from the deeper call — this catches
    // corruption from incorrectly mapped pages.
    for (i, &byte) in frame_buf.iter().enumerate() {
        assert!(byte == canary, "recursion canary corrupted at depth {} byte {}", depth, i);
        // Prevent the loop from being optimized away.
        core::hint::black_box(i);
    }

    Ok(())
}

/// Exercise demand paging via natural call-stack growth by recursing [`RECURSION_DEPTH`] levels
/// deep with [`RECURSION_FRAME_SIZE`]-byte frames, exceeding [`USER_STACK_MIN_SIZE`].
fn test_deep_recursion() -> Result<(), Error> {
    recurse(RECURSION_DEPTH)
}

/// Allocate an [`INTEGRITY_BUF_SIZE`]-byte buffer on the stack, fill it with a deterministic
/// pattern, and verify the entire contents.  This exercises demand paging across many pages beyond
/// the initial [`USER_STACK_MIN_SIZE`] mapping.
fn test_stack_write_read_integrity() -> Result<(), Error> {
    let mut buf: [u8; INTEGRITY_BUF_SIZE] = [0u8; INTEGRITY_BUF_SIZE];
    let buf: &mut [u8; INTEGRITY_BUF_SIZE] = core::hint::black_box(&mut buf);
    fill_pattern(buf);
    assert!(verify_pattern(buf), "large stack integrity buffer mismatch");
    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all demand-paging regression tests.
///
/// # Returns
///
/// `Ok(())` if all tests pass.
///
/// # Errors
///
/// Returns the first error encountered by any test case.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-kernel: demand_paging: starting demand-paging regression tests");

    test_basic_demand_paging()?;
    ::syslog::info!("test-kernel: demand_paging: PASS - basic_demand_paging");

    test_incremental_page_touch()?;
    ::syslog::info!("test-kernel: demand_paging: PASS - incremental_page_touch");

    test_deep_recursion()?;
    ::syslog::info!("test-kernel: demand_paging: PASS - deep_recursion");

    test_stack_write_read_integrity()?;
    ::syslog::info!("test-kernel: demand_paging: PASS - stack_write_read_integrity");

    ::syslog::info!("test-kernel: demand_paging: all tests passed");

    Ok(())
}
