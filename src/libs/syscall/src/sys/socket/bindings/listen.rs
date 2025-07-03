// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
};
use ::sysapi::ffi::c_int;

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn listen(sockfd: c_int, backlog: c_int) -> c_int {
    ::syslog::trace!("listen(): sockfd={:?}, backlog={:?}", sockfd, backlog);

    match crate::sys::socket::syscall::listen(sockfd, backlog) {
        Ok(_) => 0,
        Err(e) => {
            ::syslog::error!("listen(): failed to listen on socket {:?}", e);
            *__errno_location() = e.code.get();
            -1
        },
    }
}
