// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    poll::message::{
        PollRequest,
        PollResponse,
    },
    safe::RawFileDescriptor,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
use ::sys::{
    error::Error,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::ffi::{
    c_int,
    c_short,
};
use sys::{
    error::ErrorCode,
    kcall::ipc,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Events that can be polled for.
#[derive(Debug)]
pub struct PollEvents(c_short);

impl From<c_short> for PollEvents {
    fn from(value: c_short) -> Self {
        PollEvents(value)
    }
}

impl From<PollEvents> for c_short {
    fn from(value: PollEvents) -> Self {
        value.0
    }
}

impl From<&c_short> for PollEvents {
    fn from(value: &c_short) -> Self {
        PollEvents(*value)
    }
}
impl From<&PollEvents> for c_short {
    fn from(value: &PollEvents) -> Self {
        value.0
    }
}

/// Timeout for polling operations.
#[derive(Debug)]
pub struct PollTimeout(c_int);

impl From<c_int> for PollTimeout {
    fn from(value: c_int) -> Self {
        PollTimeout(value)
    }
}
impl From<PollTimeout> for c_int {
    fn from(value: PollTimeout) -> Self {
        value.0
    }
}

/// A pollable file descriptor.
#[derive(Debug)]
pub struct PollFd {
    fd: RawFileDescriptor,
    events: PollEvents,
}

impl PollFd {
    /// Creates a new `PollFd` with the given file descriptor and events.
    pub fn new(fd: RawFileDescriptor, events: PollEvents) -> Self {
        PollFd { fd, events }
    }

    /// Returns the file descriptor.
    pub fn fd(&self) -> RawFileDescriptor {
        self.fd
    }

    /// Returns the input events.
    pub fn events(&self) -> &PollEvents {
        &self.events
    }
}

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
/// - `fds`: Slice of `PollFd` structures describing the file descriptors to poll.
/// - `timeout`: Timeout in milliseconds. A negative value means infinite timeout.
///
/// # Returns
///
/// Upon success, returns a tuple containing the number of file descriptors that are ready for I/O
/// and a vector of events that occurred on each file descriptor. If `zero` is returned, the timeout
/// expired without any file descriptor becoming ready. Upon failure, an `Error` is returned.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `fds` points to a valid array of pollfd structures of length `nfds`.
///
pub fn poll(
    fds: &[PollFd],
    timeout: PollTimeout,
) -> Result<Vec<(RawFileDescriptor, PollEvents)>, Error> {
    ::syslog::trace!("poll(): fds={fds:?}, timeout={timeout:?}");

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let events: Vec<i16> = fds.iter().map(|fd| fd.events.0).collect();
    let poll_fds: Vec<RawFileDescriptor> = fds.iter().map(|fd| fd.fd).collect();
    let timeout: i32 = timeout.into();
    let request: Message = PollRequest::build(tid, &poll_fds, &events, timeout)?;
    ipc::send(&request)?;

    // Receive response.
    let response: Message = ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        let reason: &str = "poll() failed";
        ::syslog::error!("poll(): failed (fds={fds:?}, timeout={timeout:?}, status={:?})", {
            response.status
        });

        // Parse error.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, reason)),
            Err(error) => {
                ::syslog::warn!("poll(): failed to parse error code (error={error:?})");
                Err(Error::new(ErrorCode::TryAgain, reason))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::PollResponse => {
                let message: PollResponse = PollResponse::from_bytes(message.payload);
                let nready: i32 = message.nready.into();
                let mut ready: Vec<(RawFileDescriptor, PollEvents)> =
                    Vec::with_capacity(nready as usize);

                for i in 0..nready as usize {
                    ready.push((
                        message.fds[i] as RawFileDescriptor,
                        PollEvents(message.revents[i] as c_short),
                    ));
                }

                Ok(ready)
            },
            // Response was not successfully parsed.
            header => {
                ::syslog::error!(
                    "poll(): invalid response (fds={fds:?}, timeout={timeout:?}, \
                     header={header:?})"
                );
                Err(Error::new(ErrorCode::InvalidMessage, "poll() failed"))
            },
        }
    }
}
