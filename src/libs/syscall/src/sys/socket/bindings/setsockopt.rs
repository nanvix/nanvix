// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ErrorCode,
};
use ::sysapi::ffi::{
    c_int,
    c_void
};
use ::sysapi::sys_socket::socklen_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets options on sockets.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `level`: The protocol level at which the option resides.
/// - `optname`: The name of the option.
/// - `optval`: Pointer to the option value.
/// - `optlen`: Length of the option value.
///
/// # Returns
///
/// The `setsockopt()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `optval` points to a valid buffer of length `optlen` (if not null).
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setsockopt(
    sockfd: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: socklen_t,
) -> c_int {
    ::syslog::trace!(
        "setsockopt(): sockfd={sockfd:?}, level={level:?}, optname={optname:?}, \
         optval={optval:?}, optlen={optlen:?}"
    );
    // TODO: https://github.com/nanvix/nanvix/issues/471
    ::syslog::error!("setsockopt(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}