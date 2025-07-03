// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn socket(domain: c_int, typ: c_int, protocol: c_int) -> c_int {
    ::syslog::trace!("socket(): domain={:?}, type={:?}, protocol={:?}", domain, typ, protocol);
    // Attempt to convert socket address family.
    let domain: AddressFamily = match AddressFamily::try_from(domain) {
        Ok(domain) => domain,
        Err(_error) => {
            ::syslog::error!("socket(): invalid socket address family (domain={:?})", domain);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket type.
    let typ: SocketType = match SocketType::try_from(typ) {
        Ok(typ) => typ,
        Err(_error) => {
            ::syslog::error!("socket(): invalid socket type (type={:?})", typ);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket protocol.
    let protocol: Protocol = match Protocol::try_from(protocol) {
        Ok(protocol) => protocol,
        Err(_error) => {
            ::syslog::error!("socket(): invalid socket protocol (protocol={:?})", protocol);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create socket.
    match crate::sys::socket::syscall::socket(domain, typ, protocol) {
        Ok(sockfd) => sockfd,
        Err(error) => {
            ::syslog::error!("socket(): failed to create socket (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}