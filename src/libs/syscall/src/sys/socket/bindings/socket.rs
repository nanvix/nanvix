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
};
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates an endpoint for communication and returns a file descriptor for the new socket.
/// The `socket()` function creates an unbound socket in the communication domain specified by
/// `domain`, of the type specified by `typ`, using the protocol specified by `protocol`. The
/// socket is created with no name. Sockets are typically used for client-server communication,
/// where the server binds to a well-known address and clients connect to it.
///
/// # Parameters
///
/// - `domain`: Specifies the communication domain (address family) in which the socket will be
///   created. Common values include `AF_INET` for IPv4, `AF_INET6` for IPv6, and `AF_UNIX` for
///   local communication.
/// - `typ`: Specifies the socket type, which determines the semantics of communication. Common
///   values include `SOCK_STREAM` for reliable, connection-oriented communication, and `SOCK_DGRAM`
///   for unreliable, connectionless communication.
/// - `protocol`: Specifies a particular protocol to be used with the socket. Normally, only a
///   single protocol exists to support a particular socket type within a given protocol family.
///   A value of `0` selects the default protocol for the given domain and type.
///
/// # Returns
///
/// On success, the `socket()` function returns a non-negative file descriptor for the new socket.
/// On error, it returns `-1` and sets `errno` to indicate the error. The socket descriptor can
/// be used in subsequent system calls that operate on sockets.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn socket(domain: c_int, typ: c_int, protocol: c_int) -> c_int {
    // Attempt to convert socket address family.
    let domain: AddressFamily = match AddressFamily::try_from(domain) {
        Ok(domain) => domain,
        Err(error) => {
            ::syslog::error!(
                "socket(): {error:?} (domain={domain:?}, type={typ:?}, protocol={protocol:?})"
            );
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    // Attempt to convert socket type.
    let typ: SocketType = match SocketType::try_from(typ) {
        Ok(typ) => typ,
        Err(error) => {
            ::syslog::error!(
                "socket(): {error:?} (domain={domain:?}, type={typ:?}, protocol={protocol:?})"
            );
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    // Attempt to convert socket protocol.
    let protocol: Protocol = match Protocol::try_from(protocol) {
        Ok(protocol) => protocol,
        Err(error) => {
            ::syslog::error!(
                "socket(): {error:?} (domain={domain:?}, type={typ:?}, protocol={protocol:?})"
            );
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    // Create socket and check for errors.
    match crate::sys::socket::syscall::socket(domain, typ, protocol) {
        Ok(sockfd) => sockfd,
        Err(error) => {
            ::syslog::error!(
                "socket(): {error:?} (domain={domain:?}, type={typ:?}, protocol={protocol:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
