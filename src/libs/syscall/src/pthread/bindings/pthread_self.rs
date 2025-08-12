// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::pthread_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the thread identifier of the calling thread.
///
/// # Returns
///
/// The thread identifier of the calling thread.
///
#[unsafe(no_mangle)]
pub extern "C" fn pthread_self() -> pthread_t {
    crate::pthread::pthread_self()
}
