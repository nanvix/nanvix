// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::ThreadIdentifier,
};
use sys::ipc::{
    Message,
    MessageReceiver,
    MessageSender,
    MessageType,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of file descriptors that can be polled in a single request.
pub const NFDS_MAX: usize = 4;

// Ensure that the maximum number of file descriptors can be encoded in a `PollRequest`.
::static_assert::assert_eq!(NFDS_MAX < u8::MAX as usize);

//==================================================================================================
// Structure
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PollRequest {
    /// Number of file descriptors in the `fds` array.
    pub nfds: u8,
    /// File descriptors to poll.
    pub fds: [i32; NFDS_MAX],
    /// Events to poll for on each file descriptor.
    pub events: [i16; NFDS_MAX],
    /// Timeout for the poll operation, in milliseconds.
    pub timeout: i32,
    /// Padding to align the structure to the size of `LinuxDaemonMessage`.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PollRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl PollRequest {
    /// Padding size to align the structure.
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<[i32; NFDS_MAX]>() // fds
        - mem::size_of::<[i16; NFDS_MAX]>() // events
        - mem::size_of::<i32>() // timeout
        - mem::size_of::<u8>(); // nfds
}

impl PollRequest {
    /// Creates a new `PollRequest`.
    fn new(nfds: i8, fds: &[i32], events: &[i16], timeout: i32) -> Self {
        debug_assert!(nfds > 0 && nfds as usize <= NFDS_MAX, "nfds must be > 0 && <= {NFDS_MAX}");
        debug_assert!(
            !fds.is_empty() && fds.len() <= NFDS_MAX,
            "fds.len() must be > 0 && <= {NFDS_MAX}"
        );
        debug_assert!(fds.len() == events.len(), "fds and events must have the same length");

        // Pack file descriptors.
        let mut poll_fds: [i32; NFDS_MAX] = [0; NFDS_MAX];
        for (i, &fd) in fds.iter().enumerate() {
            if i < NFDS_MAX {
                poll_fds[i] = fd;
            }
        }

        // Pack events.
        let mut poll_events: [i16; NFDS_MAX] = [0; NFDS_MAX];
        for (i, &event) in events.iter().enumerate() {
            if i < NFDS_MAX {
                poll_events[i] = event;
            }
        }

        Self {
            nfds: nfds as u8,
            fds: poll_fds,
            events: poll_events,
            timeout,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Creates a `PollRequest` from a byte array.
    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts the request into a byte array.
    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a `Message` from a `PollRequest`.
    pub fn build(
        tid: ThreadIdentifier,
        fds: &[i32],
        events: &[i16],
        timeout: i32,
    ) -> Result<Message, Error> {
        // Check if number of file descriptors exceeds the maximum supported.
        if fds.len() > NFDS_MAX {
            let reason: &str = "number of file descriptors exceeds maximum supported";
            #[cfg(not(target_os = "linux"))]
            ::syslog::error!(
                "build(): {reason:?}, (max={NFDS_MAX}, fds.len()={}, events.len()={}, \
                 timeout={timeout:?})",
                fds.len(),
                events.len(),
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let nfds: i8 = match fds.len().try_into() {
            Ok(n) => n,
            Err(_) => {
                let reason: &str = "number of file descriptors exceeds maximum supported";
                #[cfg(not(target_os = "linux"))]
                ::syslog::error!(
                    "build(): {reason:?}, (max={NFDS_MAX}, fds.len()={}, events.len()={}, \
                     timeout={timeout:?})",
                    fds.len(),
                    events.len(),
                );
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        // Check if number of events does not match the number of file descriptors.
        if events.len() != fds.len() {
            let reason: &str = "number of events does not match number of file descriptors";
            #[cfg(not(target_os = "linux"))]
            ::syslog::error!(
                "build(): {reason:?}, (fds.len()={}, events.len()={}, timeout={timeout:?})",
                fds.len(),
                events.len(),
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let message: PollRequest = Self::new(nfds, fds, events, timeout);

        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::PollRequest, message.into_bytes());

        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::LINUXD),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        Ok(message)
    }
}

//==================================================================================================
// PollResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PollResponse {
    /// Number of file descriptors with ready events.
    pub nready: u8,
    /// File descriptors to poll.
    pub fds: [i32; NFDS_MAX],
    /// Events that occurred on each file descriptor.
    pub revents: [i16; NFDS_MAX],
    /// Padding to align the structure to the size of `LinuxDaemonMessage`.
    _padding: [u8; Self::PADDING_SIZE],
}

impl PollResponse {
    /// Padding size to align the structure.
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
    - mem::size_of::<u8>() // nready
    - mem::size_of::<[i32; NFDS_MAX]>() // fds
    - mem::size_of::<[i16; NFDS_MAX]>(); // revents

    /// Creates a new `PollResponse`.
    fn new(nready: u8, fds: &[i32], revents: &[i16]) -> Self {
        debug_assert!(fds.len() == revents.len(), "fds and revents must have the same length");

        // Pack file descriptors.
        let mut ready_fds: [i32; NFDS_MAX] = [0; NFDS_MAX];
        for (i, &fd) in fds.iter().enumerate() {
            if i < NFDS_MAX {
                ready_fds[i] = fd;
            }
        }

        // Pack revents.
        let mut ready_events: [i16; NFDS_MAX] = [0; NFDS_MAX];
        for (i, &revent) in revents.iter().enumerate() {
            if i < NFDS_MAX {
                ready_events[i] = revent;
            }
        }

        Self {
            nready,
            fds: ready_fds,
            revents: ready_events,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Creates a `PollResponse` from a byte array.
    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts the response into a byte array.
    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a `Message` from a `PollResponse`.
    pub fn build(
        tid: ThreadIdentifier,
        nready: u8,
        fds: &[i32],
        revents: &[i16],
    ) -> Result<Message, Error> {
        // Check if number of file descriptors exceeds the maximum supported.
        if fds.len() > NFDS_MAX {
            let reason: &str = "number of file descriptors exceeds maximum supported";
            #[cfg(not(target_os = "linux"))]
            ::syslog::error!(
                "build(): {reason:?}, (max={NFDS_MAX}, fds.len()={}, revents.len()={})",
                fds.len(),
                revents.len(),
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if number of events does not match the number of file descriptors.
        if revents.len() != fds.len() {
            let reason: &str = "number of events does not match number of file descriptors";
            #[cfg(not(target_os = "linux"))]
            ::syslog::error!(
                "build(): {reason:?}, (fds.len()={}, revents.len()={})",
                fds.len(),
                revents.len(),
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let message: PollResponse = Self::new(nready, fds, revents);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::PollResponse, message.into_bytes());

        let message: Message = Message::new(
            MessageSender::from(crate::LINUXD),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        Ok(message)
    }
}
