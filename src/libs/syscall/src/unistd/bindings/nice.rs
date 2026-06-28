// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the scheduling priority (nice value) of the calling process. Nanvix uses a fixed
/// scheduling policy and exposes no per-process priorities, so this call has no observable side
/// effects and reports the resulting nice value as `0`. It exists so that portable software that
/// adjusts its priority compiles, links, and runs.
///
/// # Parameters
///
/// - `inc`: The requested increment to the nice value (ignored).
///
/// # Returns
///
/// `0`, the effective nice value on Nanvix.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn nice(_inc: c_int) -> c_int {
    0
}
