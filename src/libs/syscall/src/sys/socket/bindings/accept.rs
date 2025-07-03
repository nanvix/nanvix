// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::socket::SocketAddr,
};
use ::sysapi::ffi::c_int;
use ::sysapi::sys_socket::{
    sockaddr,
    socklen_t,
};


#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn accept(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    ::syslog::trace!("accept(): sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?}");

    match crate::sys::socket::syscall::accept(sockfd) {
        Ok((sockfd, sockaddr_)) => {
            // Store socket address, if requested.
            let (sockaddr_, len_) = From::<&SocketAddr>::from(&sockaddr_);
            if !sockaddr.is_null() {
                *sockaddr = sockaddr_;
            }

            if !len.is_null() {
                *len = len_;
            }

            sockfd
        },
        Err(error) => {
            ::syslog::error!("accept(): failed to accept connection (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
