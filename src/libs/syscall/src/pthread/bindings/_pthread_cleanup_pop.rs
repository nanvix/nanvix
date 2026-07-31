// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Pops the top cancellation cleanup handler from the calling thread's cleanup stack.
///
/// # Parameters
///
/// - `execute`: Whether to invoke the cleanup handler.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub extern "C" fn _pthread_cleanup_pop(execute: c_int) {
    crate::pthread::pthread_cleanup_pop(execute != 0);
}
