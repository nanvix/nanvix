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
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::{
            LinuxDaemonLongMessage,
            LinuxDaemonMessagePart,
            MessagePartitioner,
        },
        poll::message::{
            PollRequest,
            PollResponse,
        },
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::sys::{
        ipc::Message,
        kcall::ipc,
        pm::ThreadIdentifier,
    },
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
#[allow(unreachable_code)]
pub fn poll(
    fds: &[PollFd],
    timeout: PollTimeout,
) -> Result<Vec<(RawFileDescriptor, PollEvents)>, Error> {
    ::syslog::trace!("poll(): fds={fds:?}, timeout={timeout:?}");

    // In standalone mode, poll is not available (no linuxd).
    #[cfg(feature = "standalone")]
    {
        Err(Error::new(ErrorCode::OperationNotSupported, "poll not available in standalone mode"))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    poll_linuxd(fds, timeout)
}

/// Forwards a `poll` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn poll_linuxd(
    fds: &[PollFd],
    timeout: PollTimeout,
) -> Result<Vec<(RawFileDescriptor, PollEvents)>, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let events: Vec<i16> = fds.iter().map(|fd| fd.events.0).collect();
    let poll_fds: Vec<RawFileDescriptor> = fds.iter().map(|fd| fd.fd).collect();
    let timeout: i32 = timeout.into();
    let request: PollRequest = PollRequest::new(&poll_fds, &events, timeout)?;
    let requests: Vec<Message> = request.into_parts(tid)?;
    for request in &requests {
        ipc::send(request)?;
    }

    // Compute maximum number of parts in the response.
    let capacity: usize = PollResponse::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
    let mut assembler: LinuxDaemonLongMessage = LinuxDaemonLongMessage::new(capacity)?;

    loop {
        let response: Message = ipc::recv()?;

        // Check if system call failed.
        if response.status != 0 {
            let reason: &str = "poll() failed";
            match ErrorCode::try_from(response.status) {
                Ok(error_code) => {
                    break Err(Error::new(error_code, reason));
                },
                Err(error) => {
                    ::syslog::error!(
                        "poll(): failed to parse error code (fds={fds:?}, timeout={timeout:?}, \
                         error={error:?})"
                    );
                    break Err(Error::new(ErrorCode::TryAgain, reason));
                },
            }
        }

        // Parse system call response.
        let message: LinuxDaemonMessage = match LinuxDaemonMessage::try_from_bytes(response.payload)
        {
            Ok(m) => m,
            Err(error) => {
                ::syslog::error!("poll(): {error:?} (fds={fds:?}, timeout={timeout:?})");
                break Err(error);
            },
        };

        match message.header {
            LinuxDaemonMessageHeader::PollResponsePart => {
                let part: LinuxDaemonMessagePart =
                    LinuxDaemonMessagePart::from_bytes(message.payload);

                // Add response part to message assembler and check for errors.
                if let Err(e) = assembler.add_part(part) {
                    ::syslog::error!(
                        "poll(): failed to assemble response (fds={fds:?}, timeout={timeout:?})"
                    );
                    break Err(e);
                }

                // Check if all response parts were not yet received.
                if !assembler.is_complete() {
                    continue;
                }

                let parts: Vec<LinuxDaemonMessagePart> = assembler.take_parts();

                // Assemble message.
                match PollResponse::from_parts(&parts) {
                    Ok(response) => {
                        let nready: usize = response.nready as usize;
                        let mut ready: Vec<(RawFileDescriptor, PollEvents)> =
                            Vec::with_capacity(nready);
                        for i in 0..nready {
                            ready.push((
                                response.fds[i] as RawFileDescriptor,
                                PollEvents(response.revents[i] as c_short),
                            ));
                        }
                        break Ok(ready);
                    },
                    Err(error) => {
                        ::syslog::error!("poll(): {error:?} (fds={fds:?}, timeout={timeout:?})");
                        break Err(error);
                    },
                }
            },
            _ => {
                let reason: &'static str = "unexpected message header";
                ::syslog::error!("poll(): {reason} (fds={fds:?}, timeout={timeout:?})");
                break Err(Error::new(ErrorCode::InvalidMessage, reason));
            },
        }
    }
}
