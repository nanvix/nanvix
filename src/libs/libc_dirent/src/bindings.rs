// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_int,
    c_void,
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens a directory stream positioned at the first entry, for the directory associated with
/// the already-open file descriptor `fd`.
///
/// # Parameters
///
/// - `fd`: An open file descriptor referring to a directory.
///
/// # Returns
///
/// On success returns a pointer to an opaque `DIR` stream object. On failure returns a null
/// pointer and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// It exists so that consumers which only reference the symbol (notably libstdc++'s
/// `std::filesystem` directory iterators) link successfully; such callers treat the null
/// return as "directory could not be opened". A future implementation should take ownership of
/// `fd` and return a `DIR` stream backed by the directory it refers to.
///
/// # Safety
///
/// This function is safe to call with any integer; passing a descriptor that is not currently
/// open does not change the (stub) behaviour.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn fdopendir(_fd: c_int) -> *mut c_void {
    ::syslog::debug!("fdopendir(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    core::ptr::null_mut()
}
