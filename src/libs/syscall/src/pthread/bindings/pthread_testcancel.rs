// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a cancellation point in the calling thread.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub extern "C" fn pthread_testcancel() {
    // Cancellation requests are not supported, so none can be pending.
}
