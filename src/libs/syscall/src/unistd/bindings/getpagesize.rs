// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the size of a memory page, in bytes. This is the legacy equivalent of
/// `sysconf(_SC_PAGESIZE)` and reports the architecture's page size as configured for Nanvix.
///
/// # Returns
///
/// The size of a memory page, in bytes.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn getpagesize() -> c_int {
    // `PAGE_SIZE` is a small power of two that always fits in `c_int`; the fallback exists only to
    // satisfy the total conversion and is never expected to be taken.
    c_int::try_from(PAGE_SIZE).unwrap_or(4096)
}
