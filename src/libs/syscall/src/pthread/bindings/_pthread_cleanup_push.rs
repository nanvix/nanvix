// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_void;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Pushes a cancellation cleanup handler onto the calling thread's cleanup stack.
///
/// # Parameters
///
/// - `routine`: Cleanup routine.
/// - `arg`: Argument to pass to the cleanup routine.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub extern "C" fn _pthread_cleanup_push(
    routine: Option<extern "C" fn(*mut c_void)>,
    arg: *mut c_void,
) {
    if routine.is_none() {
        ::syslog::warn!("_pthread_cleanup_push(): invalid cleanup routine");
    }

    // A null routine is still pushed, so that the cleanup stack stays balanced with pops.
    crate::pthread::pthread_cleanup_push(routine, arg);
}
