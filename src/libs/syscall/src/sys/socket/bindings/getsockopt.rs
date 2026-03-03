// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_socket::socklen_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets options on sockets. The `getsockopt()` function retrieves the current value of a socket
/// option for the socket specified by `sockfd`. Socket options can exist at multiple protocol
/// levels, and this function allows applications to query various socket behaviors and
/// configurations.  Common socket options include buffer sizes, timeout values, and
/// protocol-specific settings.  This function is the counterpart to `setsockopt()` and is essential
/// for socket configuration management and debugging.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket for which to retrieve the option value.
/// - `level`: The protocol level at which the option resides (e.g., SOL_SOCKET for socket-level options).
/// - `optname`: The name of the option to retrieve (e.g., SO_REUSEADDR, SO_KEEPALIVE).
/// - `optval`: Pointer to the buffer where the option value will be stored. Can be NULL to query
///   only the option length.
/// - `optlen`: Pointer to the length of the option value. On input, it specifies the size of the
///   buffer pointed to by `optval`. On output, it contains the actual size of the option value.
///
/// # Returns
///
/// The `getsockopt()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - If `optval` is not NULL, it points to a valid buffer of at least `*optlen` bytes.
/// - `optlen` points to a valid `socklen_t` value.
/// - `optval` and `optlen` remain valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
#[allow(unreachable_code)]
pub unsafe extern "C" fn getsockopt(
    sockfd: c_int,
    level: c_int,
    optname: c_int,
    optval: *mut c_void,
    optlen: *mut socklen_t,
) -> c_int {
    #[cfg(feature = "standalone")]
    {
        let _ = (sockfd, level, optname, optval, optlen);
        *__errno_location() = ::sys::error::ErrorCode::OperationNotSupported.get();
        return -1;
    }

    // TODO: https://github.com/nanvix/nanvix/issues/591
    ::syslog::debug!("getsockopt(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
