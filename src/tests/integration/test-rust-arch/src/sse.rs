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
/// Tests SSE feature detection.
///
/// # Return Values
///
/// This function returns `true` if the test passes, and `false` otherwise. This function may panic
/// if an unrecoverable error occurs.
///
pub(crate) fn test_sse_detection() -> bool {
    cpuid::has_sse()
}

///
/// # Description
///
/// Tests SSE addition operations using inline assembly.
///
/// # Return Values
///
/// This function returns `true` if the test passes, and `false` otherwise. This function may panic
/// if an unrecoverable error occurs.
///
fn test_sse_addition() -> bool {
    // Check if SSE is not supported.
    if !cpuid::has_sse() {
        ::syslog::warn!("test_sse_addition(): sse is not supported, skipping tests");
        return true;
    }

    // Test data for SSE operations.
    let mut result: [f32; SSE_VECTOR_SIZE] = [0.0; SSE_VECTOR_SIZE];
    let data1: [f32; SSE_VECTOR_SIZE] = [1.0, 2.0, 3.0, 4.0];
    let data2: [f32; SSE_VECTOR_SIZE] = [5.0, 6.0, 7.0, 8.0];

    unsafe {
        // Load data into XMM registers and perform addition.
        asm!(
            // Load first vector into xmm0.
            "movups xmm0, [{}]",
            // Load second vector into xmm1.
            "movups xmm1, [{}]",
            // Add the vectors.
            "addps xmm0, xmm1",
            // Store result.
            "movups [{}], xmm0",
            in(reg) data1.as_ptr(),
            in(reg) data2.as_ptr(),
            in(reg) result.as_mut_ptr(),
            out("xmm0") _,
            out("xmm1") _,
            options(nostack)
        );
    }

    // Check if result matches expected values.
    const EXPECTED_RESULT: [f32; SSE_VECTOR_SIZE] = [6.0, 8.0, 10.0, 12.0];
    for i in 0..SSE_VECTOR_SIZE {
        if (result[i] - EXPECTED_RESULT[i]).abs() > f32::EPSILON {
            ::syslog::error!(
                "test_sse_addition(): wrong result (index={i}, got={}, expected={})",
                result[i],
                EXPECTED_RESULT[i]
            );
            return false;
        }
    }

    true
}

///
/// # Description
///
/// Tests SSE capabilities.
///
/// # Return Values
///
/// This function returns `true` if all tests pass, and `false` otherwise. This function may panic
/// if an unrecoverable error occurs.
///
pub fn test_sse() -> bool {
    let mut all_passed: bool = true;

    all_passed &= test!(test_sse_detection());
    all_passed &= test!(test_sse_addition());

    all_passed
}
