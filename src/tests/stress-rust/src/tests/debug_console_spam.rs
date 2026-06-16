// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::StressError;
use ::sys::kcall::{
    debug::__kcall_debug as kernel_debug,
    sched::__kcall_sched_yield,
};

//==================================================================================================
// Constants
//==================================================================================================

const DEBUG_SPAM_LINES: usize = 128;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Floods the debug console with small messages to resemble noisy firmware or verbose logging that
/// might exert backpressure on debug I/O paths.
///
/// # Returns
///
/// `Ok(())` on success or an error if debug or scheduling calls fail.
///
pub fn run() -> Result<(), StressError> {
    const PAYLOADS: [&[u8]; 4] = [
        b"[stress::kcall] debug ping 0\n",
        b"[stress::kcall] debug ping 1\n",
        b"[stress::kcall] debug ping 2\n",
        b"[stress::kcall] debug ping 3\n",
    ];

    for round in 0..DEBUG_SPAM_LINES {
        let payload: &[u8] = PAYLOADS[round % PAYLOADS.len()];
        kernel_debug(payload.as_ptr(), payload.len())?;

        if round & 0x7 == 0 {
            __kcall_sched_yield()?;
        }
    }
    Ok(())
}
