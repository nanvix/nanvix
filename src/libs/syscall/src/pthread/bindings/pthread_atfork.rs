// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// TODO: add description
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pthread_atfork(
    prepare: Option<extern "C" fn()>,
    parent: Option<extern "C" fn()>,
    child: Option<extern "C" fn()>,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/483
    ::syslog::debug!(
        "pthread_atfork(): not implemented (prepare={prepare:?}, parent={parent:?}, \
         child={child:?})"
    );
    ErrorCode::InvalidSysCall.get()
}
