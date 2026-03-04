// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    poll::syscall::{
        self,
        PollEvents,
        PollFd,
        PollTimeout,
    },
};
use ::alloc::vec::Vec;
use ::sysapi::{
    ffi::c_int,
    poll::{
        nfds_t,
        pollfd,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Waits for one of a set of file descriptors to become ready to perform I/O.
///
/// # Parameters
///
/// - `fds`: Pointer to an array of pollfd structures describing the file descriptors to poll.
/// - `nfds`: Number of file descriptors in the array.
/// - `timeout`: Timeout in milliseconds. A negative value means infinite timeout.
///
/// # Returns
///
/// Upon success, returns the number of file descriptors that are ready for I/O. If `zero`, is
/// returned, the timeout expired without any file descriptor becoming ready. Upon failure, `-1` is
/// returned, and `errno` is set to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `fds` points to a valid array of pollfd structures of length `nfds`.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    let fds: &mut [pollfd] = if nfds == 0 {
        &mut []
    } else {
        core::slice::from_raw_parts_mut(fds, nfds as usize)
    };
    let poll_fds: Vec<PollFd> = fds
        .iter()
        .map(|fd| {
            let events: PollEvents = fd.events.into();
            PollFd::new(fd.fd, events)
        })
        .collect();
    let timeout: PollTimeout = timeout.into();

    match syscall::poll(&poll_fds, timeout) {
        Ok(ready) => {
            for (fd, revent) in &ready {
                if let Some(i) = fds.iter().position(|poll_fd| poll_fd.fd == *fd) {
                    fds[i].revents = revent.into();
                }
            }

            ready.len() as c_int
        },
        Err(error) => unsafe {
            ::syslog::error!("poll(): failed (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
