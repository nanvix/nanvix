// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::sys_select::{
    fd_set,
    timeval,
    FD_SETSIZE,
};

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Encodes an optional [`fd_set`] at `offset` as a one-byte presence flag (`0` = absent,
/// `1` = present) followed by the set's fixed-width wire payload. A `None` value leaves the
/// zero-initialized flag and payload untouched.
fn encode_optional_fd_set(bytes: &mut [u8], offset: usize, value: Option<fd_set>) {
    if let Some(set) = value {
        bytes[offset] = 1;
        bytes[offset + 1..offset + 1 + fd_set::WIRE_SIZE].copy_from_slice(&set.to_bytes());
    }
}

/// Decodes an optional [`fd_set`] encoded by [`encode_optional_fd_set()`].
fn decode_optional_fd_set(bytes: &[u8], offset: usize) -> Result<Option<fd_set>, Error> {
    match bytes[offset] {
        0 => Ok(None),
        1 => {
            let mut payload: [u8; fd_set::WIRE_SIZE] = [0; fd_set::WIRE_SIZE];
            payload.copy_from_slice(&bytes[offset + 1..offset + 1 + fd_set::WIRE_SIZE]);
            Ok(Some(fd_set::from_bytes(payload)))
        },
        _ => Err(Error::new(
            ErrorCode::InvalidMessage,
            "invalid fd_set presence flag in select message",
        )),
    }
}

//==================================================================================================
// SelectRequest
//==================================================================================================

/// Request message for the `select()` system call.
///
/// The payload is serialized with an explicit, offset-based wire layout (rather than relying on
/// the in-memory representation of `Option<...>` fields) so that a 32-bit guest and a 64-bit host
/// interoperate byte-for-byte:
///
/// `nfds (1) | readfds (1 + fd_set::WIRE_SIZE) | writefds (1 + fd_set::WIRE_SIZE) |
///  errorfds (1 + fd_set::WIRE_SIZE) | timeout (1 + timeval::WIRE_SIZE)` followed by padding.
///
/// Each optional field is prefixed by a one-byte presence flag (`0` = absent, `1` = present).
#[derive(Debug)]
pub struct SelectRequest {
    /// Number of file descriptors in each set (must be <= FD_SETSIZE).
    pub nfds: u8,
    /// Read file descriptors of interest.
    pub readfds: Option<fd_set>,
    /// Write file descriptors of interest.
    pub writefds: Option<fd_set>,
    /// Error/exception file descriptors of interest.
    pub errorfds: Option<fd_set>,
    /// Timeout, encoded in `timeval`'s fixed-width wire format for cross-architecture IPC.
    pub timeout: Option<[u8; timeval::WIRE_SIZE]>,
}

// Ensure the fixed-width wire layout fits within the payload, so the manual offsets/slices in
// `from_bytes()`/`into_bytes()` cannot overflow `PAYLOAD_SIZE` and panic at runtime.
::static_assert::assert_eq!(SelectRequest::WIRE_SIZE <= SystemCallMessage::PAYLOAD_SIZE);

// Ensure that the maximum number of file descriptors can be encoded in a `u8`.
::static_assert::assert_eq!(FD_SETSIZE < u8::MAX as usize);

impl SelectRequest {
    /// Wire size of an optional `fd_set` field (presence flag + set payload).
    const OPTIONAL_FD_SET_WIRE_SIZE: usize = mem::size_of::<u8>() + fd_set::WIRE_SIZE;
    /// Wire size of the optional timeout field (presence flag + timeval payload).
    const OPTIONAL_TIMEOUT_WIRE_SIZE: usize = mem::size_of::<u8>() + timeval::WIRE_SIZE;

    /// Offset of the `nfds` field.
    const OFFSET_NFDS: usize = 0;
    /// Offset of the read file descriptor set.
    const OFFSET_READFDS: usize = Self::OFFSET_NFDS + mem::size_of::<u8>();
    /// Offset of the write file descriptor set.
    const OFFSET_WRITEFDS: usize = Self::OFFSET_READFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Offset of the error file descriptor set.
    const OFFSET_ERRORFDS: usize = Self::OFFSET_WRITEFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Offset of the timeout field.
    const OFFSET_TIMEOUT: usize = Self::OFFSET_ERRORFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Total number of meaningful wire bytes.
    const WIRE_SIZE: usize = Self::OFFSET_TIMEOUT + Self::OPTIONAL_TIMEOUT_WIRE_SIZE;

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
            timeout: timeout.as_ref().map(|tv| tv.to_bytes()),
        }
    }

    /// Deserializes a request from its fixed-width wire representation.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Result<Self, Error> {
        let nfds: u8 = bytes[Self::OFFSET_NFDS];
        // Reject messages whose number of file descriptors exceeds the wire contract bound.
        if nfds as usize > FD_SETSIZE {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "number of file descriptors exceeds maximum supported in select message",
            ));
        }
        let readfds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_READFDS)?;
        let writefds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_WRITEFDS)?;
        let errorfds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_ERRORFDS)?;
        let timeout: Option<[u8; timeval::WIRE_SIZE]> = match bytes[Self::OFFSET_TIMEOUT] {
            0 => None,
            1 => {
                let mut payload: [u8; timeval::WIRE_SIZE] = [0; timeval::WIRE_SIZE];
                payload.copy_from_slice(
                    &bytes[Self::OFFSET_TIMEOUT + 1..Self::OFFSET_TIMEOUT + 1 + timeval::WIRE_SIZE],
                );
                Some(payload)
            },
            _ => {
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "invalid timeout presence flag in select message",
                ));
            },
        };
        Ok(Self {
            nfds,
            readfds,
            writefds,
            errorfds,
            timeout,
        })
    }

    /// Serializes the request into its fixed-width wire representation.
    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        let mut bytes: [u8; SystemCallMessage::PAYLOAD_SIZE] = [0; SystemCallMessage::PAYLOAD_SIZE];
        bytes[Self::OFFSET_NFDS] = self.nfds;
        encode_optional_fd_set(&mut bytes, Self::OFFSET_READFDS, self.readfds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_WRITEFDS, self.writefds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_ERRORFDS, self.errorfds);
        if let Some(payload) = self.timeout {
            bytes[Self::OFFSET_TIMEOUT] = 1;
            bytes[Self::OFFSET_TIMEOUT + 1..Self::OFFSET_TIMEOUT + 1 + timeval::WIRE_SIZE]
                .copy_from_slice(&payload);
        }
        bytes
    }

    /// Builds a kernel IPC message for a `select()` system call request.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        tid: ThreadIdentifier,
        nfds: usize,
        readfds: &Option<&mut fd_set>,
        writefds: &Option<&mut fd_set>,
        errorfds: &Option<&mut fd_set>,
        timeout: &Option<timeval>,
        destination: ProcessIdentifier,
        message_type: MessageType,
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
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::SelectRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(destination),
            message_type,
            None,
            message.into_bytes(),
        );
        Ok(message)
    }
}

