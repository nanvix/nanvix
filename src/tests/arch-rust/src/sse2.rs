// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # SSE/SSE2 System Level Tests
//!
//! This program provides system-level tests for SSE (Streaming SIMD Extensions) and
//! SSE2 capabilities in the Nanvix operating system. The tests exercise:
//!
//! - CPU feature detection for SSE, SSE2, and FXSAVE/FXRSTOR support
//! - Basic SSE operations (single precision floating point vector operations)
//! - Basic SSE2 operations (integer and double precision vector operations)
//!
//! All tests use inline assembly to ensure that vector instructions are not
//! compiled away by the optimizer, providing a true test of the underlying
//! hardware and OS support.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::test;
use ::arch::cpu::cpuid;
use ::core::arch::asm;

//==================================================================================================
// Constants
//==================================================================================================

/// Expected number of elements in SSE vector (128-bit / 32-bit = 4 elements).
const SSE_VECTOR_SIZE: usize = 4;

//==================================================================================================
// Test Functions
//==================================================================================================

///
/// # Description
///
/// Tests SSE2 feature detection.
///
/// # Return Values
///
/// This function returns `true` if the test passes, and `false` otherwise. This function may panic
/// if an unrecoverable error occurs.
///
pub(crate) fn test_sse2_detection() -> bool {
    cpuid::has_sse2()
}

///
/// # Description
///
/// Tests SSE2 addition operations using inline assembly.
///
/// # Return Values
///
/// This function returns `true` if the test passes, and `false` otherwise. This function may panic
/// if an unrecoverable error occurs.
///
fn test_sse2_addition() -> bool {
    // Check if SSE2 is not supported.
    if !cpuid::has_sse2() {
        ::syslog::warn!("test_sse2_addition(): sse2 is not supported, skipping tests");
        return true;
    }

    // Test data for SSE2 integer operations.
    let mut result: [i32; SSE_VECTOR_SIZE] = [0; SSE_VECTOR_SIZE];
    let data1: [i32; SSE_VECTOR_SIZE] = [10, 20, 30, 40];
    let data2: [i32; SSE_VECTOR_SIZE] = [1, 2, 3, 4];

    unsafe {
        // Load data into XMM registers and perform integer addition.
        asm!(
            // Load first vector into xmm0.
            "movdqu xmm0, [{}]",
            // Load second vector into xmm1.
            "movdqu xmm1, [{}]",
            // Add the integer vectors.
            "paddd xmm0, xmm1",
            // Store result.
            "movdqu [{}], xmm0",
            in(reg) data1.as_ptr(),
            in(reg) data2.as_ptr(),
            in(reg) result.as_mut_ptr(),
            out("xmm0") _,
            out("xmm1") _,
            options(nostack)
        );
    }

    // Check if result matches expected values.
    const EXPECTED_RESULT: [i32; SSE_VECTOR_SIZE] = [11, 22, 33, 44];
    for i in 0..SSE_VECTOR_SIZE {
        if result[i] != EXPECTED_RESULT[i] {
            ::syslog::error!(
                "test_sse2_addition(): wrong result (index={i}, got={}, expected={})",
                result[i],
                EXPECTED_RESULT[i]
            );
            return false;
        }
    }

    ::syslog::info!("SSE2 integer addition test passed");
    true
}

///
/// # Description
///
/// Tests SSE2 capabilities.
///
/// # Return Values
///
/// This function returns `true` if the test passes, and `false` otherwise. This function may panic
/// if an unrecoverable error occurs.
///
pub fn test_sse2() -> bool {
    let mut all_passed: bool = true;

    all_passed &= test!(test_sse2_detection());
    all_passed &= test!(test_sse2_addition());

    all_passed
}
