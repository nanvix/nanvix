// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::alloc::vec::Vec;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::{
    c_int,
    c_short,
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
/// Upon success, this function returns a tuple containing the number of file descriptors that are
/// ready for I/O and a vector of events that occurred on each file descriptor. If `zero` is
/// returned, the timeout expired without any file descriptor becoming ready. On failure, this
/// function returns an error.
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
    let reason: &str = "poll() is not available";
    ::syslog::warn!("poll(): {reason} (fds={fds:?}, timeout={timeout:?})");
    Err(Error::new(ErrorCode::OperationNotSupported, reason))
}
