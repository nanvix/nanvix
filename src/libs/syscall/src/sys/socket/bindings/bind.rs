// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ErrorCode,
    sys::socket::{
        SocketAddr,
    },
};
use ::sysapi::ffi::c_int;
use ::sysapi::sys_socket::{
    sockaddr,
    socklen_t,
};

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bind(sockfd: c_int, sockaddr: *const sockaddr, len: socklen_t) -> c_int {
    ::syslog::trace!("bind(): sockfd={:?}, sockaddr={:?}, len={:?}", sockfd, sockaddr, len);

    // Check if sock address is valid.
    if sockaddr.is_null() {
        ::syslog::error!("bind(): invalid socket address");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    ::syslog::trace!("bind(): sockaddr={:?}", unsafe { &*sockaddr });

    // Attempt to convert socket address.
    let sockaddr: SocketAddr = match SocketAddr::try_from(unsafe { &*sockaddr }) {
        Ok(sockaddr) => sockaddr,
        Err(e) => {
            ::syslog::error!("bind(): failed to convert socket address {:?}", e);
            *__errno_location() = e.code.get();
            return -1;
        },
    };

    match crate::sys::socket::syscall::bind(sockfd, &sockaddr) {
        Ok(_) => 0,
        Err(e) => {
            *__errno_location() = e.code.get();
            -1
        },
    }
}
