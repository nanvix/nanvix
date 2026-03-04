// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ErrorCode,
};
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
/// Sets options on sockets. The `setsockopt()` function provides an application program with the
/// means to control socket behavior. An application program can use `setsockopt()` to allocate
/// buffer space, control timeouts, or permit socket data broadcasts. The `level` argument specifies
/// the protocol level at which the option resides. To set options at the socket level, specify
/// `level` as `SOL_SOCKET`. To set options at other levels, supply the appropriate protocol number
/// for the protocol controlling the option.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket on which to set the option.
/// - `level`: The protocol level at which the option resides (e.g., `SOL_SOCKET` for socket-level
///   options, `IPPROTO_TCP` for TCP-level options).
/// - `optname`: The name of the option to set (e.g., `SO_REUSEADDR`, `SO_KEEPALIVE`).
/// - `optval`: Pointer to the buffer containing the option value. The type and size of this buffer
///   depends on the specific option being set.
/// - `optlen`: Length of the option value buffer in bytes.
///
/// # Returns
///
/// The `setsockopt()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Common error conditions include invalid socket descriptor, unsupported
/// option, or invalid option value.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `optval` points to a valid buffer containing at least `optlen` bytes of data.
/// - `optval` remains valid for the duration of the function call.
/// - The option value pointed to by `optval` is appropriate for the specified `level` and `optname`.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
#[allow(unreachable_code)]
pub unsafe extern "C" fn setsockopt(
    sockfd: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: socklen_t,
) -> c_int {
    #[cfg(feature = "standalone")]
    {
        let _ = (sockfd, level, optname, optval, optlen);
        *__errno_location() = ::sys::error::ErrorCode::OperationNotSupported.get();
        return -1;
    }

    // TODO: https://github.com/nanvix/nanvix/issues/471
    ::syslog::debug!("setsockopt(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