//==================================================================================================
// SelectResponse
//==================================================================================================

/// Response message for the `select()` system call.
///
/// Uses the same explicit, offset-based wire layout as [`SelectRequest`] for cross-architecture
/// IPC compatibility:
///
/// `ready_fds (1) | readfds (1 + fd_set::WIRE_SIZE) | writefds (1 + fd_set::WIRE_SIZE) |
///  errorfds (1 + fd_set::WIRE_SIZE)` followed by padding.
///
/// Each optional field is prefixed by a one-byte presence flag (`0` = absent, `1` = present).
#[derive(Debug)]
pub struct SelectResponse {
    /// Number of file descriptors ready.
    pub ready_fds: u8,
    /// Read file descriptors ready.
    pub readfds: Option<fd_set>,
    /// Write file descriptors ready.
    pub writefds: Option<fd_set>,
    /// Error/exception file descriptors ready.
    pub errorfds: Option<fd_set>,
}

// Ensure the fixed-width wire layout fits within the payload, so the manual offsets/slices in
// `from_bytes()`/`into_bytes()` cannot overflow `PAYLOAD_SIZE` and panic at runtime.
::static_assert::assert_eq!(SelectResponse::WIRE_SIZE <= SystemCallMessage::PAYLOAD_SIZE);

impl SelectResponse {
    /// Wire size of an optional `fd_set` field (presence flag + set payload).
    const OPTIONAL_FD_SET_WIRE_SIZE: usize = mem::size_of::<u8>() + fd_set::WIRE_SIZE;

    /// Offset of the `ready_fds` field.
    const OFFSET_READY_FDS: usize = 0;
    /// Offset of the read file descriptor set.
    const OFFSET_READFDS: usize = Self::OFFSET_READY_FDS + mem::size_of::<u8>();
    /// Offset of the write file descriptor set.
    const OFFSET_WRITEFDS: usize = Self::OFFSET_READFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Offset of the error file descriptor set.
    const OFFSET_ERRORFDS: usize = Self::OFFSET_WRITEFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Total number of meaningful wire bytes.
    const WIRE_SIZE: usize = Self::OFFSET_ERRORFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;

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
        }
    }

    /// Deserializes a response from its fixed-width wire representation.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Result<Self, Error> {
        let ready_fds: u8 = bytes[Self::OFFSET_READY_FDS];
        // Reject messages whose number of ready file descriptors exceeds the wire contract bound.
        if ready_fds as usize > FD_SETSIZE {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "number of ready file descriptors exceeds maximum supported in select message",
            ));
        }
        let readfds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_READFDS)?;
        let writefds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_WRITEFDS)?;
        let errorfds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_ERRORFDS)?;
        Ok(Self {
            ready_fds,
            readfds,
            writefds,
            errorfds,
        })
    }

    /// Serializes the response into its fixed-width wire representation.
    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        let mut bytes: [u8; SystemCallMessage::PAYLOAD_SIZE] = [0; SystemCallMessage::PAYLOAD_SIZE];
        bytes[Self::OFFSET_READY_FDS] = self.ready_fds;
        encode_optional_fd_set(&mut bytes, Self::OFFSET_READFDS, self.readfds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_WRITEFDS, self.writefds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_ERRORFDS, self.errorfds);
        bytes
    }

    /// Builds a kernel IPC message for a `select()` system call response.
    pub fn build(
        tid: ThreadIdentifier,
        ready_fds: u8,
        readfds: &Option<fd_set>,
        writefds: &Option<fd_set>,
        errorfds: &Option<fd_set>,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: SelectResponse = SelectResponse::new(ready_fds, readfds, writefds, errorfds);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::SelectResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(source),
            MessageReceiver::from(tid),
            message_type,
            None,
            message.into_bytes(),
        );
        message
    }
}
