// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::WorkerThreadError;
use ::sys::{
    error::ErrorCode,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::poll::{
    poll_errors::{
        POLLERR,
        POLLHUP,
        POLLNVAL,
    },
    poll_flags::{
        POLLIN,
        POLLOUT,
        POLLPRI,
        POLLRDBAND,
        POLLRDNORM,
        POLLWRBAND,
        POLLWRNORM,
    },
};
use ::syscall::poll::message::{
    PollRequest,
    PollResponse,
    NFDS_MAX,
};

//==================================================================================================
// do_poll()
//==================================================================================================

pub fn do_poll(tid: ThreadIdentifier, request: PollRequest) -> Result<Message, WorkerThreadError> {
    trace!("poll(): tid={tid:?}, request={request:?}");

    // Check if request is not valid.
    if request.nfds == 0 || request.nfds as usize > NFDS_MAX {
        error!("poll(): invalid request ({request:?})");
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }

    // Unpack request.
    let nfds: libc::nfds_t = request.nfds.into();

    let mut fds: Vec<libc::pollfd> = Vec::new();
    for i in 0..nfds as usize {
        fds.push(LibcPollFd::from_raw_nanvix_pollfd(request.fds[i], request.events[i]).0);
    }

    let timeout: libc::c_int = request.timeout;

    debug!("libc::poll(): nfds={nfds:?}, timeout={timeout:?}");
    match unsafe { libc::poll(fds.as_mut_ptr(), nfds, timeout) } {
        nready if nready >= 0 => {
            debug!("poll(): nready={nready:?}");

            match nready.try_into() {
                Ok(nready) => {
                    let mut ready_fds: Vec<i32> = Vec::with_capacity(nready as usize);
                    let mut revents: Vec<i16> = Vec::with_capacity(nready as usize);

                    for (i, fd) in fds.iter_mut().enumerate() {
                        if fd.revents != 0 {
                            ready_fds.push(fd.fd);
                            revents.push(fd.revents);

                            debug!("poll(): fd[{}] = {}, revents = {}", i, fd.fd, fd.revents);
                        } else {
                            debug!("poll(): fd[{}] = {}, no events", i, fd.fd);
                        }
                    }

                    // Build response.
                    match PollResponse::build(tid, nready, &ready_fds, &revents) {
                        Ok(response) => Ok(response),
                        Err(error) => {
                            unreachable!("poll(): failed to build response ({error:?})");
                        },
                    }
                },
                Err(_error) => {
                    unreachable!(
                        "poll(): invalid number of ready file descriptors (nread={nready:?})"
                    );
                },
            }
        },
        _ => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            error!("poll(): errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            Ok(crate::build_error(tid, error))
        },
    }
}

pub struct LibcPollFd(libc::pollfd);

fn nanvix_event_to_libc_event(events: i16) -> libc::c_short {
    let mut libc_events: libc::c_short = 0;

    if events & POLLIN != 0 {
        libc_events |= libc::POLLIN;
    }

    if events & POLLPRI != 0 {
        libc_events |= libc::POLLPRI;
    }

    if events & POLLOUT != 0 {
        libc_events |= libc::POLLOUT;
    }

    if events & POLLRDNORM != 0 {
        libc_events |= libc::POLLRDNORM;
    }

    if events & POLLRDBAND != 0 {
        libc_events |= libc::POLLRDBAND;
    }

    if events & POLLWRNORM != 0 {
        libc_events |= libc::POLLWRNORM;
    }

    if events & POLLWRBAND != 0 {
        libc_events |= libc::POLLWRBAND;
    }

    if events & POLLERR != 0 {
        libc_events |= libc::POLLERR;
    }

    if events & POLLHUP != 0 {
        libc_events |= libc::POLLHUP;
    }

    if events & POLLNVAL != 0 {
        libc_events |= libc::POLLNVAL;
    }

    libc_events
}
impl LibcPollFd {
    pub fn from_raw_nanvix_pollfd(fd: i32, events: i16) -> Self {
        let libc_events: libc::c_short = nanvix_event_to_libc_event(events);
        Self(libc::pollfd {
            fd,
            events: libc_events,
            revents: 0, // revents is not used in the request
        })
    }
}
