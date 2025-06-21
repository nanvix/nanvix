// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    poll::{
        nfds_t,
        pollfd,
    },
};
//==================================================================================================
// Standalone Functions
//==================================================================================================

unsafe extern "C" {
    pub fn close(fd: c_int) -> c_int;
    pub fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int;
    pub fn pipe(fds: *mut c_int) -> c_int;
}
