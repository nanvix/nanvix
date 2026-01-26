// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    netinet::in_::Protocol,
    sys::socket::{
        AddressFamily,
        SocketType,
    },
    ErrorCode,
};
use ::core::slice;
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a pair of connected sockets. The `socketpair()` function creates an unnamed pair of
/// connected sockets in the specified domain, of the specified type, and using the specified
/// protocol. The two sockets are identical and can be used for bidirectional communication
/// between processes. This function is typically used to create an interprocess communication
/// channel where both endpoints can send and receive data.
///
/// # Parameters
///
/// - `domain`: Specifies the communication domain (address family) in which the sockets will be
///   created. For socket pairs, this is typically `AF_UNIX` for local communication.
/// - `typ`: Specifies the socket type, which determines the semantics of communication. Common
///   values include `SOCK_STREAM` for reliable, connection-oriented communication, and `SOCK_DGRAM`
///   for unreliable, connectionless communication.
/// - `protocol`: Specifies a particular protocol to be used with the sockets. Normally, only a
///   single protocol exists to support a particular socket type within a given protocol family.
///   A value of `0` selects the default protocol for the given domain and type.
/// - `socket_fds`: Pointer to an array of two integers where the file descriptors of the created
///   sockets will be stored. The first element receives the descriptor for the first socket, and
///   the second element receives the descriptor for the second socket.
///
/// # Returns
///
/// The `socketpair()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error. Upon successful completion, the two socket descriptors are stored in
/// the array pointed to by `socket_fds`.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `socket_fds` points to a valid array of at least two `c_int` integers.
/// - `socket_fds` remains valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn socketpair(
    domain: c_int,
    typ: c_int,
    protocol: c_int,
    socket_fds: *mut c_int,
) -> c_int {
    // Check if `socket_fds` is invalid.
    if socket_fds.is_null() {
        ::syslog::error!(
            "socketpair(): invalid sockets array (domain={domain:?}, typ={typ:?}, \
             protocol={protocol:?}, socket_fds={socket_fds:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    } else if !(socket_fds as usize).is_multiple_of(::core::mem::size_of::<c_int>()) {
        ::syslog::error!(
            "socketpair(): invalid sockets array alignment (domain={domain:?}, typ={typ:?}, \
             protocol={protocol:?}, socket_fds={socket_fds:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Reconstruct array.
    let socket_fds: &mut [c_int] = slice::from_raw_parts_mut(socket_fds, 2);

    // Attempt to convert socket address family.
    let domain: AddressFamily = match domain.try_into() {
        Ok(domain) => domain,
        Err(error) => {
            ::syslog::error!(
                "socketpair(): {error:?} (domain={domain:?}, typ={typ:?}, protocol={protocol:?}, \
                 socket_fds={socket_fds:?})"
            );
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    // Attempt to convert socket type.
    let typ: SocketType = match typ.try_into() {
        Ok(typ) => typ,
        Err(error) => {
            ::syslog::error!(
                "socketpair(): {error:?} (domain={domain:?}, typ={typ:?}, protocol={protocol:?}, \
                 socket_fds={socket_fds:?})"
            );
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    // Attempt to convert socket protocol.
    let protocol: Protocol = match protocol.try_into() {
        Ok(protocol) => protocol,
        Err(error) => {
            ::syslog::error!(
                "socketpair(): {error:?} (domain={domain:?}, typ={typ:?}, protocol={protocol:?}, \
                 socket_fds={socket_fds:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create socket pair and check for errors.
    match crate::sys::socket::syscall::socketpair(domain, typ, protocol, socket_fds) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "socketpair(): {error:?} (domain={domain:?}, typ={typ:?}, protocol={protocol:?}, \
                 socket_fds={socket_fds:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
