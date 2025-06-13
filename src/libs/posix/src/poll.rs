// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_int,
    c_short,
    c_uint,
};
use sys::error::ErrorCode;

//==================================================================================================
// Types
//==================================================================================================

// Used for the number of file descriptors.
pub type nfds_t = c_uint;

//==================================================================================================
// Structures
//==================================================================================================

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct pollfd {
    /// The following descriptor being polled.
    pub fd: c_int,
    /// Input event flags.
    pub events: c_short,
    /// Output event flags.
    pub revents: c_short,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Waits for one of a set of file descriptors to become ready to perform I/O.
///
/// # Parameters
///
/// - `fds`: Pointer to an array of pollfd structures describing the file descriptors to poll.
/// - `nfds`: Number of file descriptors in the array.
/// - `timeout`: Timeout in milliseconds. A negative value means infinite timeout.
///
/// # Returns
///
/// Returns the number of file descriptors with events, `0` if timed out, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `fds` points to a valid array of pollfd structures of length `nfds`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    ::syslog::trace!("poll(): fds={fds:?}, nfds={nfds:?}, timeout={timeout:?}");
    ::syslog::error!("poll(): not implemented");
    ErrorCode::InvalidSysCall.get()
}
