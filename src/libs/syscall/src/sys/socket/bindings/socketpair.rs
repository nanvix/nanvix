// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use core::slice;
use crate::{
    errno::__errno_location,
    ErrorCode,
    sys::socket::{
        AddressFamily,
        SocketType,
    },
    netinet::in_::Protocol,
};
use ::sysapi::ffi::c_int;

///
/// # Description
///
/// Creates a pair of connected sockets.
///
/// # Parameters
///
/// - `domain`: Communication domain.
/// - `typ`: Socket type.
/// - `protocol`: Protocol.
/// - `socket_fds`: Array where the file descriptors of the sockets will be stored.
///
/// # Returns
///
/// The `socketpair()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference raw pointers.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn socketpair(
    domain: c_int,
    typ: c_int,
    protocol: c_int,
    socket_fds: *mut c_int,
) -> c_int {
    // Check if socket pair is valid.
    if socket_fds.is_null() {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Reconstruct array.
    let socket_fds: &mut [c_int] = slice::from_raw_parts_mut(socket_fds, 2);

    // Attempt to convert socket address family.
    let domain: AddressFamily = match domain.try_into() {
        Ok(domain) => domain,
        Err(_error) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket type.
    let typ: SocketType = match typ.try_into() {
        Ok(typ) => typ,
        Err(_error) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket protocol.
    let protocol: Protocol = match protocol.try_into() {
        Ok(protocol) => protocol,
        Err(_error) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::sys::socket::syscall::socketpair(domain, typ, protocol, socket_fds) {
        Ok(_) => 0,
        Err(e) => {
            *__errno_location() = e.code.get();
            -1
        },
    }
}
