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
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::sys_select::{
    fd_set,
    timeval,
    FD_SETSIZE,
};

//==================================================================================================
// SelectRequest
//==================================================================================================

/// Request message for the `select()` system call.
#[repr(C, packed)]
pub struct SelectRequest {
    /// Number of file descriptors in each set (must be <= FD_SETSIZE).
    pub nfds: u8,
    /// Read file descriptors of interest.
    pub readfds: Option<fd_set>,
    /// Write file descriptors of interest.
    pub writefds: Option<fd_set>,
    /// Error/exception file descriptors of interest.
    pub errorfds: Option<fd_set>,
    /// Timeout.
    pub timeout: Option<timeval>,
    /// Required padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SelectRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

// Ensure that the maximum number of file descriptors can be encoded in a `u8`.
::static_assert::assert_eq!(FD_SETSIZE < u8::MAX as usize);

impl SelectRequest {
    /// Size of the padding field.
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<u8>()
        - mem::size_of::<Option<fd_set>>()
        - mem::size_of::<Option<fd_set>>()
        - mem::size_of::<Option<fd_set>>()
        - mem::size_of::<Option<timeval>>();

    /// Creates a new `SelectRequest`.
    fn new(
        nfds: u8,
        readfds: &Option<&mut fd_set>,
        writefds: &Option<&mut fd_set>,
        errorfds: &Option<&mut fd_set>,
        timeout: &Option<timeval>,
    ) -> Self {
        Self {
            nfds,
            readfds: readfds.as_ref().map(|fd_set| **fd_set),
            writefds: writefds.as_ref().map(|fd_set| **fd_set),
            errorfds: errorfds.as_ref().map(|fd_set| **fd_set),
            timeout: *timeout,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a request from raw bytes.
    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Serializes the request into raw bytes.
    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a kernel IPC message for a `select()` system call request.
    pub fn build(
        tid: ThreadIdentifier,
        nfds: usize,
        readfds: &Option<&mut fd_set>,
        writefds: &Option<&mut fd_set>,
        errorfds: &Option<&mut fd_set>,
        timeout: &Option<timeval>,
    ) -> Result<Message, Error> {
        // Validate number of file descriptors.
        if nfds > FD_SETSIZE {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "number of file descriptors exceeds maximum supported",
            ));
        }

        // Attempt to encode nfds as u8 (should always succeed due to static assert, but be safe).
        let nfds_u8: u8 = match nfds.try_into() {
            Ok(v) => v,
            Err(_e) => {
                return Err(Error::new(
                    ErrorCode::ValueOutOfRange,
                    "cannot encode number of file descriptors",
                ));
            },
        };

        let message: SelectRequest =
            SelectRequest::new(nfds_u8, readfds, writefds, errorfds, timeout);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::SelectRequest, message.into_bytes());
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

impl core::fmt::Debug for SelectRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SelectRequest")
            .field("nfds", &self.nfds)
            .field("readfds", &self.readfds)
            .field("writefds", &self.writefds)
            .field("errorfds", &self.errorfds)
            .field("timeout", &self.timeout)
            .finish()
    }
}

//==================================================================================================
// SelectResponse
//==================================================================================================

/// Response message for the `select()` system call.
#[repr(C, packed)]
pub struct SelectResponse {
    /// Number of file descriptors ready.
    pub ready_fds: u8,
    /// Read file descriptors ready.
    pub readfds: Option<fd_set>,
    /// Write file descriptors ready.
    pub writefds: Option<fd_set>,
    /// Error/exception file descriptors ready.
    pub errorfds: Option<fd_set>,
    /// Required padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SelectResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl SelectResponse {
    /// Size of the padding field.
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<u8>()
        - mem::size_of::<Option<fd_set>>()
        - mem::size_of::<Option<fd_set>>()
        - mem::size_of::<Option<fd_set>>();

    /// Creates a new `SelectResponse`.
    fn new(
        ready_fds: u8,
        readfds: &Option<fd_set>,
        writefds: &Option<fd_set>,
        errorfds: &Option<fd_set>,
    ) -> Self {
        Self {
            ready_fds,
            readfds: *readfds,
            writefds: *writefds,
            errorfds: *errorfds,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a response from raw bytes.
    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Serializes the response into raw bytes.
    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a kernel IPC message for a `select()` system call response.
    pub fn build(
        tid: ThreadIdentifier,
        ready_fds: u8,
        readfds: &Option<fd_set>,
        writefds: &Option<fd_set>,
        errorfds: &Option<fd_set>,
    ) -> Message {
        let message: SelectResponse = SelectResponse::new(ready_fds, readfds, writefds, errorfds);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::SelectResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(crate::LINUXD),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );
        message
    }
}

impl core::fmt::Debug for SelectResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SelectResponse")
            .field("ready_fds", &self.ready_fds)
            .field("readfds", &self.readfds)
            .field("writefds", &self.writefds)
            .field("errorfds", &self.errorfds)
            .finish()
    }
}
